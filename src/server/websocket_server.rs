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
    conn::{Connection, ConnectionConfig},
    Result, FlareError, TransportProtocol,
    types::Platform,
    MessageParser,
};

use super::{
    conn_manager::{MemoryServerConnectionManager, ServerConnectionManager},
};
use super::config::WebSocketServerConfig;

/// WebSocket服务器
pub struct WebSocketServer {
    config: WebSocketServerConfig,
    connection_manager: Arc<dyn ServerConnectionManager>,
    message_parser: Arc<MessageParser>,
    running: Arc<RwLock<bool>>,
    server_task: Option<tokio::task::JoinHandle<()>>,
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

impl WebSocketServer {
    pub fn new(
        config: WebSocketServerConfig,
        connection_manager: Arc<dyn ServerConnectionManager>,
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
    
    /// 启动WebSocket服务器
    pub async fn start(&mut self) -> Result<()> {
        info!("启动WebSocket服务器: {}", self.config.bind_addr);
        
        // 解析绑定地址
        let bind_addr = self.config.bind_addr.parse::<std::net::SocketAddr>()
            .map_err(|e| FlareError::InvalidConfiguration(format!("无效的绑定地址 {}: {}", self.config.bind_addr, e)))?;
        
        // 创建TCP监听器
        let listener = TcpListener::bind(bind_addr).await
            .map_err(|e| FlareError::NetworkError(format!("无法绑定地址 {}: {}", self.config.bind_addr, e)))?;
        
        info!("WebSocket服务器已绑定到: {}", self.config.bind_addr);
        
        // 创建连接处理器
        let connection_handler = WebSocketConnectionHandler::new(
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
        
        // 保存任务句柄
        self.server_task = Some(server_task);
        self.cleanup_task = Some(cleanup_task);
        
        info!("WebSocket服务器已启动");
        Ok(())
    }
    
    /// 停止WebSocket服务器
    pub async fn stop(&mut self) -> Result<()> {
        info!("停止WebSocket服务器");
        
        // 设置停止标志
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        
        // 等待任务完成
        if let Some(task) = self.server_task.take() {
            if let Err(e) = task.await {
                warn!("WebSocket服务器任务异常结束: {}", e);
            }
        }
        
        if let Some(task) = self.cleanup_task.take() {
            if let Err(e) = task.await {
                warn!("WebSocket清理任务异常结束: {}", e);
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
    connection_manager: Arc<dyn ServerConnectionManager>,
    message_parser: Arc<MessageParser>,
}

impl WebSocketConnectionHandler {
    pub fn new(
        connection_manager: Arc<dyn ServerConnectionManager>,
        message_parser: Arc<MessageParser>,
    ) -> Self {
        Self {
            connection_manager,
            message_parser,
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
            FlareError::NetworkError(format!("设置TCP选项失败: {}", e))
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
            max_reconnect_attempts: 0, // 服务端不需要重连
            reconnect_delay_ms: 0,
        };
        
        // 创建WebSocket连接
        let ws_connection = crate::common::conn::websocket::WebSocketConnectionFactory::from_tungstenite_stream_plain(
            config,
            ws_stream,
            Arc::clone(&self.message_parser),
        );
        
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
        connection: crate::common::conn::websocket::WebSocketConnection,
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
                error!("处理WebSocket消息失败: {}", e);
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
        info!("WebSocket连接已建立: 用户 {} 会话 {}", user_id, session_id);
        Ok(())
    }
    
    /// 处理连接关闭
    /// 
    /// 在连接断开时调用，进行清理工作并触发相关事件
    pub async fn handle_connection_closed(&self, user_id: &str, session_id: &str) -> Result<()> {
        info!("WebSocket连接已关闭: 用户 {} 会话 {}", user_id, session_id);
        Ok(())
    }
} 