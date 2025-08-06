//! Flare IM QUIC服务器模块
//!
//! 提供QUIC协议支持的服务端实现

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use std::net::SocketAddr;
use quinn::{Endpoint, ServerConfig as QuinnServerConfig};

use crate::common::{
    conn::{Connection, ConnectionEvent, ProtoMessage, Platform, ConnectionConfig},
    Result, FlareError, TransportProtocol,
};

use super::{

    handlers::{},
    conn_manager::{MemoryServerConnectionManager, ServerConnectionManager},
    message_center::MessageProcessingCenter,
};
use super::config::QuicServerConfig;

/// QUIC服务器
pub struct QuicServer {
    config: QuicServerConfig,
    connection_manager: Arc<MemoryServerConnectionManager>,
    message_center: Arc<MessageProcessingCenter>,
    running: Arc<RwLock<bool>>,
}

impl QuicServer {
    pub fn new(
        config: QuicServerConfig,
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
    
    /// 启动QUIC服务器
    pub async fn start(&self) -> Result<()> {
        info!("启动QUIC服务器: {}", self.config.bind_addr);
        
        // 创建QUIC端点
        let endpoint = self.create_quic_endpoint().await?;
        
        info!("QUIC服务器已绑定到: {}", self.config.bind_addr);
        
        // 创建连接处理器
        let connection_handler = QuicConnectionHandler::new(
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
                    match endpoint.accept().await {
                        Some(connecting) => {
                            debug!("接受新的QUIC连接: {}", connecting.remote_address());
                            
                            // 为每个连接创建独立的任务
                            let handler = connection_handler.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handler.handle_new_connection(connecting).await {
                                    error!("处理QUIC连接失败: {}", e);
                                }
                            });
                        }
                        None => {
                            // 端点已关闭
                            break;
                        }
                    }
                }
            })
        };
        
        // 等待服务器停止
        tokio::select! {
            _ = server_task => {
                info!("QUIC服务器任务已结束");
            }
            _ = cleanup_task => {
                info!("QUIC清理任务已结束");
            }
        }
        
        info!("QUIC服务器已停止");
        Ok(())
    }
    
    /// 创建QUIC端点
    async fn create_quic_endpoint(&self) -> Result<Endpoint> {
        // 加载TLS证书和私钥
        let server_config = self.load_tls_config().await?;
        
        // 解析绑定地址
        let bind_addr = self.config.bind_addr.parse::<std::net::SocketAddr>()
            .map_err(|e| FlareError::InvalidConfiguration(format!("无效的绑定地址 {}: {}", self.config.bind_addr, e)))?;
        
        // 创建QUIC端点
        let endpoint = Endpoint::server(server_config, bind_addr)
            .map_err(|e| FlareError::NetworkError(std::io::Error::new(std::io::ErrorKind::Other, format!("创建QUIC端点失败: {}", e))))?;
        
        Ok(endpoint)
    }
    
    /// 加载TLS配置
    async fn load_tls_config(&self) -> Result<QuinnServerConfig> {
        // 使用统一的TLS处理，支持自定义证书路径
        let cert_path = &self.config.cert_path;
        let key_path = &self.config.key_path;
        
        let server_config = crate::common::tls::create_server_config(cert_path, key_path)
            .map_err(|e| FlareError::InvalidConfiguration(format!("创建服务端TLS配置失败: {}", e)))?;
        
        // 应用传输层配置
        let mut config = server_config;
        let transport_config = crate::common::tls::create_transport_config();
        config.transport_config(Arc::new(transport_config));
        
        Ok(config)
    }
}

/// QUIC连接处理器
/// 
/// 负责处理单个QUIC连接的生命周期，包括：
/// - 连接建立和认证
/// - 流处理和消息处理
/// - 连接关闭和清理
#[derive(Clone)]
pub struct QuicConnectionHandler {
    connection_manager: Arc<MemoryServerConnectionManager>,
    message_center: Arc<MessageProcessingCenter>,
}

impl QuicConnectionHandler {
    pub fn new(
        connection_manager: Arc<MemoryServerConnectionManager>,
        message_center: Arc<MessageProcessingCenter>,
    ) -> Self {
        Self {
            connection_manager,
            message_center,
        }
    }
    
