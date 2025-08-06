//! Flare IM 内存连接管理器实现
//!
//! 提供基于内存的连接管理，适用于单机部署

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use async_trait::async_trait;
use tokio::sync::{RwLock, Mutex};
use tokio::time::interval;
use tracing::{info, warn, debug, error};        

use crate::common::{
    conn::{Connection, ConnectionEvent, ConnectionState, ProtoMessage},
    Result, FlareError, TransportProtocol
};

use super::{
    ServerConnectionManager, ServerConnectionManagerConfig, ServerConnectionInfo,
    ServerConnectionManagerStats, ConnectionEventCallback
};

/// 内存中的连接记录
#[derive(Debug)]
struct MemoryConnectionRecord {
    /// 连接实例
    connection: Box<dyn Connection>,
    /// 连接信息
    info: ServerConnectionInfo,
    /// 最后活动时间
    last_activity: Instant,
}

impl MemoryConnectionRecord {
    /// 创建新的连接记录
    fn new(
        connection: Box<dyn Connection>,
        user_id: String,
        session_id: String,
    ) -> Self {
        let info = ServerConnectionInfo::new(
            connection.id().to_string(),
            user_id,
            session_id,
            connection.remote_addr().to_string(),
            connection.platform(),
            match connection.protocol() {
                "QUIC" => TransportProtocol::QUIC,
                "WebSocket" => TransportProtocol::WebSocket,
                _ => TransportProtocol::WebSocket,
            },
        );
        
        Self {
            connection,
            info,
            last_activity: Instant::now(),
        }
    }

    /// 更新活动时间
    fn update_activity(&mut self) {
        self.last_activity = Instant::now();
        self.info.update_activity();
    }

    /// 更新心跳
    fn update_heartbeat(&mut self) {
        self.info.update_heartbeat();
        self.update_activity();
    }

    /// 检查连接是否过期
    fn is_expired(&self, timeout_secs: u64) -> bool {
        self.last_activity.elapsed().as_secs() > timeout_secs
    }

    /// 检查连接是否健康
    fn is_healthy(&self, config: &ServerConnectionManagerConfig) -> bool {
        self.info.is_healthy(config)
    }
}

/// 基于内存的连接管理器实现
pub struct MemoryServerConnectionManager {
    /// 配置信息
    config: ServerConnectionManagerConfig,
    /// 连接存储：user_id:session_id -> 连接记录
    connections: Arc<RwLock<HashMap<String, MemoryConnectionRecord>>>,
    /// 用户会话映射：user_id -> session_ids
    user_sessions: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// 统计信息
    stats: Arc<RwLock<ServerConnectionManagerStats>>,
    /// 事件回调
    event_callback: Arc<RwLock<Option<ConnectionEventCallback>>>,
    /// 心跳监控任务
    heartbeat_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 清理任务
    cleanup_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl MemoryServerConnectionManager {
    /// 创建新的内存连接管理器
    pub fn new(config: ServerConnectionManagerConfig) -> Self {
        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ServerConnectionManagerStats::default())),
            event_callback: Arc::new(RwLock::new(None)),
            heartbeat_task: Arc::new(Mutex::new(None)),
            cleanup_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 创建默认配置的连接管理器
    pub fn new_default() -> Self {
        Self::new(ServerConnectionManagerConfig::default())
    }

    /// 生成连接键
    fn connection_key(user_id: &str, session_id: &str) -> String {
        format!("{}:{}", user_id, session_id)
    }

    /// 触发连接事件
    async fn trigger_event(&self, user_id: String, event: ConnectionEvent) {
        if let Some(callback) = &*self.event_callback.read().await {
            callback(user_id, event);
        }
    }

    /// 更新统计信息
    async fn update_stats(&self) {
        let connections = self.connections.read().await;
        let user_sessions = self.user_sessions.read().await;
        
        let mut stats = self.stats.write().await;
        stats.total_connections = connections.len();
        stats.online_users = user_sessions.len();
        stats.last_updated = chrono::Utc::now();
        
        // 计算活跃连接数
        let mut active_count = 0;
        let mut disconnected_count = 0;
        
        for record in connections.values() {
            match record.info.state {
                ConnectionState::Connected => active_count += 1,
                ConnectionState::Disconnected => disconnected_count += 1,
                _ => {}
            }
        }
        
        stats.active_connections = active_count;
        stats.disconnected_connections = disconnected_count;
    }

    /// 启动心跳监控任务
    async fn start_heartbeat_monitoring(&self) -> Result<()> {
        let connections = Arc::clone(&self.connections);
        let config = self.config.clone();
        let event_callback = Arc::clone(&self.event_callback);
        let running = Arc::clone(&self.running);
        
        let task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(config.heartbeat_interval_ms));
            
