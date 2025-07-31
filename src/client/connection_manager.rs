//! Flare IM 客户端连接管理器
//!
//! 提供与服务端配合的连接管理、心跳处理、自动重连等功能
//! 确保连接在有网络的情况下可用

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex, mpsc};
use tokio::time::{interval, sleep, timeout};
use async_trait::async_trait;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::common::{
    Result, FlareError, TransportProtocol, ConnectionInfo
};
use crate::client::traits::{ClientConnectionManager, ReconnectionManager, NetworkMonitor};
use crate::client::traits::{ReconnectionStrategy, ReconnectionEvent, NetworkStatus as ClientNetworkStatus, NetworkMetrics};

/// 客户端连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum ClientConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// 客户端连接健康状态
#[derive(Debug, Clone)]
pub struct ClientConnectionHealth {
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub heartbeat_count: u64,
    pub missed_heartbeats: u64,
    pub avg_latency_ms: u64,
    pub connection_quality: f64, // 0.0 to 1.0
    pub network_status: ClientNetworkStatus,
    pub reconnect_attempts: u32,
}

/// 客户端连接配置
#[derive(Debug, Clone)]
pub struct ClientConnectionConfig {
    pub server_url: String,
    pub user_id: String,
    pub preferred_protocol: TransportProtocol,
    pub heartbeat_interval_ms: u64,
    pub heartbeat_timeout_ms: u64,
    pub max_missed_heartbeats: u64,
    pub connection_timeout_ms: u64,
    pub enable_auto_reconnect: bool,
    pub enable_network_monitoring: bool,
    pub network_check_interval_ms: u64,
}

impl Default for ClientConnectionConfig {
    fn default() -> Self {
        Self {
            server_url: "flare-im://localhost".to_string(),
            user_id: String::new(),
            preferred_protocol: TransportProtocol::QUIC,
            heartbeat_interval_ms: 30000, // 30秒
            heartbeat_timeout_ms: 60000,  // 60秒
            max_missed_heartbeats: 3,
            connection_timeout_ms: 10000, // 10秒
            enable_auto_reconnect: true,
            enable_network_monitoring: true,
            network_check_interval_ms: 10000, // 10秒
        }
    }
}

/// 客户端连接会话
#[derive(Debug, Clone)]
pub struct ClientConnectionSession {
    pub session_id: String,
    pub server_url: String,
    pub user_id: String,
    pub protocol: TransportProtocol,
    pub state: ClientConnectionState,
    pub health: ClientConnectionHealth,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

impl ClientConnectionSession {
    pub fn new(config: &ClientConnectionConfig) -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            server_url: config.server_url.clone(),
            user_id: config.user_id.clone(),
            protocol: config.preferred_protocol,
            state: ClientConnectionState::Disconnected,
            health: ClientConnectionHealth {
                last_heartbeat: chrono::Utc::now(),
                heartbeat_count: 0,
                missed_heartbeats: 0,
                avg_latency_ms: 0,
                connection_quality: 1.0,
                network_status: ClientNetworkStatus::Disconnected,
                reconnect_attempts: 0,
            },
            connected_at: None,
            last_activity: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn is_healthy(&self, config: &ClientConnectionConfig) -> bool {
        let now = chrono::Utc::now();
        let time_since_heartbeat = now.signed_duration_since(self.health.last_heartbeat);
        let timeout_duration = chrono::Duration::milliseconds(config.heartbeat_timeout_ms as i64);
        
        time_since_heartbeat < timeout_duration && 
        self.health.missed_heartbeats < config.max_missed_heartbeats
    }

    pub fn update_heartbeat(&mut self) {
        self.health.last_heartbeat = chrono::Utc::now();
        self.health.heartbeat_count += 1;
        self.health.missed_heartbeats = 0;
        self.last_activity = chrono::Utc::now();
    }

    pub fn mark_heartbeat_missed(&mut self) {
        self.health.missed_heartbeats += 1;
    }

