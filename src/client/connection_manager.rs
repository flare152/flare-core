//! 客户端连接管理器
//!
//! 负责管理客户端连接，包括连接创建、重连、心跳等功能

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock, oneshot};
use tokio::time::interval;
use tracing::{info, error, debug, warn};
use uuid;

use crate::common::{
    conn::Connection,
    Result, FlareError, TransportProtocol,
};
use crate::server::conn_manager::memory::ConnectionEvent;
use crate::client::types::{HealthCheckConfig, HealthStatus, HealthCheckResult};
use crate::client::callbacks::ClientCallbackManager;

/// 连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// 响应等待器类型
pub type ResponseWaiter = oneshot::Sender<crate::common::protocol::UnifiedProtocolMessage>;

/// 连接管理器
pub struct ConnectionManager {
    config: crate::client::config::ClientConfig,
    state: Arc<RwLock<ConnectionState>>,
    current_connection: Arc<Mutex<Option<Box<dyn Connection + Send + Sync>>>>,
    last_connection_time: Arc<RwLock<Option<Instant>>>,
    reconnect_count: Arc<RwLock<u32>>,
    event_callback: Arc<Mutex<Option<Box<dyn Fn(ConnectionEvent) + Send + Sync>>>>,
    /// 待处理的响应等待器（request_id -> waiter）
    pending_responses: Arc<RwLock<HashMap<String, ResponseWaiter>>>,
    /// 心跳任务句柄
    heartbeat_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 是否启用心跳
    heartbeat_enabled: Arc<RwLock<bool>>,
    /// 健康检查配置
    health_check_config: HealthCheckConfig,
    /// 健康检查任务句柄
    health_check_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 心跳失败计数
    heartbeat_failures: Arc<RwLock<u32>>,
    /// 最后心跳时间
    last_heartbeat_time: Arc<RwLock<Option<Instant>>>,
    /// 最后活动时间
    last_activity_time: Arc<RwLock<Instant>>,
    /// 回调管理器
    callback_manager: Arc<RwLock<ClientCallbackManager>>,
    /// 会话ID
    session_id: Arc<RwLock<String>>,
    /// 连接统计
    connection_stats: Arc<RwLock<crate::client::types::ConnectionStats>>,
    /// 消息解析器
    message_parser: Arc<crate::common::MessageParser>,
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub fn new(config: crate::client::config::ClientConfig, message_parser: Arc<crate::common::MessageParser>) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            current_connection: Arc::new(Mutex::new(None)),
            last_connection_time: Arc::new(RwLock::new(None)),
            reconnect_count: Arc::new(RwLock::new(0)),
            event_callback: Arc::new(Mutex::new(None)),
            pending_responses: Arc::new(RwLock::new(HashMap::new())),
            heartbeat_task: Arc::new(Mutex::new(None)),
            heartbeat_enabled: Arc::new(RwLock::new(true)),
            health_check_config: HealthCheckConfig::default(),
            health_check_task: Arc::new(Mutex::new(None)),
            heartbeat_failures: Arc::new(RwLock::new(0)),
            last_heartbeat_time: Arc::new(RwLock::new(None)),
            last_activity_time: Arc::new(RwLock::new(Instant::now())),
            callback_manager: Arc::new(RwLock::new(ClientCallbackManager::new())),
            session_id: Arc::new(RwLock::new(uuid::Uuid::new_v4().to_string())),
            connection_stats: Arc::new(RwLock::new(crate::client::types::ConnectionStats::default())),
            message_parser,
        }
    }

    /// 设置事件回调
    pub async fn set_event_callback(&mut self, callback: Box<dyn Fn(ConnectionEvent) + Send + Sync>) {
        let mut callback_guard = self.event_callback.lock().await;
        *callback_guard = Some(callback);
    }

    /// 设置回调管理器
    pub async fn set_callback_manager(&self, callback_manager: ClientCallbackManager) {
        let mut manager = self.callback_manager.write().await;
        *manager = callback_manager;
    }

    /// 设置健康检查配置
    pub async fn set_health_check_config(&mut self, config: HealthCheckConfig) {
        self.health_check_config = config;
    }

    /// 触发事件
    async fn trigger_event(&self, event: ConnectionEvent) {
        if let Some(callback) = &*self.event_callback.lock().await {
            callback(event);
        }
    }

    /// 更新最后活动时间
    async fn update_last_activity(&self) {
        let mut last_activity = self.last_activity_time.write().await;
        *last_activity = Instant::now();
    }

    /// 更新连接统计
    async fn update_connection_stats(&self, is_sent: bool, bytes: usize) {
        let mut stats = self.connection_stats.write().await;
        if is_sent {
            stats.messages_sent += 1;
            stats.bytes_sent += bytes as u64;
        } else {
            stats.messages_received += 1;
            stats.bytes_received += bytes as u64;
        }
        stats.last_activity = chrono::Utc::now();
    }

    /// 连接到服务器
    pub async fn connect(&mut self) -> Result<()> {
        info!("开始连接到服务器");
        
        // 更新状态为连接中
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Connecting;
        }

        match self.config.protocol_selection_mode {
            crate::client::config::ProtocolSelectionMode::AutoRacing => {
                self.connect_with_racing().await
            }
            crate::client::config::ProtocolSelectionMode::Specific(protocol) => {
                self.connect_with_protocol(protocol).await
            }
            crate::client::config::ProtocolSelectionMode::Manual => {
                Err(FlareError::ConnectionFailed("手动模式需要用户选择协议".to_string()))
            }
        }
    }

    /// 使用协议竞速模式连接
    async fn connect_with_racing(&mut self) -> Result<()> {
        info!("使用协议竞速模式连接");
        
        let mut racer = crate::client::protocol_racer::ProtocolRacer::new();
        let best_protocol = racer.race_protocols(&self.config).await?;
        
        info!("协议竞速完成，选择协议: {:?}", best_protocol);
        self.connect_with_protocol(best_protocol).await
    }

    /// 使用指定协议连接
    async fn connect_with_protocol(&mut self, protocol: TransportProtocol) -> Result<()> {
        info!("使用协议连接: {:?}", protocol);
        
        let connection = match protocol {
            TransportProtocol::WebSocket => {
                self.create_websocket_connection().await?
            }
            TransportProtocol::QUIC => {
                self.create_quic_connection().await?
            }
            TransportProtocol::Auto => {
                // 自动选择：优先QUIC，回退WebSocket
                match self.create_quic_connection().await {
                    Ok(conn) => conn,
                    Err(_) => self.create_websocket_connection().await?,
                }
            }
        };

        // 设置连接
        {
            let mut conn_guard = self.current_connection.lock().await;
            *conn_guard = Some(connection);
        }

        // 更新状态和时间
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Connected;
        }
        {
            let mut time = self.last_connection_time.write().await;
            *time = Some(Instant::now());
        }

        // 重置健康检查状态
        {
            let mut failures = self.heartbeat_failures.write().await;
            *failures = 0;
        }
        {
            let mut last_heartbeat = self.last_heartbeat_time.write().await;
            *last_heartbeat = Some(Instant::now());
        }

        // 启动心跳任务
        self.start_heartbeat_task().await;

        // 启动健康检查任务
        self.start_health_check_task().await;

        // 触发连接成功事件
        self.trigger_event(ConnectionEvent::Connected).await;

        info!("连接成功建立");
        Ok(())
    }

    /// 创建 WebSocket 连接
    async fn create_websocket_connection(&self) -> Result<Box<dyn Connection + Send + Sync>> {
        let connector = crate::client::websocket_connector::WebSocketConnector::new(
            self.config.clone(),
            Arc::clone(&self.message_parser),
        );
        connector.create_connection("ws://localhost:4000", Duration::from_secs(30)).await
    }

    /// 创建 QUIC 连接
    async fn create_quic_connection(&self) -> Result<Box<dyn Connection + Send + Sync>> {
        let connector = crate::client::quic_connector::QuicConnector::new(
            self.config.clone(),
            Arc::clone(&self.message_parser),
        );
        connector.create_connection("quic://localhost:4010", Duration::from_secs(30)).await
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        info!("断开连接");
        
        // 停止心跳任务
        self.stop_heartbeat_task().await;
        
        // 停止健康检查任务
        self.stop_health_check_task().await;
        
        // 清理待处理的响应
        self.clear_pending_responses().await;
        
        // 关闭当前连接
        if let Some(connection) = self.current_connection.lock().await.take() {
            if let Err(e) = connection.close().await {
                error!("关闭连接时出错: {}", e);
            }
        }

        // 更新状态
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Disconnected;
        }

        // 触发断开连接事件
        self.trigger_event(ConnectionEvent::Disconnected).await;

        info!("连接已断开");
        Ok(())
    }

    /// 重新连接
    pub async fn reconnect(&mut self) -> Result<()> {
        info!("开始重新连接");
        
        // 检查重连次数限制
        {
            let reconnect_count = *self.reconnect_count.read().await;
            if reconnect_count >= self.health_check_config.max_reconnect_attempts {
                return Err(FlareError::ConnectionFailed("达到最大重连次数".to_string()));
            }
        }

        // 更新状态
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Reconnecting;
        }

        // 增加重连计数
        {
            let mut count = self.reconnect_count.write().await;
            *count += 1;
        }

        // 触发重连事件
        self.trigger_event(ConnectionEvent::Reconnecting).await;

        // 等待重连延迟
        tokio::time::sleep(Duration::from_millis(self.health_check_config.reconnect_delay_ms)).await;

        // 尝试重新连接
        let result = self.connect().await;

        if result.is_ok() {
            // 重置重连计数
            {
                let mut count = self.reconnect_count.write().await;
                *count = 0;
            }
        }

        result
    }

    /// 发送消息
    pub async fn send_message(&self, message: crate::common::protocol::UnifiedProtocolMessage) -> Result<()> {
        let connection_guard = self.current_connection.lock().await;
        if let Some(ref conn) = *connection_guard {
            let result = conn.send(message.clone()).await;
            if result.is_ok() {
                // 更新统计
                self.update_connection_stats(true, message.c.len()).await;
                self.update_last_activity().await;
            }
            result
        } else {
            Err(FlareError::ConnectionFailed("未连接".to_string()))
        }
    }

    /// 检查是否已连接
    pub async fn is_connected(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, ConnectionState::Connected)
    }

    /// 获取连接状态
    pub async fn get_state(&self) -> ConnectionState {
        let state = self.state.read().await;
        state.clone()
    }

    /// 获取重连次数
    pub async fn get_reconnect_count(&self) -> u32 {
        let count = self.reconnect_count.read().await;
        *count
    }

    /// 获取最后连接时间
    pub async fn get_last_connection_time(&self) -> Option<Instant> {
        let time = self.last_connection_time.read().await;
        *time
    }

    /// 发送消息并等待响应
    /// 
    /// # 参数
    /// * `message` - 要发送的消息
    /// * `timeout` - 超时时间
    /// 
    /// # 返回
    /// * `Result<UnifiedProtocolMessage>` - 响应消息或超时错误
    pub async fn send_and_wait_response(
        &self, 
        mut message: crate::common::protocol::UnifiedProtocolMessage, 
        timeout: Duration
    ) -> Result<crate::common::protocol::UnifiedProtocolMessage> {
        // 检查连接状态
        if !self.is_connected().await {
            return Err(FlareError::ConnectionFailed("未连接".to_string()));
        }

        // 生成请求ID并设置到消息中
        let request_id = uuid::Uuid::new_v4().to_string();
        message = message.with_request_id(request_id.clone());
        
        // 创建响应通道
        let (tx, rx) = oneshot::channel();
        
        // 注册响应等待器
        {
            let mut pending = self.pending_responses.write().await;
            pending.insert(request_id.clone(), tx);
        }
        
        // 发送消息
        let send_result = self.send_message(message).await;
        if let Err(e) = send_result {
            // 移除等待器
            let mut pending = self.pending_responses.write().await;
            pending.remove(&request_id);
            return Err(e);
        }
        
        // 等待响应或超时
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                // 清理等待器
                let mut pending = self.pending_responses.write().await;
                pending.remove(&request_id);
                Err(FlareError::NetworkError("响应通道已关闭".to_string()))
            }
            Err(_) => {
                // 清理等待器
                let mut pending = self.pending_responses.write().await;
                pending.remove(&request_id);
                Err(FlareError::connection_timeout("等待响应超时".to_string()))
            }
        }
    }

    /// 处理接收到的响应消息
    /// 
    /// 此方法需要在消息接收处调用，用于匹配等待的响应
    pub async fn handle_response(&self, response: &crate::common::protocol::UnifiedProtocolMessage) -> bool {
        // 检查是否有请求ID
        if let Some(request_id) = response.get_request_id() {
            let mut pending = self.pending_responses.write().await;
            if let Some(waiter) = pending.remove(request_id) {
                let _ = waiter.send(response.clone());
                debug!("处理响应消息: {}", request_id);
                return true;
            }
        }
        false
    }

    /// 发送心跳消息
    pub async fn send_heartbeat(&self) -> Result<()> {
        let heartbeat_msg = crate::common::protocol::UnifiedProtocolMessage::heartbeat();
        debug!("发送心跳消息");
        let result = self.send_message(heartbeat_msg).await;
        
        if result.is_ok() {
            // 更新心跳时间
            let mut last_heartbeat = self.last_heartbeat_time.write().await;
            *last_heartbeat = Some(Instant::now());
            
            // 更新统计
            let mut stats = self.connection_stats.write().await;
            stats.heartbeat_count += 1;
        }
        
        result
    }

    /// 启用/禁用心跳
    pub async fn set_heartbeat_enabled(&self, enabled: bool) {
        let mut heartbeat_enabled = self.heartbeat_enabled.write().await;
        *heartbeat_enabled = enabled;
        
        if enabled {
            // 如果连接状态是已连接，启动心跳任务
            if self.is_connected().await {
                drop(heartbeat_enabled); // 释放锁
                self.start_heartbeat_task().await;
            }
        } else {
            drop(heartbeat_enabled); // 释放锁
            self.stop_heartbeat_task().await;
        }
    }

    /// 启动心跳任务
    async fn start_heartbeat_task(&self) {
        // 检查心跳是否启用
        let heartbeat_enabled = *self.heartbeat_enabled.read().await;
        if !heartbeat_enabled {
            return;
        }

        // 停止现有的心跳任务
        self.stop_heartbeat_task().await;
        
        let connection_manager = self.clone_for_heartbeat();
        let heartbeat_interval = Duration::from_millis(self.health_check_config.heartbeat_interval_ms);
        
        let task = tokio::spawn(async move {
            let mut interval = interval(heartbeat_interval);
            loop {
                interval.tick().await;
                
                // 检查连接状态
                if !connection_manager.is_connected().await {
                    debug!("连接已断开，停止心跳任务");
                    break;
                }
                
                // 检查心跳是否仍然启用
                let enabled = *connection_manager.heartbeat_enabled.read().await;
                if !enabled {
                    debug!("心跳已禁用，停止心跳任务");
                    break;
                }
                
                // 发送心跳
                if let Err(e) = connection_manager.send_heartbeat().await {
                    error!("发送心跳失败: {}", e);
                    // 增加心跳失败计数
                    let mut failures = connection_manager.heartbeat_failures.write().await;
                    *failures += 1;
                }
            }
        });
        
        let mut heartbeat_task_guard = self.heartbeat_task.lock().await;
        *heartbeat_task_guard = Some(task);
        debug!("心跳任务已启动");
    }

    /// 停止心跳任务
    async fn stop_heartbeat_task(&self) {
        let mut heartbeat_task_guard = self.heartbeat_task.lock().await;
        if let Some(task) = heartbeat_task_guard.take() {
            task.abort();
            debug!("心跳任务已停止");
        }
    }

    /// 启动健康检查任务
    async fn start_health_check_task(&self) {
        // 停止现有的健康检查任务
        self.stop_health_check_task().await;
        
        let connection_manager = self.clone_for_health_check();
        let check_interval = Duration::from_millis(self.health_check_config.activity_check_interval_ms);
        
        let task = tokio::spawn(async move {
            let mut interval = interval(check_interval);
            loop {
                interval.tick().await;
                
                // 检查连接状态
                if !connection_manager.is_connected().await {
                    debug!("连接已断开，停止健康检查任务");
                    break;
                }
                
                // 执行健康检查
                let health_result = connection_manager.perform_health_check().await;
                
                // 根据健康检查结果决定是否重连
                match health_result.status {
                    HealthStatus::Dead => {
                        warn!("连接已死亡，尝试重连");
                        if connection_manager.health_check_config.auto_reconnect_enabled {
                            if let Err(e) = connection_manager.reconnect().await {
                                error!("自动重连失败: {}", e);
                            }
                        }
                        break;
                    }
                    HealthStatus::Unhealthy => {
                        warn!("连接不健康，尝试重连");
                        if connection_manager.health_check_config.auto_reconnect_enabled {
                            if let Err(e) = connection_manager.reconnect().await {
                                error!("自动重连失败: {}", e);
                            }
                        }
                    }
                    HealthStatus::Warning => {
                        debug!("连接状态警告，继续监控");
                    }
                    HealthStatus::Healthy => {
                        debug!("连接状态健康");
                    }
                }
            }
        });
        
        let mut health_check_task_guard = self.health_check_task.lock().await;
        *health_check_task_guard = Some(task);
        debug!("健康检查任务已启动");
    }

    /// 停止健康检查任务
    async fn stop_health_check_task(&self) {
        let mut health_check_task_guard = self.health_check_task.lock().await;
        if let Some(task) = health_check_task_guard.take() {
            task.abort();
            debug!("健康检查任务已停止");
        }
    }

    /// 执行健康检查
    pub async fn perform_health_check(&self) -> HealthCheckResult {
        let last_activity = *self.last_activity_time.read().await;
        let heartbeat_failures = *self.heartbeat_failures.read().await;
        let last_heartbeat = *self.last_heartbeat_time.read().await;
        
        let now = Instant::now();
        let activity_duration = now.duration_since(last_activity);
        let heartbeat_latency = if let Some(last_heartbeat_time) = last_heartbeat {
            now.duration_since(last_heartbeat_time).as_millis() as u64
        } else {
            0
        };
        
        // 计算连接质量
        let mut connection_quality = 1.0;
        if heartbeat_failures > 0 {
            connection_quality -= (heartbeat_failures as f64 * 0.2).min(0.8);
        }
        if activity_duration > Duration::from_millis(self.health_check_config.activity_timeout_ms) {
            connection_quality -= 0.5;
        }
        connection_quality = connection_quality.max(0.0);
        
        // 确定健康状态
        let status = if !self.is_connected().await {
            HealthStatus::Dead
        } else if heartbeat_failures >= self.health_check_config.max_heartbeat_failures {
            HealthStatus::Unhealthy
        } else if activity_duration > Duration::from_millis(self.health_check_config.activity_timeout_ms) {
            HealthStatus::Unhealthy
        } else if heartbeat_failures > 0 || heartbeat_latency > self.health_check_config.heartbeat_timeout_ms {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };
        
        // 更新连接统计
        {
            let mut stats = self.connection_stats.write().await;
            stats.heartbeat_latency_ms = heartbeat_latency;
            stats.connection_quality = connection_quality;
        }
        
        HealthCheckResult {
            status,
            heartbeat_latency_ms: heartbeat_latency,
            last_activity: chrono::Utc::now(),
            heartbeat_failures,
            connection_quality,
        }
    }

    /// 获取健康检查结果
    pub async fn get_health_status(&self) -> HealthCheckResult {
        self.perform_health_check().await
    }

    /// 获取连接统计
    pub async fn get_connection_stats(&self) -> crate::client::types::ConnectionStats {
        let stats = self.connection_stats.read().await;
        stats.clone()
    }

    /// 处理接收到的消息
    pub async fn handle_received_message(&self, message: crate::common::protocol::UnifiedProtocolMessage) -> Result<()> {
        // 更新最后活动时间
        self.update_last_activity().await;
        
        // 更新统计
        self.update_connection_stats(false, message.c.len()).await;
        
        // 处理心跳确认
        if message.is_heartbeat_ack() {
            // 重置心跳失败计数
            let mut failures = self.heartbeat_failures.write().await;
            *failures = 0;
            debug!("收到心跳确认，重置失败计数");
        }
        
        // 通过回调管理器处理消息
        let session_id = self.session_id.read().await.clone();
        let callback_manager = self.callback_manager.read().await;
        if let Err(e) = callback_manager.handle_protocol_message(&message, session_id).await {
            error!("处理消息失败: {}", e);
        }
        
        Ok(())
    }

    /// 清理待处理的响应
    async fn clear_pending_responses(&self) {
        let mut pending = self.pending_responses.write().await;
        for (request_id, waiter) in pending.drain() {
            debug!("清理待处理响应: {}", request_id);
            let _ = waiter.send(crate::common::protocol::UnifiedProtocolMessage::error(
                "CONNECTION_CLOSED".to_string(), 
                "连接已关闭".to_string()
            ));
        }
    }

    /// 创建用于心跳任务的克隆
    /// 
    /// 由于心跳任务需要独立运行，我们需要一个不可变的引用
    fn clone_for_heartbeat(&self) -> ConnectionManagerHeartbeat {
        ConnectionManagerHeartbeat {
            current_connection: Arc::clone(&self.current_connection),
            state: Arc::clone(&self.state),
            heartbeat_enabled: Arc::clone(&self.heartbeat_enabled),
            heartbeat_failures: Arc::clone(&self.heartbeat_failures),
            last_heartbeat_time: Arc::clone(&self.last_heartbeat_time),
        }
    }

    /// 创建用于健康检查任务的克隆
    fn clone_for_health_check(&self) -> ConnectionManagerHealthCheck {
        ConnectionManagerHealthCheck {
            state: Arc::clone(&self.state),
            last_activity_time: Arc::clone(&self.last_activity_time),
            heartbeat_failures: Arc::clone(&self.heartbeat_failures),
            last_heartbeat_time: Arc::clone(&self.last_heartbeat_time),
            health_check_config: self.health_check_config.clone(),
        }
    }

    /// 获取待处理响应数量（用于调试）
    pub async fn get_pending_response_count(&self) -> usize {
        let pending = self.pending_responses.read().await;
        pending.len()
    }
}

