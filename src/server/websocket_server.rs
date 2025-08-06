//! Flare IM WebSocket服务器模块
//!
//! 提供WebSocket协议支持的服务端实现

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

use crate::common::{
    conn::{Connection, ConnectionEvent, ProtoMessage, Platform, ConnectionConfig},
    Result, FlareError, TransportProtocol,
};

use super::{

    handlers::{},
    conn_manager::{MemoryServerConnectionManager, ServerConnectionManager},
    message_center::MessageProcessingCenter,
};
use super::config::WebSocketServerConfig;

/// WebSocket服务器
pub struct WebSocketServer {
    config: WebSocketServerConfig,
    connection_manager: Arc<MemoryServerConnectionManager>,
    message_center: Arc<MessageProcessingCenter>,
    running: Arc<RwLock<bool>>,
}

impl WebSocketServer {
    pub fn new(
        config: WebSocketServerConfig,
        connection_manager: Arc<MemoryServerConnectionManager>,
        message_center: Arc<MessageProcessingCenter>,
        running: Arc<RwLock<bool>>,
    ) -> Self {
        Self {
            config,
            connection_manager,
            message_center,
            running,
        }
    }
    
    /// 启动WebSocket服务器
    pub async fn start(&self) -> Result<()> {
        info!("启动WebSocket服务器: {}", self.config.bind_addr);
        
        // 解析绑定地址
        let bind_addr = self.config.bind_addr.parse::<std::net::SocketAddr>()
            .map_err(|e| FlareError::InvalidConfiguration(format!("无效的绑定地址 {}: {}", self.config.bind_addr, e)))?;
        
        // 创建TCP监听器
        let listener = TcpListener::bind(bind_addr).await
            .map_err(|e| FlareError::NetworkError(std::io::Error::new(std::io::ErrorKind::AddrInUse, format!("无法绑定地址 {}: {}", self.config.bind_addr, e))))?;
        
        info!("WebSocket服务器已绑定到: {}", self.config.bind_addr);
        
        // 创建连接处理器
        let connection_handler = WebSocketConnectionHandler::new(
            self.connection_manager.clone(),
            self.message_center.clone(),
        );
        
        // 启动清理任务
        let cleanup_interval = Duration::from_secs(60); // 每分钟清理一次
        let cleanup_task = {
            let connection_manager = self.connection_manager.clone();
            let running = self.running.clone();
            tokio::spawn(async move {
                let mut interval = interval(cleanup_interval);
                while *running.read().await {
                    interval.tick().await;
                    if let Err(e) = connection_manager.cleanup_expired_connections(300).await {
                        warn!("清理过期连接失败: {}", e);
                    }
                }
            })
        };
        
        // 主服务器循环
        let server_task = {
            let connection_handler = connection_handler.clone();
            let running = self.running.clone();
            tokio::spawn(async move {
                while *running.read().await {
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            debug!("接受新的TCP连接: {}", addr);
                            
                            // 为每个连接创建独立的任务
                            let handler = connection_handler.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handler.handle_new_connection(stream, addr).await {
                                    error!("处理WebSocket连接失败: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("接受TCP连接失败: {}", e);
                        }
                    }
                }
            })
        };
        
        // 等待服务器停止
        tokio::select! {
            _ = server_task => {
                info!("WebSocket服务器任务已结束");
            }
            _ = cleanup_task => {
                info!("WebSocket清理任务已结束");
            }
        }
        
        info!("WebSocket服务器已停止");
        Ok(())
    }
}

/// WebSocket连接处理器
/// 
/// 负责处理单个WebSocket连接的生命周期，包括：
/// - 连接建立和认证
/// - 消息处理
/// - 连接关闭和清理
#[derive(Clone)]
pub struct WebSocketConnectionHandler {
    connection_manager: Arc<MemoryServerConnectionManager>,
    message_center: Arc<MessageProcessingCenter>,
}

impl WebSocketConnectionHandler {
    pub fn new(
        connection_manager: Arc<MemoryServerConnectionManager>,
        message_center: Arc<MessageProcessingCenter>,
    ) -> Self {
        Self {
            connection_manager,
            message_center,
        }
    }
    