    pub fn mark_reconnect_attempt(&mut self) {
        self.health.reconnect_attempts += 1;
    }
}

/// 客户端高级连接管理器
/// 提供完善的连接管理、心跳处理、自动重连、网络监控等功能
pub struct AdvancedClientConnectionManager {
    config: ClientConnectionConfig,
    session: Arc<RwLock<Option<ClientConnectionSession>>>,
    reconnection_strategy: Arc<RwLock<ReconnectionStrategy>>,
    connection_callbacks: Arc<RwLock<Vec<Box<dyn Fn(ClientConnectionState) + Send + Sync>>>>,
    reconnection_callbacks: Arc<RwLock<Vec<Box<dyn Fn(ReconnectionEvent) + Send + Sync>>>>,
    network_callbacks: Arc<RwLock<Vec<Box<dyn Fn(ClientNetworkStatus) + Send + Sync>>>>,
    
    // 后台任务
    heartbeat_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    network_monitor_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    reconnection_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    
    // 控制通道
    control_tx: Arc<Mutex<Option<mpsc::Sender<ConnectionCommand>>>>,
    control_rx: Arc<Mutex<Option<mpsc::Receiver<ConnectionCommand>>>>,
}

/// 连接命令
#[derive(Debug)]
enum ConnectionCommand {
    Connect,
    Disconnect,
    Reconnect,
    SendHeartbeat,
    Stop,
}

impl AdvancedClientConnectionManager {
    /// 创建新的客户端连接管理器
    pub fn new(config: ClientConnectionConfig) -> Self {
        let (control_tx, control_rx) = mpsc::channel(100);
        
        Self {
            config,
            session: Arc::new(RwLock::new(None)),
            reconnection_strategy: Arc::new(RwLock::new(ReconnectionStrategy {
                max_attempts: 5,
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
                jitter_enabled: true,
            })),
            connection_callbacks: Arc::new(RwLock::new(Vec::new())),
            reconnection_callbacks: Arc::new(RwLock::new(Vec::new())),
            network_callbacks: Arc::new(RwLock::new(Vec::new())),
            heartbeat_task: Arc::new(Mutex::new(None)),
            network_monitor_task: Arc::new(Mutex::new(None)),
            reconnection_task: Arc::new(Mutex::new(None)),
            control_tx: Arc::new(Mutex::new(Some(control_tx))),
            control_rx: Arc::new(Mutex::new(Some(control_rx))),
        }
    }

    /// 启动连接管理器
    pub async fn start(&self) -> Result<()> {
        info!("启动客户端连接管理器");
        
        // 创建初始会话
        {
            let mut session = self.session.write().await;
            *session = Some(ClientConnectionSession::new(&self.config));
        }
        
        // 启动心跳任务
        self.start_heartbeat_task().await?;
        
        // 启动网络监控
        if self.config.enable_network_monitoring {
            self.start_network_monitor_task().await?;
        }
        
        // 启动控制循环
        self.start_control_loop().await?;
        
        Ok(())
    }

    /// 停止连接管理器
    pub async fn stop(&self) -> Result<()> {
        info!("停止客户端连接管理器");
        
        // 发送停止命令
        if let Some(tx) = self.control_tx.lock().await.take() {
            let _ = tx.send(ConnectionCommand::Stop).await;
        }
        
        // 停止所有任务
        {
            let mut task = self.heartbeat_task.lock().await;
            if let Some(task) = task.take() {
                task.abort();
            }
        }
        
        {
            let mut task = self.network_monitor_task.lock().await;
            if let Some(task) = task.take() {
                task.abort();
            }
        }
        
        {
            let mut task = self.reconnection_task.lock().await;
            if let Some(task) = task.take() {
                task.abort();
            }
        }
        
        Ok(())
    }