/// 心跳任务专用的连接管理器引用
/// 
/// 包含心跳任务所需的最小字段集合
struct ConnectionManagerHeartbeat {
    current_connection: Arc<Mutex<Option<Box<dyn Connection + Send + Sync>>>>,
    state: Arc<RwLock<ConnectionState>>,
    heartbeat_enabled: Arc<RwLock<bool>>,
    heartbeat_failures: Arc<RwLock<u32>>,
    last_heartbeat_time: Arc<RwLock<Option<Instant>>>,
}

impl ConnectionManagerHeartbeat {
    async fn is_connected(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, ConnectionState::Connected)
    }
    
    async fn send_heartbeat(&self) -> Result<()> {
        let connection_guard = self.current_connection.lock().await;
        if let Some(ref conn) = *connection_guard {
            let heartbeat_msg = crate::common::protocol::UnifiedProtocolMessage::heartbeat();
            let result = conn.send(heartbeat_msg).await;
            
            if result.is_ok() {
                // 更新心跳时间
                let mut last_heartbeat = self.last_heartbeat_time.write().await;
                *last_heartbeat = Some(Instant::now());
            }
            
            result
        } else {
            Err(FlareError::ConnectionFailed("未连接".to_string()))
        }
    }
}

/// 健康检查任务专用的连接管理器引用
struct ConnectionManagerHealthCheck {
    state: Arc<RwLock<ConnectionState>>,
    last_activity_time: Arc<RwLock<Instant>>,
    heartbeat_failures: Arc<RwLock<u32>>,
    last_heartbeat_time: Arc<RwLock<Option<Instant>>>,
    health_check_config: HealthCheckConfig,
}