    /// 处理新的QUIC连接
    /// 
    /// 这是连接处理的主要入口点，负责：
    /// 1. 接受QUIC连接
    /// 2. 创建连接实例
    /// 3. 用户认证
    /// 4. 添加到连接管理器
    /// 5. 启动流处理循环
    pub async fn handle_new_connection(
        &self,
        incoming: quinn::Incoming,
    ) -> Result<()> {
        debug!("处理新的QUIC连接: {}", incoming.remote_address());
        
        // 接受连接
        let connection = incoming.await
            .map_err(|e| FlareError::ConnectionFailed(format!("QUIC连接失败: {}", e)))?;
        
        let remote_addr = connection.remote_address();
        debug!("QUIC连接已建立: {}", remote_addr);
        
        // 创建连接配置
        let connection_id = uuid::Uuid::new_v4().to_string();
        let config = ConnectionConfig {
            id: connection_id.clone(),
            remote_addr: remote_addr.to_string(),
            platform: Platform::Unknown, // QUIC连接的平台信息需要从客户端获取
            protocol: TransportProtocol::QUIC,
            timeout_ms: 300_000, // 5分钟
            heartbeat_interval_ms: 30_000, // 30秒
            max_reconnect_attempts: 0, // 服务端不需要重连
            reconnect_delay_ms: 0,
        };
        
        // 等待双向流，添加超时处理
        let (send_stream, recv_stream) = tokio::time::timeout(
            Duration::from_secs(10),
            connection.accept_bi()
        ).await
            .map_err(|_| FlareError::ConnectionFailed("等待QUIC双向流超时".to_string()))?
            .map_err(|e| FlareError::ConnectionFailed(format!("接受QUIC流失败: {}", e)))?;
        
        // 创建QUIC连接
        let mut quic_connection = crate::common::conn::quic::QuicConnectionFactory::from_quinn_connection(
            config,
            connection,
            send_stream,
            recv_stream,
        ).await;
        
        // 启动接收任务
        quic_connection.start_receive_task().await?;
        
        // 等待认证
        let user_id = self.wait_for_authentication(&mut quic_connection).await?;
        let session_id = quic_connection.get_connection_id().to_string();
        
        // 添加到连接管理器
        self.connection_manager.add_connection(
            quic_connection.clone_box(),
            user_id.clone(),
            session_id.clone(),
        ).await?;
        
        // 处理连接建立
        self.handle_connection_established(&user_id, &session_id).await?;
        
        // 启动消息处理循环
        self.handle_connection_messages(quic_connection, user_id, session_id).await?;
        
        Ok(())
    }
    
    /// 等待客户端认证
    /// 
    /// 在连接建立后等待客户端发送认证消息，超时时间为30秒
    async fn wait_for_authentication(
        &self,
        connection: &mut crate::common::conn::quic::QuicConnection,
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
        connection: crate::common::conn::quic::QuicConnection,
        user_id: String,
        session_id: String,
    ) -> Result<()> {
        debug!("开始处理连接消息: 用户 {} 会话 {}", user_id, session_id);
        
        loop {
            match connection.receive().await {
                Ok(message) => {
                    // 处理消息
                    if let Err(e) = self.handle_quic_message(&message, &user_id).await {
                        error!("处理QUIC消息失败: {}", e);
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
    
    /// 处理QUIC消息
    /// 
    /// 根据消息类型分发到不同的处理逻辑
    async fn handle_quic_message(&self, message: &ProtoMessage, user_id: &str) -> Result<()> {
        debug!("处理QUIC消息: 用户 {} 类型 {}", user_id, message.message_type);
        
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
            handler.handle_user_connect(user_id, session_id, Platform::Unknown).await?;
        }
        
        info!("QUIC连接已建立: 用户 {} 会话 {}", user_id, session_id);
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
        
        info!("QUIC连接已关闭: 用户 {} 会话 {}", user_id, session_id);
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
        
        debug!("QUIC心跳: 用户 {} 会话 {}", user_id, session_id);
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