    /// 启动心跳任务
    async fn start_heartbeat_task(&self) -> Result<()> {
        let session = Arc::clone(&self.session);
        let config = self.config.clone();
        let callbacks = Arc::clone(&self.connection_callbacks);
        
        let heartbeat_interval = Duration::from_millis(config.heartbeat_interval_ms);
        let mut interval_timer = interval(heartbeat_interval);
        
        let task = tokio::spawn(async move {
            loop {
                interval_timer.tick().await;
                
                let mut session_write = session.write().await;
                if let Some(session) = session_write.as_mut() {
                    if session.state == ClientConnectionState::Connected {
                        // 发送心跳
                        if let Err(e) = Self::send_heartbeat_to_server(session).await {
                            warn!("发送心跳失败: {}", e);
                            session.mark_heartbeat_missed();
                        } else {
                            session.update_heartbeat();
                        }
                        
                        // 检查连接健康状态
                        if !session.is_healthy(&config) {
                            warn!("连接不健康，标记为断开");
                            session.state = ClientConnectionState::Disconnected;
                            
                            // 触发回调
                            let callbacks_read = callbacks.read().await;
                            for callback in callbacks_read.iter() {
                                callback(ClientConnectionState::Disconnected);
                            }
                        }
                    }
                }
            }
        });
        
        {
            let mut task_guard = self.heartbeat_task.lock().await;
            *task_guard = Some(task);
        }
        
        Ok(())
    }

    /// 启动网络监控任务
    async fn start_network_monitor_task(&self) -> Result<()> {
        let session = Arc::clone(&self.session);
        let config = self.config.clone();
        let callbacks = Arc::clone(&self.network_callbacks);
        
        let check_interval = Duration::from_millis(config.network_check_interval_ms);
        let mut interval_timer = interval(check_interval);
        
        let task = tokio::spawn(async move {
            loop {
                interval_timer.tick().await;
                
                let network_status = Self::check_network_status().await;
                
                let mut session_write = session.write().await;
                if let Some(session) = session_write.as_mut() {
                    session.health.network_status = network_status.clone();
                    
                    // 根据网络状态调整连接质量
                    match network_status {
                        ClientNetworkStatus::Connected => session.health.connection_quality = 1.0,
                        ClientNetworkStatus::Poor => session.health.connection_quality = 0.5,
                        ClientNetworkStatus::Disconnected => session.health.connection_quality = 0.0,
                        _ => session.health.connection_quality = 0.8,
                    }
                }
                
                // 触发网络状态回调
                let callbacks_read = callbacks.read().await;
                for callback in callbacks_read.iter() {
                    callback(network_status.clone());
                }
            }
        });
        
        {
            let mut task_guard = self.network_monitor_task.lock().await;
            *task_guard = Some(task);
        }
        
        Ok(())
    }