impl ConnectionManagerHealthCheck {
    async fn is_connected(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, ConnectionState::Connected)
    }
    
    async fn perform_health_check(&self) -> HealthCheckResult {
        let last_activity = *self.last_activity_time.read().await;
        let heartbeat_failures = *self.heartbeat_failures.read().await;
        let last_heartbeat = *self.last_heartbeat_time.read().await;
        
        let now = Instant::now();
        let activity_duration = now.duration_since(last_activity);
        let heartbeat_latency = if let Some(last_heartbeat_time) = last_heartbeat {
            now.duration_since(last_heartbeat_time).as_millis() as u64
        } else {
            0
        };
        
        // 计算连接质量
        let mut connection_quality = 1.0;
        if heartbeat_failures > 0 {
            connection_quality -= (heartbeat_failures as f64 * 0.2).min(0.8);
        }
        if activity_duration > Duration::from_millis(self.health_check_config.activity_timeout_ms) {
            connection_quality -= 0.5;
        }
        connection_quality = connection_quality.max(0.0);
        
        // 确定健康状态
        let status = if !self.is_connected().await {
            HealthStatus::Dead
        } else if heartbeat_failures >= self.health_check_config.max_heartbeat_failures {
            HealthStatus::Unhealthy
        } else if activity_duration > Duration::from_millis(self.health_check_config.activity_timeout_ms) {
            HealthStatus::Unhealthy
        } else if heartbeat_failures > 0 || heartbeat_latency > self.health_check_config.heartbeat_timeout_ms {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };
        
        HealthCheckResult {
            status,
            heartbeat_latency_ms: heartbeat_latency,
            last_activity: chrono::Utc::now(),
            heartbeat_failures,
            connection_quality,
        }
    }
    
    async fn reconnect(&self) -> Result<()> {
        // 这里需要实现重连逻辑
        // 由于这是一个简化的引用，我们返回错误
        Err(FlareError::ConnectionFailed("重连功能需要完整的连接管理器".to_string()))
    }
} 