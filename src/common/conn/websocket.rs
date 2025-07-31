//! Flare IM WebSocket 连接实现
//!
//! 基于WebSocket协议的连接实现，包装tokio-tungstenite连接

use std::pin::Pin;
use std::future::Future;
use std::time::{Duration, Instant};
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::{RwLock, Mutex, mpsc};
use tracing::{info, error, debug};
use futures_util::{StreamExt, SinkExt};

use super::{
    Connection, ConnectionConfig, ConnectionState, ConnectionStats, 
    ConnectionEvent, Platform
};
use crate::common::{
    conn::ProtoMessage,
    Result, FlareError, TransportProtocol,
};

/// WebSocket 连接实现
#[derive(Clone)]
pub struct WebSocketConnection {
    config: ConnectionConfig,
    state: Arc<RwLock<ConnectionState>>,
    stats: Arc<RwLock<ConnectionStats>>,
    last_activity: Arc<RwLock<Instant>>,
    event_callback: Arc<Mutex<Option<Box<dyn Fn(ConnectionEvent) + Send + Sync>>>>,
    
    // WebSocket特定字段
    session_id: String,
    
    // tokio-tungstenite连接包装
    ws_stream: Arc<Mutex<Option<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>>,
    
    // 消息通道
    message_tx: Arc<Mutex<Option<mpsc::Sender<ProtoMessage>>>>,
    message_rx: Arc<Mutex<Option<mpsc::Receiver<ProtoMessage>>>>,
    
    // 接收任务
    receive_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl std::fmt::Debug for WebSocketConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketConnection")
            .field("config", &self.config)
            .field("state", &self.state)
            .field("stats", &self.stats)
            .field("last_activity", &self.last_activity)
            .field("session_id", &self.session_id)
            .field("ws_stream", &"<WebSocketStream>")
            .field("message_tx", &"<mpsc::Sender>")
            .field("message_rx", &"<mpsc::Receiver>")
            .field("receive_task", &"<JoinHandle>")
            .finish()
    }
}