    /// 启动控制循环
    async fn start_control_loop(&self) -> Result<()> {
        let session = Arc::clone(&self.session);
        let config = self.config.clone();
        let callbacks = Arc::clone(&self.connection_callbacks);
        
        let mut rx_guard = self.control_rx.lock().await;
        if let Some(mut rx) = rx_guard.take() {
            let task = tokio::spawn(async move {
                while let Some(command) = rx.recv().await {
                    match command {
                        ConnectionCommand::Connect => {
                            let mut session_write = session.write().await;
                            if let Some(session) = session_write.as_mut() {
                                session.state = ClientConnectionState::Connecting;
                                
                                // 触发回调
                                let callbacks_read = callbacks.read().await;
                                for callback in callbacks_read.iter() {
                                    callback(ClientConnectionState::Connecting);
                                }
                                
                                // 模拟连接过程
                                match Self::connect_to_server(session, &config).await {
                                    Ok(_) => {
                                        session.state = ClientConnectionState::Connected;
                                        session.connected_at = Some(chrono::Utc::now());
                                        info!("连接成功");
                                        
                                        // 触发回调
                                        let callbacks_read = callbacks.read().await;
                                        for callback in callbacks_read.iter() {
                                            callback(ClientConnectionState::Connected);
                                        }
                                    }
                                    Err(e) => {
                                        session.state = ClientConnectionState::Failed;
                                        error!("连接失败: {}", e);
                                        
                                        // 触发回调
                                        let callbacks_read = callbacks.read().await;
                                        for callback in callbacks_read.iter() {
                                            callback(ClientConnectionState::Failed);
                                        }
                                    }
                                }
                            }
                        }
                        ConnectionCommand::Disconnect => {
                            let mut session_write = session.write().await;
                            if let Some(session) = session_write.as_mut() {
                                session.state = ClientConnectionState::Disconnected;
                                session.connected_at = None;
                                info!("连接已断开");
                                
                                // 触发回调
                                let callbacks_read = callbacks.read().await;
                                for callback in callbacks_read.iter() {
                                    callback(ClientConnectionState::Disconnected);
                                }
                            }
                        }
                        ConnectionCommand::Reconnect => {
                            let mut session_write = session.write().await;
                            if let Some(session) = session_write.as_mut() {
                                session.state = ClientConnectionState::Reconnecting;
                                session.mark_reconnect_attempt();
                                info!("尝试重连，第{}次", session.health.reconnect_attempts);
                                
                                // 触发回调
                                let callbacks_read = callbacks.read().await;
                                for callback in callbacks_read.iter() {
                                    callback(ClientConnectionState::Reconnecting);
                                }
                                
                                // 模拟重连过程
                                match Self::connect_to_server(session, &config).await {
                                    Ok(_) => {
                                        session.state = ClientConnectionState::Connected;
                                        session.connected_at = Some(chrono::Utc::now());
                                        info!("重连成功");
                                        
                                        // 触发回调
                                        let callbacks_read = callbacks.read().await;
                                        for callback in callbacks_read.iter() {
                                            callback(ClientConnectionState::Connected);
                                        }
                                    }
                                    Err(e) => {
                                        session.state = ClientConnectionState::Failed;
                                        error!("重连失败: {}", e);
                                        
                                        // 触发回调
                                        let callbacks_read = callbacks.read().await;
                                        for callback in callbacks_read.iter() {
                                            callback(ClientConnectionState::Failed);
                                        }
                                    }
                                }
                            }
                        }
                        ConnectionCommand::SendHeartbeat => {
                            let mut session_write = session.write().await;
                            if let Some(session) = session_write.as_mut() {
                                if session.state == ClientConnectionState::Connected {
                                    if let Err(e) = Self::send_heartbeat_to_server(session).await {
                                        warn!("发送心跳失败: {}", e);
                                        session.mark_heartbeat_missed();
                                    } else {
                                        session.update_heartbeat();
                                    }
                                }
                            }
                        }
                        ConnectionCommand::Stop => {
                            info!("收到停止命令");
                            break;
                        }
                    }
                }
            });
            
            {
                let mut task_guard = self.reconnection_task.lock().await;
                *task_guard = Some(task);
            }
        }
        
        Ok(())
    }

    /// 连接到服务器
    async fn connect_to_server(session: &mut ClientConnectionSession, config: &ClientConnectionConfig) -> Result<()> {
        // 这里应该实现实际的连接逻辑
        // 为了演示，我们模拟连接过程
        let connect_timeout = Duration::from_millis(config.connection_timeout_ms);
        
        match timeout(connect_timeout, async {
            // 模拟网络延迟
            sleep(Duration::from_millis(100)).await;
            
            // 模拟连接成功率
            if rand::random::<f64>() > 0.1 { // 90%成功率
                Ok(())
            } else {
                Err(FlareError::ConnectionFailed("连接超时".to_string()))
            }
        }).await {
            Ok(result) => result,
            Err(_) => Err(FlareError::ConnectionFailed("连接超时".to_string())),
        }
    }

