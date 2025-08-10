//! Flare IM WebSocket 连接实现
//!
//! 基于WebSocket协议的连接实现，支持普通TCP和TLS两种模式

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tokio::time::Duration;
use futures_util::{SinkExt, StreamExt};
use tracing::{info, error, debug};
use async_trait::async_trait;

use crate::common::{
    conn::{Connection, ConnectionConfig, ConnectionStats}, 
    error::{Result, FlareError},
    MessageParser
};
use crate::common::types::{Platform, ConnectionStatus, TransportProtocol};
use crate::common::protocol::UnifiedProtocolMessage;

/// WebSocket连接实现
/// 
/// 提供基于WebSocket协议的连接功能，包括：
/// - 消息发送和接收
/// - 连接状态管理
/// - 元数据管理
/// - 异步任务管理
pub struct WebSocketConnection {
    /// 连接配置
    config: ConnectionConfig,
    /// 连接状态
    state: Arc<RwLock<ConnectionStatus>>,
    /// 连接统计信息
    stats: Arc<RwLock<ConnectionStats>>,
    /// 最后活动时间
    last_activity: Arc<RwLock<Instant>>,
    /// 会话ID
    session_id: String,
    /// WebSocket流 - 使用枚举支持两种类型
    ws_stream: Arc<Mutex<Option<WebSocketStreamEnum>>>,
    /// 接收任务句柄
    receive_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
    /// 消息解析器
    message_parser: Arc<MessageParser>,
}

/// WebSocket流枚举 - 支持两种类型
#[derive(Debug)]
enum WebSocketStreamEnum {
    MaybeTls(tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>),
    Plain(tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>),
}



impl WebSocketConnection {
    /// 创建新的WebSocket连接
    pub fn new(config: ConnectionConfig, message_parser: Arc<MessageParser>) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        
        debug!("创建新的WebSocket连接: {} - {}", config.id, session_id);
        
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            session_id,
            ws_stream: Arc::new(Mutex::new(None)),
            receive_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
            message_parser,
        }
    }
    
    /// 从现有的tokio-tungstenite连接创建WebSocket连接（MaybeTlsStream）
    pub fn from_tungstenite_stream(
        config: ConnectionConfig,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        message_parser: Arc<MessageParser>,
    ) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        
        debug!("从TLS WebSocket流创建连接: {} - {}", config.id, session_id);
        
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionStatus::Connected)),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            session_id,
            ws_stream: Arc::new(Mutex::new(Some(WebSocketStreamEnum::MaybeTls(ws_stream)))),
            receive_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
            message_parser,
        }
    }
    
    /// 从现有的tokio-tungstenite连接创建WebSocket连接（纯TcpStream）
    pub fn from_tungstenite_stream_plain(
        config: ConnectionConfig,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        message_parser: Arc<MessageParser>,
    ) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        
        debug!("从普通WebSocket流创建连接: {} - {}", config.id, session_id);
        
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionStatus::Connected)),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            session_id,
            ws_stream: Arc::new(Mutex::new(Some(WebSocketStreamEnum::Plain(ws_stream)))),
            receive_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
            message_parser,
        }
    }
    
    /// 设置WebSocket流
    pub async fn set_ws_stream(
        &mut self,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    ) {
        debug!("设置TLS WebSocket流: {}", self.session_id);
        
        {
            let mut stream_guard = self.ws_stream.lock().await;
            *stream_guard = Some(WebSocketStreamEnum::MaybeTls(ws_stream));
        }
        
        // 更新连接状态
        {
            let mut state = self.state.write().await;
            *state = ConnectionStatus::Connected;
        }
        
        info!("TLS WebSocket流已设置: {}", self.session_id);
    }
    
    /// 设置普通WebSocket流
    pub async fn set_ws_stream_plain(
        &mut self,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    ) {
        debug!("设置普通WebSocket流: {}", self.session_id);
        
        {
            let mut stream_guard = self.ws_stream.lock().await;
            *stream_guard = Some(WebSocketStreamEnum::Plain(ws_stream));
        }
        
        // 更新连接状态
        {
            let mut state = self.state.write().await;
            *state = ConnectionStatus::Connected;
        }
        
        info!("普通WebSocket流已设置: {}", self.session_id);
    }
    
    /// 解析消息
    fn parse_message(data: &[u8]) -> Result<UnifiedProtocolMessage> {
        serde_json::from_slice(data)
            .map_err(|e| FlareError::protocol_error(format!("解析WebSocket消息失败: {}", e)))
    }
    
    /// 序列化消息
    fn serialize_message(message: &UnifiedProtocolMessage) -> Result<Vec<u8>> {
        serde_json::to_vec(message)
            .map_err(|e| FlareError::protocol_error(format!("序列化WebSocket消息失败: {}", e)))
    }
    
    /// 获取会话ID
    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }
    
    /// 获取连接ID
    pub fn get_connection_id(&self) -> &str {
        &self.config.id
    }
    
    /// 获取连接状态
    pub async fn get_status(&self) -> ConnectionStatus {
        let state = self.state.read().await;
        *state
    }
    
    /// 设置消息解析器
    pub async fn set_message_parser(&mut self, parser: Arc<MessageParser>) {
        self.message_parser = parser;
    }
}