impl WebSocketConnection {
    /// 从现有的tokio-tungstenite连接创建WebSocket连接
    pub fn from_tungstenite_stream(
        config: ConnectionConfig,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    ) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Connected)),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            event_callback: Arc::new(Mutex::new(None)),
            session_id,
            ws_stream: Arc::new(Mutex::new(Some(ws_stream))),
            message_tx: Arc::new(Mutex::new(None)),
            message_rx: Arc::new(Mutex::new(None)),
            receive_task: Arc::new(Mutex::new(None)),
        }
    }
    
    /// 创建新的WebSocket连接（用于测试）
    pub fn new(config: ConnectionConfig) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            event_callback: Arc::new(Mutex::new(None)),
            session_id,
            ws_stream: Arc::new(Mutex::new(None)),
            message_tx: Arc::new(Mutex::new(None)),
            message_rx: Arc::new(Mutex::new(None)),
            receive_task: Arc::new(Mutex::new(None)),
        }
    }
    
    /// 设置事件回调
    pub fn set_event_callback(&mut self, callback: Box<dyn Fn(ConnectionEvent) + Send + Sync>) {
        let mut callback_guard = self.event_callback.blocking_lock();
        *callback_guard = Some(callback);
    }
    
    /// 触发事件
    async fn trigger_event(&self, event: ConnectionEvent) {
        if let Some(callback) = &*self.event_callback.lock().await {
            callback(event);
        }
    }
    
    /// 更新活动时间
    async fn update_activity(&self) {
        let mut activity = self.last_activity.write().await;
        *activity = Instant::now();
    }
    
    /// 更新统计信息
    async fn update_stats(&self, bytes_sent: u64, bytes_received: u64) {
        let mut stats = self.stats.write().await;
        stats.bytes_sent += bytes_sent;
        stats.bytes_received += bytes_received;
        stats.last_activity = chrono::Utc::now();
        
        if bytes_sent > 0 {
            stats.messages_sent += 1;
        }
        if bytes_received > 0 {
            stats.messages_received += 1;
        }
    }
    
    /// 获取连接统计
    pub async fn get_stats(&self) -> ConnectionStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// 获取连接状态
    pub async fn get_state(&self) -> ConnectionState {
        let state = self.state.read().await;
        state.clone()
    }
    
    /// 获取会话ID
    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }
    
    /// 检查是否有WebSocket流
    pub async fn has_ws_stream(&self) -> bool {
        let stream = self.ws_stream.lock().await;
        stream.is_some()
    }
    
    /// 初始化消息通道
    pub async fn init_message_channels(&mut self) {
        let (tx, rx) = mpsc::channel(100);
        {
            let mut tx_guard = self.message_tx.lock().await;
            *tx_guard = Some(tx);
        }
        {
            let mut rx_guard = self.message_rx.lock().await;
            *rx_guard = Some(rx);
        }
    }
    
    /// 处理心跳消息
    async fn handle_heartbeat(&self, user_id: String, protocol: TransportProtocol) -> Result<()> {
        debug!("处理心跳消息: 用户 {}, 协议: {:?}", user_id, protocol);
        
        // 更新活动时间
        self.update_activity().await;
        
        // 触发心跳事件
        self.trigger_event(ConnectionEvent::Heartbeat).await;
        
        // 创建心跳确认消息
        let proto_msg = ProtoMessage::new(
            uuid::Uuid::new_v4().to_string(),
            "heartbeat_ack".to_string(),
            vec![],
        );
        
        // 发送心跳确认消息
        self.send(proto_msg).await?;
        
        Ok(())
    }
    
    /// 处理Ping消息并发送Pong响应
    async fn handle_ping(&self, ping_data: Vec<u8>) -> Result<()> {
        debug!("收到Ping消息: {} 字节, 会话ID: {}", ping_data.len(), self.session_id);
        
        // 更新活动时间
        self.update_activity().await;
        
        // 发送Pong响应
        if let Some(mut stream) = self.ws_stream.lock().await.take() {
            let pong_msg = tokio_tungstenite::tungstenite::Message::Pong(ping_data.clone());
            
            match stream.send(pong_msg).await {
                Ok(_) => {
                    debug!("发送Pong响应成功: 会话ID: {}", self.session_id);
                    
                    // 更新统计
                    {
                        let mut stats_guard = self.stats.write().await;
                        stats_guard.bytes_sent += ping_data.len() as u64;
                    }
                    
                    // 更新活动时间
                    {
                        let mut activity_guard = self.last_activity.write().await;
                        *activity_guard = Instant::now();
                    }
                }
                Err(e) => {
                    error!("发送Pong响应失败: {}", e);
                }
            }
            
            // 将流放回
            *self.ws_stream.lock().await = Some(stream);
        }
        
        Ok(())
    }
    
    /// 启动接收任务
    pub async fn start_receive_task(&self) -> Result<()> {
        let ws_stream = Arc::clone(&self.ws_stream);
        let event_callback = Arc::clone(&self.event_callback);
        let stats = Arc::clone(&self.stats);
        let activity = Arc::clone(&self.last_activity);
        let session_id = self.session_id.clone();
        let config = self.config.clone();
        
        // 克隆self的引用用于处理心跳
        let heartbeat_handler = Arc::new(self.clone_box());
        
        let task = tokio::spawn(async move {
            if let Some(mut stream) = ws_stream.lock().await.take() {
                while let Some(msg_result) = stream.next().await {
                    match msg_result {
                        Ok(msg) => {
                            match msg {
                                tokio_tungstenite::tungstenite::Message::Text(text) => {
                                    // 解析文本消息
                                    match serde_json::from_str::<ProtoMessage>(&text) {
                                        Ok(proto_msg) => {
                                            // 更新统计
                                            {
                                                let mut stats_guard = stats.write().await;
                                                stats_guard.bytes_received += text.len() as u64;
                                                stats_guard.messages_received += 1;
                                                stats_guard.last_activity = chrono::Utc::now();
                                            }
                                            
                                            // 更新活动时间
                                            {
                                                let mut activity_guard = activity.write().await;
                                                *activity_guard = Instant::now();
                                            }
                                            
                                            // 检查是否是心跳消息
                                            if proto_msg.message_type == "heartbeat" {
                                                debug!("收到心跳消息: 会话ID: {}", session_id);
                                            } else {
                                                // 触发普通消息事件
                                                if let Some(callback) = &*event_callback.lock().await {
                                                    callback(ConnectionEvent::MessageReceived(proto_msg));
                                                }
                                            }
                                            
                                            debug!("WebSocket接收文本消息: {} 字节, 会话ID: {}", text.len(), session_id);
                                        }
                                        Err(e) => {
                                            error!("解析WebSocket消息失败: {}", e);
                                        }
                                    }
                                }
                                tokio_tungstenite::tungstenite::Message::Binary(data) => {
                                    // 解析二进制消息
                                    match serde_json::from_slice::<ProtoMessage>(&data) {
                                        Ok(proto_msg) => {
                                            // 更新统计
                                            {
                                                let mut stats_guard = stats.write().await;
                                                stats_guard.bytes_received += data.len() as u64;
                                                stats_guard.messages_received += 1;
                                                stats_guard.last_activity = chrono::Utc::now();
                                            }
                                            
                                            // 更新活动时间
                                            {
                                                let mut activity_guard = activity.write().await;
                                                *activity_guard = Instant::now();
                                            }
                                            
                                            // 检查是否是心跳消息
                                            if proto_msg.message_type == "heartbeat" {
                                                debug!("收到心跳消息: 会话ID: {}", session_id);
                                            } else {
                                                // 触发普通消息事件
                                                if let Some(callback) = &*event_callback.lock().await {
                                                    callback(ConnectionEvent::MessageReceived(proto_msg));
                                                }
                                            }
                                            
                                            debug!("WebSocket接收二进制消息: {} 字节, 会话ID: {}", data.len(), session_id);
                                        }
                                        Err(e) => {
                                            error!("解析WebSocket消息失败: {}", e);
                                        }
                                    }
                                }
                                tokio_tungstenite::tungstenite::Message::Close(close_frame) => {
                                    debug!("WebSocket连接关闭: {}, 关闭帧: {:?}", session_id, close_frame);
                                    
                                    // 触发连接关闭事件
                                    if let Some(callback) = &*event_callback.lock().await {
                                        callback(ConnectionEvent::Disconnected);
                                    }
                                    
                                    break;
                                }
                                tokio_tungstenite::tungstenite::Message::Ping(ping_data) => {
                                    // 处理Ping消息
                                    debug!("收到Ping消息: {} 字节", ping_data.len());
                                }
                                tokio_tungstenite::tungstenite::Message::Pong(_) => {
                                    debug!("WebSocket收到Pong: {}", session_id);
                                    
                                    // 更新活动时间
                                    {
                                        let mut activity_guard = activity.write().await;
                                        *activity_guard = Instant::now();
                                    }
                                    
                                    // 触发心跳事件
                                    if let Some(callback) = &*event_callback.lock().await {
                                        callback(ConnectionEvent::Heartbeat);
                                    }
                                }
                                tokio_tungstenite::tungstenite::Message::Frame(_) => {
                                    // 忽略原始帧
                                }
                            }
                        }
                        Err(e) => {
                            error!("WebSocket接收消息错误: {}", e);
                            
                            // 触发错误事件
                            if let Some(callback) = &*event_callback.lock().await {
                                callback(ConnectionEvent::Error(format!("接收消息错误: {}", e)));
                            }
                            
                            break;
                        }
                    }
                }
                
                // 连接已断开，触发断开事件
                if let Some(callback) = &*event_callback.lock().await {
                    callback(ConnectionEvent::Disconnected);
                }
                
                // 将流放回
                *ws_stream.lock().await = Some(stream);
            }
        });
        
        {
            let mut task_guard = self.receive_task.lock().await;
            *task_guard = Some(task);
        }
        
        Ok(())
    }
}