    /// 发送心跳到服务器
    async fn send_heartbeat_to_server(session: &ClientConnectionSession) -> Result<()> {
        // 这里应该实现实际的心跳发送逻辑
        // 为了演示，我们模拟心跳发送
        sleep(Duration::from_millis(10)).await;
        
        // 模拟心跳成功率
        if rand::random::<f64>() > 0.05 { // 95%成功率
            Ok(())
        } else {
            Err(FlareError::ConnectionFailed("心跳发送失败".to_string()))
        }
    }

    /// 检查网络状态
    async fn check_network_status() -> ClientNetworkStatus {
        // 这里应该实现实际的网络状态检测
        // 可以基于ping、DNS解析、HTTP请求等
        // 为了演示，我们模拟网络状态
        let random = rand::random::<f64>();
        
        match random {
            r if r > 0.8 => ClientNetworkStatus::Connected,
            r if r > 0.6 => ClientNetworkStatus::Poor,
            _ => ClientNetworkStatus::Disconnected,
        }
    }

    /// 发送控制命令
    async fn send_command(&self, command: ConnectionCommand) -> Result<()> {
        if let Some(tx) = &*self.control_tx.lock().await {
            tx.send(command).await
                .map_err(|_| FlareError::InternalError("控制通道已关闭".to_string()))?;
        }
        Ok(())
    }
}

#[async_trait]
impl ClientConnectionManager for AdvancedClientConnectionManager {
    async fn connect(&mut self, server_url: &str) -> Result<()> {
        self.config.server_url = server_url.to_string();
        self.send_command(ConnectionCommand::Connect).await
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.send_command(ConnectionCommand::Disconnect).await
    }

    async fn is_connected(&self) -> bool {
        let session = self.session.read().await;
        if let Some(session) = session.as_ref() {
            session.state == ClientConnectionState::Connected
        } else {
            false
        }
    }

    async fn reconnect(&mut self) -> Result<()> {
        self.send_command(ConnectionCommand::Reconnect).await
    }