            while *running.read().await {
                interval.tick().await;
                
                let mut connections_write = connections.write().await;
                let keys_to_remove: Vec<String> = Vec::new();
                
                for (key, record) in connections_write.iter_mut() {
                    // 检查连接健康状态
                    if !record.is_healthy(&config) {
                        record.info.mark_heartbeat_missed();
                        
                        // 如果心跳丢失次数过多，标记为断开
                        if record.info.missed_heartbeats >= config.max_missed_heartbeats {
                            record.info.state = ConnectionState::Disconnected;
                            
                            // 触发断开事件
                            if let Some(callback) = &*event_callback.read().await {
                                let user_id = record.info.user_id.clone();
                                callback(user_id, ConnectionEvent::Disconnected);
                            }
                            
                            debug!("连接心跳超时，标记为断开: {}", key);
                        }
                    }
                }
                
                // 移除断开的连接
                for key in keys_to_remove {
                    if let Some(_record) = connections_write.remove(&key) {
                        debug!("移除断开的连接: {}", key);
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

    /// 启动清理任务
    async fn start_cleanup_task(&self) -> Result<()> {
        let connections = Arc::clone(&self.connections);
        let user_sessions = Arc::clone(&self.user_sessions);
        let config = self.config.clone();
        let running = Arc::clone(&self.running);
        
        let task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(config.cleanup_interval_ms));
            
            while *running.read().await {
                interval.tick().await;
                
                let timeout_secs = config.connection_timeout_ms / 1000;
                let mut connections_write = connections.write().await;
                let mut user_sessions_write = user_sessions.write().await;
                
                let keys_to_remove: Vec<String> = connections_write
                    .iter()
                    .filter(|(_, record)| record.is_expired(timeout_secs))
                    .map(|(key, _)| key.clone())
                    .collect();
                
                for key in keys_to_remove {
                    if let Some(record) = connections_write.remove(&key) {
                        // 从用户会话映射中移除
                        if let Some(sessions) = user_sessions_write.get_mut(&record.info.user_id) {
                            sessions.remove(&record.info.session_id);
                            if sessions.is_empty() {
                                user_sessions_write.remove(&record.info.user_id);
                            }
                        }
                        
                        debug!("清理过期连接: {}", key);
                    }
                }
            }
        });
        
        {
            let mut task_guard = self.cleanup_task.lock().await;
            *task_guard = Some(task);
        }
        
        Ok(())
    }
}

#[async_trait]
impl ServerConnectionManager for MemoryServerConnectionManager {
    async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        
        *running = true;
        
        // 启动心跳监控任务
        self.start_heartbeat_monitoring().await?;
        
        // 启动清理任务
        self.start_cleanup_task().await?;
        
        info!("内存连接管理器已启动");
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        
        *running = false;
        
        // 停止心跳任务
        {
            let mut task_guard = self.heartbeat_task.lock().await;
            if let Some(task) = task_guard.take() {
                task.abort();
            }
        }
        
        // 停止清理任务
        {
            let mut task_guard = self.cleanup_task.lock().await;
            if let Some(task) = task_guard.take() {
                task.abort();
            }
        }
        