impl std::fmt::Debug for WebSocketConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketConnection")
            .field("config", &self.config)
            .field("session_id", &self.session_id)
            .field("state", &"<Arc<RwLock<ConnectionStatus>>>")
            .field("stats", &"<Arc<RwLock<ConnectionStats>>>")
            .field("last_activity", &"<Arc<RwLock<Instant>>>")
            .field("ws_stream", &"<Arc<Mutex<Option<WebSocketStreamEnum>>>>")
            .field("receive_task", &"<Arc<Mutex<Option<JoinHandle<()>>>>>")
            .field("running", &"<Arc<RwLock<bool>>>")
            .field("message_parser", &"<Arc<MessageParser>>")
            .finish()
    }
}

#[async_trait]
impl Connection for WebSocketConnection {
    fn id(&self) -> &str {
        &self.session_id
    }
    
    fn remote_addr(&self) -> &str {
        &self.config.remote_addr
    }
    
    fn platform(&self) -> Platform {
        Platform::WebSocket
    }
    
    fn protocol(&self) -> TransportProtocol {
        TransportProtocol::WebSocket
    }
    
    async fn is_active(&self, _timeout: Duration) -> bool {
        let state = self.state.read().await;
        *state == ConnectionStatus::Connected
    }
    
    fn send(&self, msg: UnifiedProtocolMessage) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let ws_stream = Arc::clone(&self.ws_stream);
        let stats = Arc::clone(&self.stats);
        let last_activity = Arc::clone(&self.last_activity);
        Box::pin(async move {
            let mut stream_guard = ws_stream.lock().await;
            if let Some(ref mut stream_enum) = &mut *stream_guard {
                let data = Self::serialize_message(&msg)?;
                let data_len = data.len();
                let ws_msg = tungstenite::Message::Binary(data.into());
                
                match stream_enum {
                    WebSocketStreamEnum::MaybeTls(ws_stream) => {
                        ws_stream.send(ws_msg).await
                            .map_err(|e| FlareError::message_send_failed(format!("发送消息失败: {}", e)))?;
                        
                        // 更新最后活跃时间和统计信息
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.last_activity = chrono::Utc::now();
                            stats_guard.bytes_sent += data_len as u64;
                            stats_guard.messages_sent += 1;
                        }
                        
                        {
                            let mut activity_guard = last_activity.write().await;
                            *activity_guard = Instant::now();
                        }
                        
                        debug!("WebSocket消息已发送: {:?}", msg);
                        Ok(())
                    }
                    WebSocketStreamEnum::Plain(ws_stream) => {
                        ws_stream.send(ws_msg).await
                            .map_err(|e| FlareError::message_send_failed(format!("发送消息失败: {}", e)))?;
                        
                        // 更新最后活跃时间和统计信息
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.last_activity = chrono::Utc::now();
                            stats_guard.bytes_sent += data_len as u64;
                            stats_guard.messages_sent += 1;
                        }
                        
                        {
                            let mut activity_guard = last_activity.write().await;
                            *activity_guard = Instant::now();
                        }
                        
                        debug!("WebSocket消息已发送: {:?}", msg);
                        Ok(())
                    }
                }
            } else {
                Err(FlareError::connection_failed("WebSocket流未设置"))
            }
        })
    }
    
    fn start_receive_task(&self, callback: Box<dyn Fn(UnifiedProtocolMessage) + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let ws_stream = Arc::clone(&self.ws_stream);
        let running = Arc::clone(&self.running);
        let stats = Arc::clone(&self.stats);
        let last_activity = Arc::clone(&self.last_activity);
        let message_parser = Arc::clone(&self.message_parser);
        let session_id = self.session_id.clone();
        
        Box::pin(async move {
            let mut running_guard = running.write().await;
            if *running_guard {
                debug!("WebSocket接收任务已在运行: {}", session_id);
                return Ok(()); // 任务已经在运行
            }
            *running_guard = true;
            drop(running_guard);
            
            let session_id_clone = session_id.clone();
            let task = tokio::spawn(async move {
                debug!("WebSocket接收任务开始运行: {}", session_id_clone);
                while *running.read().await {
                    let mut stream_guard = ws_stream.lock().await;
                    if let Some(stream_enum) = &mut *stream_guard {
                        let msg_result = match stream_enum {
                            WebSocketStreamEnum::MaybeTls(ws_stream) => ws_stream.next().await,
                            WebSocketStreamEnum::Plain(ws_stream) => ws_stream.next().await,
                        };
                        
                        match msg_result {
                            Some(Ok(msg)) => {
                                match msg {
                                   tungstenite::Message::Binary(data) => {
                                        // 更新最后活跃时间和统计信息
                                        {
                                            let data_len = data.len();
                                            let mut stats_guard = stats.write().await;
                                            stats_guard.last_activity = chrono::Utc::now();
                                            stats_guard.bytes_received += data_len as u64;
                                            stats_guard.messages_received += 1;
                                        }
                                        
                                        {
                                            let mut activity_guard = last_activity.write().await;
                                            *activity_guard = Instant::now();
                                        }
                                        
                                        // 使用消息解析器处理消息
                                        match message_parser.handle_message(session_id.clone(), &data).await {
                                            Ok(()) => {
                                                debug!("WebSocket消息已通过解析器处理");
                                            }
                                            Err(e) => {
                                                error!("WebSocket消息解析失败: {}", e);
                                                // 如果解析失败，创建自定义消息
                                                let custom_msg = UnifiedProtocolMessage::custom_message(
                                                    "binary_data".to_string(),
                                                    data.to_vec()
                                                );
                                                debug!("WebSocket接收到自定义二进制消息");
                                                callback(custom_msg);
                                            }
                                        }
                                    }
                                    tungstenite::Message::Close(_) => {
                                        debug!("WebSocket接收流结束: {}", session_id);
                                        break;
                                    }
                                    _ => {
                                        // 忽略其他类型的消息
                                        continue;
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                error!("读取WebSocket流失败: {} - {}", session_id, e);
                                break;
                            }
                            None => {
                                debug!("WebSocket接收流结束: {}", session_id);
                                break;
                            }
                        }
                    } else {
                        error!("WebSocket接收流未设置: {}", session_id);
                        break;
                    }
                }
                debug!("WebSocket接收任务结束: {}", session_id);
            });
            {
                let mut task_guard = self.receive_task.lock().await;
                *task_guard = Some(task);
            }
            Ok(())
        })
    }
    
    fn close(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let _ws_stream = Arc::clone(&self.ws_stream);
        let running = Arc::clone(&self.running);
        let receive_task = Arc::clone(&self.receive_task);
        let state = Arc::clone(&self.state);
        let config_id = self.config.id.clone();
        let session_id = self.session_id.clone();
        
        Box::pin(async move {
            debug!("开始关闭WebSocket连接: {} - {}", config_id, session_id);
            
            // 发送连接关闭事件
            let unified_msg = UnifiedProtocolMessage::disconnect(
                config_id.clone(),
                session_id.clone(),
                "Connection closed by client".to_string()
            );
            
            // 尝试发送连接关闭事件
            let res = self.send(unified_msg).await;
            if res.is_err() {
                error!("发送连接关闭事件失败: {} - {}", config_id, session_id);
            }
            
            // 更新状态
            {
                let mut state_guard = state.write().await;
                *state_guard = ConnectionStatus::Disconnected;
            }
            
            // 停止运行
            {
                let mut running_guard = running.write().await;
                *running_guard = false;
            }
            
            // 停止任务
            {
                let mut task_guard = receive_task.lock().await;
                if let Some(task) = task_guard.take() {
                    task.abort();
                }
            }
            
            info!("WebSocket连接已断开: {} - {}", config_id, session_id);
            Ok(())
        })
    }
    
    fn clone_box(&self) -> Box<dyn Connection> {
        Box::new(WebSocketConnection {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
            stats: Arc::clone(&self.stats),
            last_activity: Arc::clone(&self.last_activity),
            session_id: self.session_id.clone(),
            ws_stream: Arc::clone(&self.ws_stream),
            receive_task: Arc::clone(&self.receive_task),
            running: Arc::clone(&self.running),
            message_parser: Arc::clone(&self.message_parser),
        })
    }
    
    fn update_last_activity(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let stats = Arc::clone(&self.stats);
        let last_activity = Arc::clone(&self.last_activity);
        Box::pin(async move {
            let mut stats_guard = stats.write().await;
            stats_guard.last_activity = chrono::Utc::now();
            
            let mut activity_guard = last_activity.write().await;
            *activity_guard = Instant::now();
        })
    }
    
    fn get_last_activity(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = chrono::DateTime<chrono::Utc>> + Send + '_>> {
        let stats = Arc::clone(&self.stats);
        Box::pin(async move {
            let stats_guard = stats.read().await;
            stats_guard.last_activity
        })
    }
}

/// WebSocket连接工厂
pub struct WebSocketConnectionFactory;

impl WebSocketConnectionFactory {
    /// 从现有的tokio-tungstenite连接创建WebSocket连接（MaybeTlsStream）
    pub fn from_tungstenite_stream(
        config: ConnectionConfig,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        message_parser: Arc<MessageParser>,
    ) -> WebSocketConnection {
        WebSocketConnection::from_tungstenite_stream(config, ws_stream, message_parser)
    }
    
    /// 从现有的tokio-tungstenite连接创建WebSocket连接（纯TcpStream）
    pub fn from_tungstenite_stream_plain(
        config: ConnectionConfig,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        message_parser: Arc<MessageParser>,
    ) -> WebSocketConnection {
        WebSocketConnection::from_tungstenite_stream_plain(config, ws_stream, message_parser)
    }
    
    /// 创建新的WebSocket连接（用于测试）
    pub fn create_connection(config: ConnectionConfig, message_parser: Arc<MessageParser>) -> WebSocketConnection {
        WebSocketConnection::new(config, message_parser)
    }
} 