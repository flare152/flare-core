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
    conn::{Connection, ConnectionConfig},
    Result, FlareError, TransportProtocol,
    types::Platform,
    MessageParser,
};

use super::{
    conn_manager::{MemoryServerConnectionManager, ServerConnectionManager},
};
use super::config::QuicServerConfig;

/// QUIC服务器
pub struct QuicServer {
    config: QuicServerConfig,
    connection_manager: Arc<MemoryServerConnectionManager>,
    message_parser: Arc<MessageParser>,
    running: Arc<RwLock<bool>>,
    server_task: Option<tokio::task::JoinHandle<()>>,
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

impl QuicServer {
    pub fn new(
        config: QuicServerConfig,
        connection_manager: Arc<MemoryServerConnectionManager>,
        message_parser: Arc<MessageParser>,
        running: Arc<RwLock<bool>>,
    ) -> Self {
        Self {
            config,
            connection_manager,
            message_parser,
            running,
            server_task: None,
            cleanup_task: None,
        }
    }
    
    /// 启动QUIC服务器
    pub async fn start(&mut self) -> Result<()> {
        info!("启动QUIC服务器: {}", self.config.bind_addr);
        
        // 创建QUIC端点
        let endpoint = self.create_quic_endpoint().await?;
        
        info!("QUIC服务器已绑定到: {}", self.config.bind_addr);
        
        // 创建连接处理器
        let connection_handler = QuicConnectionHandler::new(
            self.connection_manager.clone(),
            self.message_parser.clone(),
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
        
        // 保存任务句柄
        self.server_task = Some(server_task);
        self.cleanup_task = Some(cleanup_task);
        
        info!("QUIC服务器已启动");
        Ok(())
    }
    
    /// 停止QUIC服务器
    pub async fn stop(&mut self) -> Result<()> {
        info!("停止QUIC服务器");
        
        // 设置停止标志
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        
        // 等待任务完成
        if let Some(task) = self.server_task.take() {
            if let Err(e) = task.await {
                warn!("QUIC服务器任务异常结束: {}", e);
            }
        }
        
        if let Some(task) = self.cleanup_task.take() {
            if let Err(e) = task.await {
                warn!("QUIC清理任务异常结束: {}", e);
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
            .map_err(|e| FlareError::NetworkError(format!("创建QUIC端点失败: {}", e)))?;
        
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
    message_parser: Arc<MessageParser>,
}

impl QuicConnectionHandler {
    pub fn new(
        connection_manager: Arc<MemoryServerConnectionManager>,
        message_parser: Arc<MessageParser>,
    ) -> Self {
        Self {
            connection_manager,
            message_parser,
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
        
        // 使用通道来接收认证消息
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1);
        
        // 启动接收任务
        let tx_clone = tx.clone();
        connection.start_receive_task(Box::new(move |msg| {
            let tx = tx_clone.clone();
            tokio::spawn(async move {
                // 将UnifiedProtocolMessage转换为Vec<u8>
                let msg_data = serde_json::to_vec(&msg).unwrap_or_default();
                let _ = tx.send(msg_data).await;
            });
        })).await?;
        
        while start.elapsed() < timeout {
            // 尝试接收认证消息
            match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
                Ok(Some(message_data)) => {
                    // 解析认证消息
                    if let Some(user_id) = self.parse_auth_message(&message_data).await? {
                        info!("用户认证成功: {}", user_id);
                        return Ok(user_id);
                    }
                }
                Ok(None) => {
                    debug!("接收通道已关闭");
                    break;
                }
                Err(_) => {
                    // 超时，继续循环
                    continue;
                }
            }
        }
        
        Err(FlareError::AuthenticationError("认证超时".to_string()))
    }
    
    /// 解析认证消息
    /// 
    /// 解析客户端发送的认证消息，提取token并验证
    async fn parse_auth_message(&self, message_data: &[u8]) -> Result<Option<String>> {
        // 使用消息解析器处理认证消息
        match self.message_parser.parse_message(message_data).await {
            Ok(message) => {
                // 检查是否是认证消息
                if message.is_connect() {
                    if let Some((user_id, _)) = message.as_connect_info() {
                        return Ok(Some(user_id));
                    }
                }
                Ok(None)
            }
            Err(_) => {
                // 解析失败，尝试其他方式
                if let Ok(token) = String::from_utf8(message_data.to_vec()) {
                    // 简单的token验证
                    if token.starts_with("auth_") {
                        return Ok(Some(token[5..].to_string()));
                    }
                }
                Ok(None)
            }
        }
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
        
        // 使用通道来接收消息
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
        let user_id_clone = user_id.clone();
        let session_id_clone = session_id.clone();
        
        // 启动接收任务
        let tx_clone = tx.clone();
        connection.start_receive_task(Box::new(move |msg| {
            let tx = tx_clone.clone();
            tokio::spawn(async move {
                // 将UnifiedProtocolMessage转换为Vec<u8>
                let msg_data = serde_json::to_vec(&msg).unwrap_or_default();
                let _ = tx.send(msg_data).await;
            });
        })).await?;
        
        // 处理消息循环
        while let Some(message_data) = rx.recv().await {
            // 使用消息解析器处理消息
            if let Err(e) = self.message_parser.handle_message(session_id_clone.clone(), &message_data).await {
                error!("处理QUIC消息失败: {}", e);
                break;
            }
        }
        
        // 处理连接关闭
        self.handle_connection_closed(&user_id_clone, &session_id_clone).await?;
        
        // 从连接管理器移除
        self.connection_manager.remove_connection(&user_id_clone, &session_id_clone).await?;
        
        Ok(())
    }
    
    /// 处理连接建立
    /// 
    /// 在连接成功建立并认证后调用，触发相关事件
    pub async fn handle_connection_established(&self, user_id: &str, session_id: &str) -> Result<()> {
        info!("QUIC连接已建立: 用户 {} 会话 {}", user_id, session_id);
        Ok(())
    }
    
    /// 处理连接关闭
    /// 
    /// 在连接断开时调用，进行清理工作并触发相关事件
    pub async fn handle_connection_closed(&self, user_id: &str, session_id: &str) -> Result<()> {
        info!("QUIC连接已关闭: 用户 {} 会话 {}", user_id, session_id);
        Ok(())
    }
} 