    async fn get_connection_info(&self) -> Result<Option<ConnectionInfo>> {
        let session = self.session.read().await;
        if let Some(session) = session.as_ref() {
            if session.state == ClientConnectionState::Connected {
                Ok(Some(ConnectionInfo {
                    user_id: session.user_id.clone(),
                    session_id: session.session_id.clone(),
                    remote_addr: session.server_url.clone(),
                    protocol: session.protocol,
                    status: match session.state {
                        ClientConnectionState::Connected => crate::common::ConnectionStatus::Connected,
                        ClientConnectionState::Connecting => crate::common::ConnectionStatus::Connecting,
                        ClientConnectionState::Reconnecting => crate::common::ConnectionStatus::Reconnecting,
                        ClientConnectionState::Failed => crate::common::ConnectionStatus::Failed,
                        ClientConnectionState::Disconnected => crate::common::ConnectionStatus::Disconnected,
                    },
                    connected_at: session.connected_at.unwrap_or_else(chrono::Utc::now),
                    last_activity: session.health.last_heartbeat,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn send_heartbeat(&self) -> Result<()> {
        self.send_command(ConnectionCommand::SendHeartbeat).await
    }

    fn set_connection_callback(&mut self, callback: Box<dyn Fn(crate::common::ConnectionStatus) + Send + Sync>) {
        // 这里需要将ConnectionStatus转换为ClientConnectionState
        let wrapped_callback = Box::new(move |state: ClientConnectionState| {
            let status = match state {
                ClientConnectionState::Connected => crate::common::ConnectionStatus::Connected,
                ClientConnectionState::Disconnected => crate::common::ConnectionStatus::Disconnected,
                ClientConnectionState::Connecting => crate::common::ConnectionStatus::Connecting,
                ClientConnectionState::Reconnecting => crate::common::ConnectionStatus::Reconnecting,
                ClientConnectionState::Failed => crate::common::ConnectionStatus::Failed,
            };
            callback(status);
        });
        
        let callbacks = Arc::clone(&self.connection_callbacks);
        tokio::spawn(async move {
            let mut callbacks_write = callbacks.write().await;
            callbacks_write.push(wrapped_callback);
        });
    }
}

#[async_trait]
impl ReconnectionManager for AdvancedClientConnectionManager {
    fn enable_auto_reconnect(&mut self, enabled: bool) {
        self.config.enable_auto_reconnect = enabled;
    }

    fn set_reconnection_strategy(&mut self, strategy: ReconnectionStrategy) {
        let strategy_arc = Arc::clone(&self.reconnection_strategy);
        tokio::spawn(async move {
            let mut strategy_write = strategy_arc.write().await;
            *strategy_write = strategy;
        });
    }

    async fn reconnect(&mut self) -> Result<()> {
        self.send_command(ConnectionCommand::Reconnect).await
    }

    async fn get_reconnection_stats(&self) -> Result<crate::client::traits::ReconnectionStats> {
        let session = self.session.read().await;
        if let Some(session) = session.as_ref() {
            Ok(crate::client::traits::ReconnectionStats {
                total_reconnections: session.health.reconnect_attempts as u64,
                successful_reconnections: if session.state == ClientConnectionState::Connected { 1 } else { 0 },
                failed_reconnections: session.health.reconnect_attempts.saturating_sub(1) as u64,
                avg_reconnection_time_ms: 1000, // 模拟值
                last_reconnection: session.connected_at,
            })
        } else {
            Ok(crate::client::traits::ReconnectionStats {
                total_reconnections: 0,
                successful_reconnections: 0,
                failed_reconnections: 0,
                avg_reconnection_time_ms: 0,
                last_reconnection: None,
            })
        }
    }

    fn set_reconnection_callback(&mut self, callback: Box<dyn Fn(ReconnectionEvent) + Send + Sync>) {
        let callbacks = Arc::clone(&self.reconnection_callbacks);
        tokio::spawn(async move {
            let mut callbacks_write = callbacks.write().await;
            callbacks_write.push(callback);
        });
    }
}

#[async_trait]
impl NetworkMonitor for AdvancedClientConnectionManager {
    async fn start_monitoring(&mut self) -> Result<()> {
        // 网络监控已经在start()方法中启动
        Ok(())
    }

    async fn stop_monitoring(&mut self) -> Result<()> {
        let mut task = self.network_monitor_task.lock().await;
        if let Some(task) = task.take() {
            task.abort();
        }
        Ok(())
    }

    async fn get_network_status(&self) -> Result<ClientNetworkStatus> {
        let session = self.session.read().await;
        if let Some(session) = session.as_ref() {
            Ok(session.health.network_status.clone())
        } else {
            Ok(ClientNetworkStatus::Disconnected)
        }
    }

    async fn get_network_metrics(&self) -> Result<NetworkMetrics> {
        let session = self.session.read().await;
        if let Some(session) = session.as_ref() {
            Ok(NetworkMetrics {
                latency_ms: session.health.avg_latency_ms,
                bandwidth_bps: 1000000.0, // 模拟1Mbps
                packet_loss_rate: 0.01, // 模拟1%丢包率
                connection_quality: session.health.connection_quality,
                timestamp: chrono::Utc::now(),
            })
        } else {
            Ok(NetworkMetrics {
                latency_ms: 0,
                bandwidth_bps: 0.0,
                packet_loss_rate: 1.0,
                connection_quality: 0.0,
                timestamp: chrono::Utc::now(),
            })
        }
    }

    fn set_network_change_callback(&mut self, callback: Box<dyn Fn(ClientNetworkStatus) + Send + Sync>) {
        let callbacks = Arc::clone(&self.network_callbacks);
        tokio::spawn(async move {
            let mut callbacks_write = callbacks.write().await;
            callbacks_write.push(callback);
        });
    }
}

impl Drop for AdvancedClientConnectionManager {
    fn drop(&mut self) {
        // 在析构时确保所有任务都被正确停止
        let _ = tokio::runtime::Handle::current().block_on(async {
            self.stop().await
        });
    }
} 