    /// 处理新的WebSocket连接
    /// 
    /// 这是连接处理的主要入口点，负责：
    /// 1. WebSocket握手
    /// 2. 创建连接实例
    /// 3. 用户认证
    /// 4. 添加到连接管理器
    /// 5. 启动消息处理循环
    pub async fn handle_new_connection(
        &self,
        stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
    ) -> Result<()> {
        debug!("处理新的WebSocket连接: {}", addr);
        
        // 设置TCP流为非阻塞模式
        stream.set_nodelay(true).map_err(|e| {
            error!("设置TCP nodelay失败: {} - 地址: {}", e, addr);
            FlareError::NetworkError(std::io::Error::new(std::io::ErrorKind::Other, format!("设置TCP选项失败: {}", e)))
        })?;
        
        // 注意：tokio::net::TcpStream 不支持 set_keepalive
        // 如果需要 keepalive，需要使用 socket2 库
        
        info!("开始WebSocket握手: {}", addr);
        
        // 执行WebSocket握手
        let ws_stream = accept_async(stream).await
            .map_err(|e| {
                error!("WebSocket握手失败: {} - 地址: {}", e, addr);
                // 添加更详细的错误信息
                match &e {
                    tokio_tungstenite::tungstenite::Error::Http(http_error) => {
                        error!("HTTP错误: {:?}", http_error);
                    }
                    tokio_tungstenite::tungstenite::Error::Protocol(protocol_error) => {
                        error!("协议错误: {:?}", protocol_error);
                    }
                    tokio_tungstenite::tungstenite::Error::Io(io_error) => {
                        error!("IO错误: {:?}", io_error);
                    }
                    _ => {
                        error!("其他错误: {:?}", e);
                    }
                }
                FlareError::ProtocolError(format!("WebSocket握手失败: {}", e))
            })?;
        
        debug!("WebSocket握手成功: {}", addr);
        
        // 创建连接配置
        let connection_id = uuid::Uuid::new_v4().to_string();
        let config = ConnectionConfig {
            id: connection_id.clone(),
            remote_addr: addr.to_string(),
            platform: Platform::Web,
            protocol: TransportProtocol::WebSocket,
            timeout_ms: 300_000, // 5分钟
            heartbeat_interval_ms: 30_000, // 30秒
            max_reconnect_attempts: 0, // 服务端不需要重连
            reconnect_delay_ms: 0,
        };
        
        // 创建WebSocket连接
        let ws_connection = crate::common::conn::websocket::WebSocketConnectionFactory::from_tungstenite_stream_plain(
            config,
            ws_stream,
        );
        
        // 启动接收任务
        ws_connection.start_receive_task().await?;
        
        // 等待认证
        // let user_id = self.wait_for_authentication(&mut ws_connection).await?;
        let user_id = "test_user".to_string(); // 暂时写死
        let session_id = ws_connection.get_session_id().to_string();
        
        // 添加到连接管理器
        info!("开始将连接添加到连接管理器: 用户={}, 会话={}", user_id, session_id);
        match self.connection_manager.add_connection(
            ws_connection.clone_box(),
            user_id.clone(),
            session_id.clone(),
        ).await {
            Ok(_) => {
                info!("连接成功添加到连接管理器: 用户={}, 会话={}", user_id, session_id);
            }
            Err(e) => {
                error!("添加连接到连接管理器失败: 用户={}, 会话={}, 错误={}", user_id, session_id, e);
                return Err(e);
            }
        }
        
        // 处理连接建立
        self.handle_connection_established(&user_id, &session_id).await?;
        
        // 启动消息处理循环
        self.handle_connection_messages(ws_connection, user_id, session_id).await?;
        
        Ok(())
    }
    