#[async_trait]
impl Connection for WebSocketConnection {
    fn id(&self) -> &str {
        &self.config.id
    }
    
    fn remote_addr(&self) -> &str {
        &self.config.remote_addr
    }
    
    fn platform(&self) -> Platform {
        self.config.platform.clone()
    }
    
    fn protocol(&self) -> &str {
        "WebSocket"
    }
    
    async fn is_active(&self, timeout: Duration) -> bool {
        let activity = self.last_activity.read().await;
        let elapsed = activity.elapsed();
        elapsed < timeout
    }
    
    fn send(&self, msg: ProtoMessage) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let stats = Arc::clone(&self.stats);
        let activity = Arc::clone(&self.last_activity);
        let event_callback = Arc::clone(&self.event_callback);
        let ws_stream = Arc::clone(&self.ws_stream);
        let session_id = self.session_id.clone();
        
        Box::pin(async move {
            // 序列化消息
            let msg_json = match serde_json::to_string(&msg) {
                Ok(json) => json,
                Err(e) => {
                    error!("序列化WebSocket消息失败: {}", e);
                    return Err(crate::FlareError::SerializationError(e));
                }
            };
            
            // 发送到WebSocket流
            if let Some(mut stream) = ws_stream.lock().await.take() {
                let ws_msg = tokio_tungstenite::tungstenite::Message::Text(msg_json.clone());
                
                match stream.send(ws_msg).await {
                    Ok(_) => {
                        // 更新统计
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.bytes_sent += msg_json.len() as u64;
                            stats_guard.messages_sent += 1;
                            stats_guard.last_activity = chrono::Utc::now();
                        }
                        
                        // 更新活动时间
                        {
                            let mut activity_guard = activity.write().await;
                            *activity_guard = Instant::now();
                        }
                        
                        // 触发事件
                        if let Some(callback) = &*event_callback.lock().await {
                            callback(ConnectionEvent::MessageSent(msg));
                        }
                        
                        debug!("WebSocket发送消息: {} 字节, 会话ID: {}", msg_json.len(), session_id);
                        
                        // 将流放回
                        *ws_stream.lock().await = Some(stream);
                        Ok(())
                    }
                    Err(e) => {
                        error!("WebSocket发送消息失败: {}", e);
                        // 将流放回
                        *ws_stream.lock().await = Some(stream);
                        Err(crate::FlareError::NetworkError(std::io::Error::new(std::io::ErrorKind::Other, e)))
                    }
                }
            } else {
                Err(crate::FlareError::ConnectionFailed("WebSocket流不可用".to_string()))
            }
        })
    }
    
    fn receive(&self) -> Pin<Box<dyn Future<Output = Result<ProtoMessage>> + Send + '_>> {
        let message_rx = Arc::clone(&self.message_rx);
        let session_id = self.session_id.clone();
        
        Box::pin(async move {
            // 从内部消息队列接收消息
            if let Some(rx) = &mut *message_rx.lock().await {
                match rx.recv().await {
                    Some(msg) => {
                        debug!("WebSocket从队列接收消息: {} 字节, 会话ID: {}", msg.payload.len(), session_id);
                        Ok(msg)
                    }
                    None => {
                        Err(crate::FlareError::ConnectionFailed("WebSocket消息通道已关闭".to_string()))
                    }
                }
            } else {
                Err(crate::FlareError::ConnectionFailed("WebSocket消息通道未初始化".to_string()))
            }
        })
    }
    
    fn close(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let state = Arc::clone(&self.state);
        let event_callback = Arc::clone(&self.event_callback);
        let ws_stream = Arc::clone(&self.ws_stream);
        let receive_task = Arc::clone(&self.receive_task);
        let session_id = self.session_id.clone();
        
        Box::pin(async move {
            // 停止接收任务
            {
                let mut task_guard = receive_task.lock().await;
                if let Some(task) = task_guard.take() {
                    task.abort();
                }
            }
            
            // 关闭WebSocket连接
            if let Some(mut stream) = ws_stream.lock().await.take() {
                let close_msg = tokio_tungstenite::tungstenite::Message::Close(None);
                if let Err(e) = stream.send(close_msg).await {
                    error!("发送WebSocket关闭消息失败: {}", e);
                }
                debug!("关闭WebSocket连接: {}", session_id);
            }
            
            // 更新状态
            {
                let mut state_guard = state.write().await;
                *state_guard = ConnectionState::Disconnected;
            }
            
            // 触发事件
            if let Some(callback) = &*event_callback.lock().await {
                callback(ConnectionEvent::Disconnected);
            }
            
            info!("WebSocket连接已关闭: {}", session_id);
            Ok(())
        })
    }
    
    fn clone_box(&self) -> Box<dyn Connection> {
        Box::new(WebSocketConnection {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
            stats: Arc::clone(&self.stats),
            last_activity: Arc::clone(&self.last_activity),
            event_callback: Arc::clone(&self.event_callback),
            session_id: self.session_id.clone(),
            ws_stream: Arc::clone(&self.ws_stream),
            message_tx: Arc::clone(&self.message_tx),
            message_rx: Arc::clone(&self.message_rx),
            receive_task: Arc::clone(&self.receive_task),
        })
    }
}