        info!("内存连接管理器已停止");
        Ok(())
    }
    
    async fn add_connection(
        &self,
        connection: Box<dyn Connection>,
        user_id: String,
        session_id: String,
    ) -> Result<()> {
        let key = Self::connection_key(&user_id, &session_id);
        
        info!("开始添加连接: 用户={}, 会话={}, 键={}", user_id, session_id, key);
        
        // 检查连接数限制
        {
            let connections = self.connections.read().await;
            let current_count = connections.len();
            let max_connections = self.config.max_connections;
            
            info!("连接数检查: 当前={}, 最大={}", current_count, max_connections);
            
            if current_count >= max_connections {
                let error_msg = format!("连接数已达上限: 当前={}, 最大={}", current_count, max_connections);
                error!("{}", error_msg);
                return Err(FlareError::ResourceExhausted(error_msg));
            }
        }
        
        info!("连接数检查通过，开始创建连接记录");
        
        // 创建连接记录
        let record = MemoryConnectionRecord::new(connection, user_id.clone(), session_id.clone());
        info!("连接记录创建成功");
        
        // 添加到连接存储
        {
            info!("开始添加到连接存储");
            let mut connections_write = self.connections.write().await;
            connections_write.insert(key.clone(), record);
            let new_count = connections_write.len();
            info!("连接存储更新成功，当前连接数: {}", new_count);
        }
        
        // 更新用户会话映射
        {
            info!("开始更新用户会话映射");
            let mut user_sessions_write = self.user_sessions.write().await;
            user_sessions_write
                .entry(user_id.clone())
                .or_insert_with(HashSet::new)
                .insert(session_id.clone());
            let user_count = user_sessions_write.len();
            info!("用户会话映射更新成功，当前在线用户数: {}", user_count);
        }
        
        // 更新统计信息
        info!("开始更新统计信息");
        self.update_stats().await;
        info!("统计信息更新完成");
        
        // 触发连接事件
        info!("开始触发连接事件");
        self.trigger_event(user_id.clone(), ConnectionEvent::Connected).await;
        info!("连接事件触发完成");
        
        info!("连接添加成功: 用户={}, 会话={}, 键={}", user_id, session_id, key);
        Ok(())
    }
    
    async fn remove_connection(&self, user_id: &str, session_id: &str) -> Result<()> {
        let key = Self::connection_key(user_id, session_id);
        
        // 从连接存储中移除
        let connection_removed = {
            let mut connections_write = self.connections.write().await;
            connections_write.remove(&key).is_some()
        };
        
        if connection_removed {
            // 从用户会话映射中移除
            {
                let mut user_sessions_write = self.user_sessions.write().await;
                if let Some(sessions) = user_sessions_write.get_mut(user_id) {
                    sessions.remove(session_id);
                    if sessions.is_empty() {
                        user_sessions_write.remove(user_id);
                    }
                }
            }
            
            // 更新统计信息
            self.update_stats().await;
            
            // 触发断开事件
            self.trigger_event(user_id.to_string(), ConnectionEvent::Disconnected).await;
            
            debug!("移除连接: {}", key);
        }
        
        Ok(())
    }
    
    async fn get_connection(&self, user_id: &str, session_id: &str) -> Result<Option<Box<dyn Connection>>> {
        let key = Self::connection_key(user_id, session_id);
        
        // 先检查连接是否存在
        {
            let connections_read = self.connections.read().await;
            if connections_read.get(&key).is_none() {
                return Ok(None);
            }
        }
        
        // 更新活动时间
        {
            let mut connections_write = self.connections.write().await;
            if let Some(record) = connections_write.get_mut(&key) {
                record.update_activity();
            }
        }
        
        // 返回连接
        {
            let connections_read = self.connections.read().await;
            if let Some(record) = connections_read.get(&key) {
                Ok(Some(record.connection.clone_box()))
            } else {
                Ok(None)
            }
        }
    }
    
    async fn get_user_connections(&self, user_id: &str) -> Result<Vec<Box<dyn Connection>>> {
        let user_sessions = self.user_sessions.read().await;
        let connections = self.connections.read().await;
        
        let mut user_connections = Vec::new();
        
        if let Some(sessions) = user_sessions.get(user_id) {
            for session_id in sessions {
                let key = Self::connection_key(user_id, session_id);
                if let Some(record) = connections.get(&key) {
                    user_connections.push(record.connection.clone_box());
                }
            }
        }
        
        Ok(user_connections)
    }
    
    async fn is_user_online(&self, user_id: &str) -> Result<bool> {
        let user_sessions = self.user_sessions.read().await;
        Ok(user_sessions.contains_key(user_id))
    }
    
    async fn get_online_user_count(&self) -> Result<usize> {
        let user_sessions = self.user_sessions.read().await;
        Ok(user_sessions.len())
    }
    
    async fn get_connection_count(&self) -> Result<usize> {
        let connections = self.connections.read().await;
        Ok(connections.len())
    }
    
    async fn send_message_to_user(&self, user_id: &str, message: ProtoMessage) -> Result<usize> {
        let user_connections = self.get_user_connections(user_id).await?;
        let mut success_count = 0;
        
        for connection in user_connections {
            match connection.send(message.clone()).await {
                Ok(_) => {
                    success_count += 1;
                    
                    // 更新统计信息
                    {
                        let mut stats = self.stats.write().await;
                        stats.total_messages += 1;
                        stats.total_bytes += message.payload.len() as u64;
                    }
                }
                Err(e) => {
                    warn!("发送消息失败: {}", e);
                }
            }
        }
        
        Ok(success_count)
    }
    
    async fn broadcast_message(&self, message: ProtoMessage) -> Result<usize> {
        let connections = self.connections.read().await;
        let mut success_count = 0;
        
        for record in connections.values() {
            match record.connection.send(message.clone()).await {
                Ok(_) => {
                    success_count += 1;
                    
                    // 更新统计信息
                    {
                        let mut stats = self.stats.write().await;
                        stats.total_messages += 1;
                        stats.total_bytes += message.payload.len() as u64;
                    }
                }
                Err(e) => {
                    warn!("广播消息失败: {}", e);
                }
            }
        }
        
        Ok(success_count)
    }
    
    async fn update_heartbeat(&self, user_id: &str, session_id: &str) -> Result<()> {
        let key = Self::connection_key(user_id, session_id);
        let mut connections = self.connections.write().await;
        
        if let Some(record) = connections.get_mut(&key) {
            record.update_heartbeat();
            debug!("更新心跳: {}", key);
        }
        
        Ok(())
    }
    
    async fn cleanup_expired_connections(&self, timeout_secs: u64) -> Result<usize> {
        let mut connections = self.connections.write().await;
        let mut user_sessions = self.user_sessions.write().await;
        
        let keys_to_remove: Vec<String> = connections
            .iter()
            .filter(|(_, record)| record.is_expired(timeout_secs))
            .map(|(key, _)| key.clone())
            .collect();
        
        let mut removed_count = 0;
        
        for key in keys_to_remove {
            if let Some(record) = connections.remove(&key) {
                // 从用户会话映射中移除
                if let Some(sessions) = user_sessions.get_mut(&record.info.user_id) {
                    sessions.remove(&record.info.session_id);
                    if sessions.is_empty() {
                        user_sessions.remove(&record.info.user_id);
                    }
                }
                
                removed_count += 1;
                debug!("清理过期连接: {}", key);
            }
        }
        
        // 更新统计信息
        drop(connections);
        drop(user_sessions);
        self.update_stats().await;
        
        info!("清理了 {} 个过期连接", removed_count);
        Ok(removed_count)
    }
    
    async fn force_disconnect_user(&self, user_id: &str, _reason: Option<String>) -> Result<usize> {
        let user_connections = self.get_user_connections(user_id).await?;
        let mut disconnected_count = 0;
        
        for connection in user_connections {
            match connection.close().await {
                Ok(_) => {
                    disconnected_count += 1;
                }
                Err(e) => {
                    warn!("强制断开连接失败: {}", e);
                }
            }
        }
        
        // 从存储中移除所有连接
        {
            let mut connections = self.connections.write().await;
            let mut user_sessions = self.user_sessions.write().await;
            
            let keys_to_remove: Vec<String> = connections
                .keys()
                .filter(|key| key.starts_with(&format!("{}:", user_id)))
                .cloned()
                .collect();
            
            for key in keys_to_remove {
                connections.remove(&key);
            }
            
            user_sessions.remove(user_id);
        }
        
        // 更新统计信息
        self.update_stats().await;
        
        // 触发断开事件
        self.trigger_event(
            user_id.to_string(),
            ConnectionEvent::Disconnected
        ).await;
        
        info!("强制断开用户 {} 的 {} 个连接", user_id, disconnected_count);
        Ok(disconnected_count)
    }
    
    async fn get_connection_info(&self, user_id: &str, session_id: &str) -> Result<Option<ServerConnectionInfo>> {
        let key = Self::connection_key(user_id, session_id);
        let connections = self.connections.read().await;
        
        if let Some(record) = connections.get(&key) {
            Ok(Some(record.info.clone()))
        } else {
            Ok(None)
        }
    }
    
    async fn get_user_connection_infos(&self, user_id: &str) -> Result<Vec<ServerConnectionInfo>> {
        let user_sessions = self.user_sessions.read().await;
        let connections = self.connections.read().await;
        
        let mut infos = Vec::new();
        
        if let Some(sessions) = user_sessions.get(user_id) {
            for session_id in sessions {
                let key = Self::connection_key(user_id, session_id);
                if let Some(record) = connections.get(&key) {
                    infos.push(record.info.clone());
                }
            }
        }
        
        Ok(infos)
    }
    
    async fn get_stats(&self) -> Result<ServerConnectionManagerStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    async fn set_connection_event_callback(&self, callback: ConnectionEventCallback) {
        let mut callback_guard = self.event_callback.write().await;
        *callback_guard = Some(callback);
    }
    
    fn get_config(&self) -> &ServerConnectionManagerConfig {
        &self.config
    }
}

impl Drop for MemoryServerConnectionManager {
    fn drop(&mut self) {
        // 确保在析构时停止所有任务
        let runtime = tokio::runtime::Handle::current();
        if let Ok(()) = runtime.block_on(self.stop()) {
            debug!("内存连接管理器已正确停止");
        }
    }
} 