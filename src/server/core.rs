//! Flare IM 服务端核心模块
//!
//! 提供统一的QUIC和WebSocket服务端

use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tracing::{info, error, debug};

use crate::common::{
    conn::{ConnectionEvent, ProtoMessage, Platform},
    Result,
};

use super::{
    config::{ServerConfig, ConnectionManagerConfig},
    handlers::{AuthHandler, MessageHandler, EventHandler, DefaultAuthHandler, DefaultMessageHandler, DefaultEventHandler},
    conn_manager::{
        MemoryServerConnectionManager, ServerConnectionManagerStats, 
        ServerConnectionManagerConfig, ServerConnectionManager
    },
    websocket_server::WebSocketServer,
    quic_server::QuicServer,
    message_center::MessageProcessingCenter,
};

/// Flare IM 服务端
/// 提供统一的QUIC和WebSocket服务端
pub struct FlareIMServer {
    /// 配置
    config: ServerConfig,
    /// 连接管理器
    connection_manager: Arc<MemoryServerConnectionManager>,
    /// 消息处理中心
    message_center: Arc<MessageProcessingCenter>,
    /// WebSocket服务器
    websocket_server: Option<Arc<WebSocketServer>>,
    /// QUIC服务器
    quic_server: Option<Arc<QuicServer>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
    /// 服务器任务
    server_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl FlareIMServer {
    /// 创建新的 Flare IM 服务端
    pub fn new(config: ServerConfig) -> Self {
        // 将 ConnectionManagerConfig 转换为 ServerConnectionManagerConfig
        let conn_config = ServerConnectionManagerConfig {
            max_connections: config.connection_manager.max_connections,
            connection_timeout_ms: config.connection_manager.connection_timeout_ms,
            heartbeat_interval_ms: config.connection_manager.heartbeat_interval_ms,
            heartbeat_timeout_ms: config.connection_manager.heartbeat_timeout_ms,
            max_missed_heartbeats: config.connection_manager.max_missed_heartbeats as u64,
            cleanup_interval_ms: config.connection_manager.cleanup_interval_ms,
            enable_auto_reconnect: config.connection_manager.enable_auto_reconnect,
            max_reconnect_attempts: config.connection_manager.max_reconnect_attempts,
            reconnect_delay_ms: config.connection_manager.reconnect_delay_ms,
        };
        
        let connection_manager = Arc::new(MemoryServerConnectionManager::new(conn_config));
        
        // 创建消息处理中心
        let message_center = Arc::new(MessageProcessingCenter::new(
            connection_manager.clone(),
            None, // auth_handler
            None, // message_handler
            None, // event_handler
        ));
        
        Self {
            config,
            connection_manager,
            message_center,
            websocket_server: None,
            quic_server: None,
            running: Arc::new(RwLock::new(false)),
            server_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// 设置认证处理器
    pub fn with_auth_handler(mut self, handler: Arc<dyn AuthHandler>) -> Self {
        // 更新消息处理中心
        let new_center = Arc::new(MessageProcessingCenter::new(
            self.connection_manager.clone(),
            Some(handler),
            self.message_center.get_message_handler(),
            self.message_center.get_event_handler(),
        ));
        self.message_center = new_center;
        self
    }

    /// 设置消息处理器
    pub fn with_message_handler(mut self, handler: Arc<dyn MessageHandler>) -> Self {
        let new_center = Arc::new(MessageProcessingCenter::new(
            self.connection_manager.clone(),
            self.message_center.get_auth_handler(),
            Some(handler),
            self.message_center.get_event_handler(),
        ));
        self.message_center = new_center;
        self
    }

    /// 设置事件处理器
    pub fn with_event_handler(mut self, handler: Arc<dyn EventHandler>) -> Self {
        let new_center = Arc::new(MessageProcessingCenter::new(
            self.connection_manager.clone(),
            self.message_center.get_auth_handler(),
            self.message_center.get_message_handler(),
            Some(handler),
        ));
        self.message_center = new_center;
        self
    }

    /// 启动服务端
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        
        *running = true;
        
        // 启动连接管理器
        ServerConnectionManager::start(&*self.connection_manager).await?;
        
        // 设置连接事件回调
        self.setup_connection_event_callback().await;
        
        // 启动WebSocket服务器
        if self.config.websocket.enabled {
            self.start_websocket_server().await?;
        }
        
        // 启动QUIC服务器
        if self.config.quic.enabled {
            self.start_quic_server().await?;
        }
        
        info!("Flare IM 服务端已启动");
        info!("WebSocket: {} (启用: {})", self.config.websocket.bind_addr, self.config.websocket.enabled);
        info!("QUIC: {} (启用: {})", self.config.quic.bind_addr, self.config.quic.enabled);

        Ok(())
    }

    /// 停止服务端
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        
        *running = false;
        
        // 停止所有服务器任务
        {
            let mut tasks = self.server_tasks.lock().await;
            for task in tasks.drain(..) {
                task.abort();
            }
        }
        
        // 停止连接管理器
        ServerConnectionManager::stop(&*self.connection_manager).await?;
        
        info!("Flare IM 服务端已停止");
        Ok(())
    }

    /// 获取连接管理器
    pub fn get_connection_manager(&self) -> Arc<MemoryServerConnectionManager> {
        Arc::clone(&self.connection_manager)
    }
    
    /// 获取统计信息
    pub async fn get_stats(&self) -> Result<ServerConnectionManagerStats> {
        ServerConnectionManager::get_stats(&*self.connection_manager).await
    }

    /// 统一消息处理入口
    /// 
    /// 这是FlareIMServer的核心消息处理方法，负责：
    /// 1. 消息路由和分发
    /// 2. 调用消息处理器
    /// 3. 触发相关事件
    pub async fn handle_message(&self, user_id: &str, session_id: &str, message: ProtoMessage) -> Result<ProtoMessage> {
        // 使用消息处理中心统一处理
        self.message_center.process_message(user_id, session_id, message).await
    }



    /// 广播消息给所有用户
    pub async fn broadcast_message(&self, message: ProtoMessage) -> Result<usize> {
        self.connection_manager.broadcast_message(message).await
    }

    /// 发送消息给指定用户
    pub async fn send_message_to_user(&self, user_id: &str, message: ProtoMessage) -> Result<usize> {
        self.connection_manager.send_message_to_user(user_id, message).await
    }

    /// 强制断开用户连接
    pub async fn force_disconnect_user(&self, user_id: &str, reason: Option<String>) -> Result<usize> {
        self.connection_manager.force_disconnect_user(user_id, reason).await
    }
    
    /// 设置连接事件回调
    async fn setup_connection_event_callback(&self) {
        let message_center = Arc::clone(&self.message_center);
        
        ServerConnectionManager::set_connection_event_callback(&*self.connection_manager, Box::new(move |user_id, event| {
            let message_center = message_center.clone();
            
            tokio::spawn(async move {
                // 获取处理器
                let event_handler = message_center.get_event_handler();
                let message_handler = message_center.get_message_handler();
                
                // 调用事件处理器
                if let Some(handler) = &event_handler {
                    if let Err(e) = handler.handle_connection_event(&user_id, event.clone()).await {
                        error!("事件处理失败: {}", e);
                    }
                }
                
                // 处理特定事件
                match event {
                    ConnectionEvent::Connected => {
                        if let Some(handler) = &message_handler {
                            if let Err(e) = handler.handle_user_connect(&user_id, "session", Platform::Unknown).await {
                                error!("用户连接处理失败: {}", e);
                            }
                        }
                    }
                    ConnectionEvent::Disconnected => {
                        if let Some(handler) = &message_handler {
                            if let Err(e) = handler.handle_user_disconnect(&user_id, "session").await {
                                error!("用户断开处理失败: {}", e);
                            }
                        }
                    }
                    ConnectionEvent::Heartbeat => {
                        if let Some(handler) = &message_handler {
                            if let Err(e) = handler.handle_heartbeat(&user_id, "session").await {
                                error!("心跳处理失败: {}", e);
                            }
                        }
                    }
                    _ => {}
                }
            });
        }));
    }
    
    /// 启动WebSocket服务器
    async fn start_websocket_server(&self) -> Result<()> {
        let config = self.config.websocket.clone();
        let connection_manager = Arc::clone(&self.connection_manager);
        let message_center = Arc::clone(&self.message_center);
        let running = Arc::clone(&self.running);
        
        let task = tokio::spawn(async move {
            let server = WebSocketServer::new(
                config,
                connection_manager,
                message_center,
                running,
            );
            
            if let Err(e) = server.start().await {
                error!("WebSocket服务器启动失败: {}", e);
            }
        });
        
        {
            let mut tasks = self.server_tasks.lock().await;
            tasks.push(task);
        }
        
        Ok(())
    }
    
    /// 启动QUIC服务器
    async fn start_quic_server(&self) -> Result<()> {
        let config = self.config.quic.clone();
        let connection_manager = Arc::clone(&self.connection_manager);
        let message_center = Arc::clone(&self.message_center);
        let running = Arc::clone(&self.running);
        
        let task = tokio::spawn(async move {
            let server = QuicServer::new(
                config,
                connection_manager,
                message_center,
                running,
            );
            
            if let Err(e) = server.start().await {
                error!("QUIC服务器启动失败: {}", e);
            }
        });
        
        {
            let mut tasks = self.server_tasks.lock().await;
            tasks.push(task);
        }
        
        Ok(())
    }
}

/// Flare IM 服务端构建器
pub struct FlareIMServerBuilder {
    config: ServerConfig,
    auth_handler: Option<Arc<dyn AuthHandler>>,
    message_handler: Option<Arc<dyn MessageHandler>>,
    event_handler: Option<Arc<dyn EventHandler>>,
}

impl FlareIMServerBuilder {
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
            auth_handler: None,
            message_handler: None,
            event_handler: None,
        }
    }
    
    pub fn with_config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }
    
    pub fn with_auth_handler(mut self, handler: Arc<dyn AuthHandler>) -> Self {
        self.auth_handler = Some(handler);
        self
    }

    pub fn with_message_handler(mut self, handler: Arc<dyn MessageHandler>) -> Self {
        self.message_handler = Some(handler);
        self
    }

    pub fn with_event_handler(mut self, handler: Arc<dyn EventHandler>) -> Self {
        self.event_handler = Some(handler);
        self
    }

    pub fn websocket_addr(mut self, addr: std::net::SocketAddr) -> Self {
        self.config.websocket.bind_addr = addr;
        self
    }

    pub fn quic_addr(mut self, addr: std::net::SocketAddr) -> Self {
        self.config.quic.bind_addr = addr;
        self
    }

    pub fn enable_websocket(mut self, enabled: bool) -> Self {
        self.config.websocket.enabled = enabled;
        self
    }

    pub fn enable_quic(mut self, enabled: bool) -> Self {
        self.config.quic.enabled = enabled;
        self
    }

    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.connection_manager.max_connections = max;
        self
    }

    pub fn enable_auth(mut self, enabled: bool) -> Self {
        self.config.auth.enabled = enabled;
        self
    }
    
    pub fn log_level(mut self, level: String) -> Self {
        self.config.logging.level = level;
        self
    }
    
    pub fn build(self) -> FlareIMServer {
        let mut server = FlareIMServer::new(self.config);
        
        if let Some(handler) = self.auth_handler {
            server = server.with_auth_handler(handler);
        }
        
        if let Some(handler) = self.message_handler {
            server = server.with_message_handler(handler);
        }
        
        if let Some(handler) = self.event_handler {
            server = server.with_event_handler(handler);
        }
        
        server
    }
}

impl Default for FlareIMServerBuilder {
    fn default() -> Self {
        Self::new()
    }
} 