    /// 等待客户端认证
    /// 
    /// 在连接建立后等待客户端发送认证消息，超时时间为30秒
    async fn wait_for_authentication(
        &self,
        connection: &mut crate::common::conn::websocket::WebSocketConnection,
    ) -> Result<String> {
        let timeout = Duration::from_secs(30); // 30秒认证超时
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            // 尝试接收认证消息
            match connection.receive().await {
                Ok(message) => {
                    // 解析认证消息
                    if let Some(user_id) = self.parse_auth_message(&message).await? {
                        info!("用户认证成功: {}", user_id);
                        return Ok(user_id);
                    }
                }
                Err(e) => {
                    debug!("接收认证消息失败: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
        
        Err(FlareError::AuthenticationError("认证超时".to_string()))
    }
    
    /// 解析认证消息
    /// 
    /// 解析客户端发送的认证消息，提取token并验证
    async fn parse_auth_message(&self, message: &ProtoMessage) -> Result<Option<String>> {
        if message.message_type == "auth" {
            // 解析token
            let token = String::from_utf8_lossy(&message.payload);
            let token = token.trim_matches('"'); // 移除JSON引号
            
            // 验证token
            if let Some(user_id) = self.validate_user_token(token).await? {
                return Ok(Some(user_id));
            }
        }
        
        Ok(None)
    }
    
    /// 处理连接消息循环
    /// 
    /// 这是连接的主要消息处理循环，负责：
    /// 1. 接收客户端消息
    /// 2. 处理不同类型的消息
    /// 3. 在连接断开时进行清理
    async fn handle_connection_messages(
        &self,
        connection: crate::common::conn::websocket::WebSocketConnection,
        user_id: String,
        session_id: String,
    ) -> Result<()> {
        debug!("开始处理连接消息: 用户 {} 会话 {}", user_id, session_id);
        
        loop {
            match connection.receive().await {
                Ok(message) => {
                    // 处理消息
                    if let Err(e) = self.handle_websocket_message(&message, &user_id).await {
                        error!("处理WebSocket消息失败: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    debug!("接收消息失败: {}", e);
                    break;
                }
            }
        }
        
        // 处理连接关闭
        self.handle_connection_closed(&user_id, &session_id).await?;
        
        // 从连接管理器移除
        self.connection_manager.remove_connection(&user_id, &session_id).await?;
        
        Ok(())
    }
    
    /// 处理WebSocket消息
    /// 
    /// 根据消息类型分发到不同的处理逻辑
    async fn handle_websocket_message(&self, message: &ProtoMessage, user_id: &str) -> Result<()> {
        debug!("处理WebSocket消息: 用户 {} 类型 {}", user_id, message.message_type);
        
        match message.message_type.as_str() {
            "message" => {
                // 处理普通消息
                let response = self.handle_message_received(user_id, message.clone()).await?;
                
                // 发送响应给用户的所有连接
                let _ = self.connection_manager.send_message_to_user(user_id, response).await;
            }
            "heartbeat" => {
                // 处理心跳
                self.handle_heartbeat(user_id, "").await?;
            }
            "ping" => {
                // 处理ping - 发送pong响应
                let pong = ProtoMessage::new(
                    uuid::Uuid::new_v4().to_string(),
                    "pong".to_string(),
                    message.payload.clone(),
                );
                let _ = self.connection_manager.send_message_to_user(user_id, pong).await;
            }
            _ => {
                warn!("未知消息类型: {}", message.message_type);
            }
        }
        
        Ok(())
    }
    
    /// 处理连接建立
    /// 
    /// 在连接成功建立并认证后调用，触发相关事件
    pub async fn handle_connection_established(&self, user_id: &str, session_id: &str) -> Result<()> {
        // 获取处理器
        let event_handler = self.message_center.get_event_handler();
        let message_handler = self.message_center.get_message_handler();
        
        // 触发连接事件
        if let Some(handler) = &event_handler {
            handler.handle_connection_event(user_id, ConnectionEvent::Connected).await?;
        }
        
        // 调用消息处理器
        if let Some(handler) = &message_handler {
            handler.handle_user_connect(user_id, session_id, Platform::Web).await?;
        }
        
        info!("WebSocket连接已建立: 用户 {} 会话 {}", user_id, session_id);
        Ok(())
    }
    
    /// 处理消息接收
    /// 
    /// 处理接收到的消息，调用消息处理器并触发相关事件
    pub async fn handle_message_received(&self, user_id: &str, message: ProtoMessage) -> Result<ProtoMessage> {
        // 使用消息处理中心统一处理
        self.message_center.process_message(user_id, "", message).await
    }
    
    /// 处理连接关闭
    /// 
    /// 在连接断开时调用，进行清理工作并触发相关事件
    pub async fn handle_connection_closed(&self, user_id: &str, session_id: &str) -> Result<()> {
        // 获取处理器
        let message_handler = self.message_center.get_message_handler();
        let event_handler = self.message_center.get_event_handler();
        
        // 调用消息处理器
        if let Some(handler) = &message_handler {
            handler.handle_user_disconnect(user_id, session_id).await?;
        }
        
        // 触发断开事件
        if let Some(handler) = &event_handler {
            handler.handle_connection_event(user_id, ConnectionEvent::Disconnected).await?;
        }
        
        info!("WebSocket连接已关闭: 用户 {} 会话 {}", user_id, session_id);
        Ok(())
    }
    
    /// 处理心跳
    /// 
    /// 处理客户端发送的心跳消息
    pub async fn handle_heartbeat(&self, user_id: &str, session_id: &str) -> Result<()> {
        // 获取处理器
        let message_handler = self.message_center.get_message_handler();
        let event_handler = self.message_center.get_event_handler();
        
        // 调用消息处理器
        if let Some(handler) = &message_handler {
            handler.handle_heartbeat(user_id, session_id).await?;
        }
        
        // 触发心跳事件
        if let Some(handler) = &event_handler {
            handler.handle_connection_event(user_id, ConnectionEvent::Heartbeat).await?;
        }
        
        debug!("WebSocket心跳: 用户 {} 会话 {}", user_id, session_id);
        Ok(())
    }
    
    /// 验证用户令牌
    /// 
    /// 验证客户端提供的认证令牌
    pub async fn validate_user_token(&self, token: &str) -> Result<Option<String>> {
        if let Some(handler) = self.message_center.get_auth_handler() {
            handler.validate_token(token).await
        } else {
            Ok(Some("anonymous".to_string())) // 如果没有认证处理器，默认允许匿名用户
        }
    }
} 