/// WebSocket连接工厂
pub struct WebSocketConnectionFactory;

impl WebSocketConnectionFactory {
    /// 从现有的tokio-tungstenite连接创建WebSocket连接（TLS）
    pub fn from_tungstenite_stream(
        config: ConnectionConfig,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    ) -> WebSocketConnection {
        WebSocketConnection::from_tungstenite_stream(config, ws_stream)
    }
    
    /// 从现有的tokio-tungstenite连接创建WebSocket连接（非TLS）
    pub fn from_tungstenite_stream_plain(
        config: ConnectionConfig,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    ) -> WebSocketConnection {
        // 创建一个新的连接实例
        let session_id = uuid::Uuid::new_v4().to_string();
        
        WebSocketConnection {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Connected)),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            event_callback: Arc::new(Mutex::new(None)),
            session_id,
            ws_stream: Arc::new(Mutex::new(None)), // 暂时设为None，后续需要重新设计
            message_tx: Arc::new(Mutex::new(None)),
            message_rx: Arc::new(Mutex::new(None)),
            receive_task: Arc::new(Mutex::new(None)),
        }
    }
    
    /// 创建新的WebSocket连接（用于测试）
    pub fn create_connection(config: ConnectionConfig) -> WebSocketConnection {
        WebSocketConnection::new(config)
    }
} 