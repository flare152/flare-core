//! Flare IM WebSocket 连接实现
//!
//! 基于WebSocket协议的连接实现，支持普通TCP和TLS两种模式

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tokio::time::Duration;
use futures_util::{SinkExt, StreamExt};
use tracing::{info, warn, error, debug};
use async_trait::async_trait;

use crate::common::{
    conn::{Connection, ConnectionConfig, ConnectionState, Platform, ProtoMessage, ConnectionStats, ConnectionEvent},
    error::{Result, FlareError},
};

/// WebSocket连接 - 支持普通TCP和TLS两种模式
pub struct WebSocketConnection {
    config: ConnectionConfig,
    state: Arc<RwLock<ConnectionState>>,
    stats: Arc<RwLock<ConnectionStats>>,
    last_activity: Arc<RwLock<Instant>>,
    event_callback: Arc<Mutex<Option<Box<dyn Fn(ConnectionEvent) + Send + Sync>>>>,
    
    // WebSocket特定字段
    session_id: String,
    
    // WebSocket流 - 使用枚举支持两种类型
    ws_stream: Arc<Mutex<Option<WebSocketStreamEnum>>>,
    
    // 接收任务
    receive_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// WebSocket流枚举 - 支持两种类型
enum WebSocketStreamEnum {
    MaybeTls(tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>),
    Plain(tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>),
}

impl std::fmt::Debug for WebSocketConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketConnection")
            .field("config", &self.config)
            .field("state", &self.state)
            .field("session_id", &self.session_id)
            .finish()
    }
}

impl WebSocketConnection {
    /// 从现有的tokio-tungstenite连接创建WebSocket连接（MaybeTlsStream）
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
            ws_stream: Arc::new(Mutex::new(Some(WebSocketStreamEnum::MaybeTls(ws_stream)))),
            receive_task: Arc::new(Mutex::new(None)),
        }
    }
    
    /// 从现有的tokio-tungstenite连接创建WebSocket连接（纯TcpStream）
    pub fn from_tungstenite_stream_plain(
        config: ConnectionConfig,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    ) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Connected)),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            event_callback: Arc::new(Mutex::new(None)),
            session_id,
            ws_stream: Arc::new(Mutex::new(Some(WebSocketStreamEnum::Plain(ws_stream)))),
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
        let stream_guard = self.ws_stream.lock().await;
        stream_guard.is_some()
    }
    
    /// 启动接收任务
    pub async fn start_receive_task(&self) -> Result<()> {
        let mut stream_guard = self.ws_stream.lock().await;
        if let Some(ws_stream_enum) = stream_guard.take() {
            drop(stream_guard);
            
            let session_id = self.session_id.clone();
            let stats = Arc::clone(&self.stats);
            let activity = Arc::clone(&self.last_activity);
            let event_callback = Arc::clone(&self.event_callback);
            let state = Arc::clone(&self.state);
            let ws_stream_arc = Arc::new(Mutex::new(ws_stream_enum));
            
            let task = tokio::spawn(async move {
                info!("开始WebSocket接收任务: {}", session_id);
                
                loop {
                    let msg_result = {
                        let mut stream = ws_stream_arc.lock().await;
                        match &mut *stream {
                            WebSocketStreamEnum::MaybeTls(ws_stream) => ws_stream.next().await,
                            WebSocketStreamEnum::Plain(ws_stream) => ws_stream.next().await,
                        }
                    };
                    
                    match msg_result {
                        Some(Ok(msg)) => {
                            if let Err(e) = Self::handle_websocket_message(
                                msg, &session_id, &stats, &activity, &event_callback
                            ).await {
                                error!("处理WebSocket消息失败: {}", e);
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("WebSocket接收错误: {}", e);
                            break;
                        }
                        None => {
                            info!("WebSocket连接已关闭");
                            break;
                        }
                    }
                }
                
                // 更新连接状态
                {
                    let mut state_guard = state.write().await;
                    *state_guard = ConnectionState::Disconnected;
                }
                
                // 触发断开连接事件
                if let Some(callback) = &*event_callback.lock().await {
                    callback(ConnectionEvent::Disconnected);
                }
                
                info!("WebSocket接收任务结束: {}", session_id);
            });
            
            let mut task_guard = self.receive_task.lock().await;
            *task_guard = Some(task);
            
            Ok(())
        } else {
            Err(FlareError::ConnectionFailed("WebSocket流不可用".to_string()))
        }
    }
    
    /// 处理WebSocket消息
    async fn handle_websocket_message(
        msg: tokio_tungstenite::tungstenite::Message,
        session_id: &str,
        stats: &Arc<RwLock<ConnectionStats>>,
        activity: &Arc<RwLock<Instant>>,
        event_callback: &Arc<Mutex<Option<Box<dyn Fn(ConnectionEvent) + Send + Sync>>>>,
    ) -> Result<()> {
        info!("收到WebSocket消息: {:?}", msg);
        match msg {
            tokio_tungstenite::tungstenite::Message::Text(text) => {
                debug!("收到WebSocket文本消息: {}", text);
                
                // 更新活动时间
                {
                    let mut activity_guard = activity.write().await;
                    *activity_guard = Instant::now();
                }
                
                // 更新统计信息
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.bytes_received += text.len() as u64;
                    stats_guard.messages_received += 1;
                    stats_guard.last_activity = chrono::Utc::now();
                }
                
                // 解析消息并触发事件
                match serde_json::from_str::<ProtoMessage>(&text) {
                    Ok(proto_msg) => {
                        if let Some(callback) = &*event_callback.lock().await {
                            callback(ConnectionEvent::MessageReceived(proto_msg));
                        }
                    }
                    Err(e) => {
                        warn!("解析WebSocket消息失败: {}", e);
                    }
                }
            }
            tokio_tungstenite::tungstenite::Message::Binary(data) => {
                debug!("收到WebSocket二进制消息: {} bytes", data.len());
                
                // 更新活动时间
                {
                    let mut activity_guard = activity.write().await;
                    *activity_guard = Instant::now();
                }
                
                // 更新统计信息
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.bytes_received += data.len() as u64;
                    stats_guard.messages_received += 1;
                    stats_guard.last_activity = chrono::Utc::now();
                }
                
                // 触发二进制消息事件
                if let Some(callback) = &*event_callback.lock().await {
                    let proto_msg = ProtoMessage::new(
                        uuid::Uuid::new_v4().to_string(),
                        "binary".to_string(),
                        data.to_vec(),
                    );
                    callback(ConnectionEvent::MessageReceived(proto_msg));
                }
            }
            tokio_tungstenite::tungstenite::Message::Ping(data) => {
                debug!("收到WebSocket Ping: {} bytes", data.len());
                // 可以在这里处理心跳
            }
            tokio_tungstenite::tungstenite::Message::Pong(data) => {
                debug!("收到WebSocket Pong: {} bytes", data.len());
                // 可以在这里处理心跳响应
            }
            tokio_tungstenite::tungstenite::Message::Close(frame) => {
                info!("收到WebSocket关闭消息: {:?}", frame);
                // 连接将被关闭
            }
            tokio_tungstenite::tungstenite::Message::Frame(_) => {
                // 忽略原始帧
            }
        }
        
        Ok(())
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
    
    fn protocol(&self) -> &str {
        "websocket"
    }
    
    async fn is_active(&self, _timeout: Duration) -> bool {
        let state = self.state.read().await;
        matches!(state.clone(), ConnectionState::Connected)
    }
    
    fn send(&self, msg: ProtoMessage) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let ws_stream = Arc::clone(&self.ws_stream);
        let session_id = self.session_id.clone();
        
        Box::pin(async move {
            let mut stream_guard = ws_stream.lock().await;
            if let Some(ref mut stream_enum) = *stream_guard {
                let text = serde_json::to_string(&msg).map_err(|e| {
                    FlareError::NetworkError(std::io::Error::new(std::io::ErrorKind::Other, e))
                })?;
                
                let ws_msg = tokio_tungstenite::tungstenite::Message::Text(text.into());
                match stream_enum {
                    WebSocketStreamEnum::MaybeTls(ws_stream) => {
                        match ws_stream.send(ws_msg).await {
                            Ok(_) => {
                                info!("WebSocket消息发送成功: {}", session_id);
                                Ok(())
                            }
                            Err(e) => {
                                error!("WebSocket消息发送失败: {}", e);
                                Err(FlareError::NetworkError(std::io::Error::new(std::io::ErrorKind::Other, e)))
                            }
                        }
                    }
                    WebSocketStreamEnum::Plain(ws_stream) => {
                        match ws_stream.send(ws_msg).await {
                            Ok(_) => {
                                info!("WebSocket消息发送成功: {}", session_id);
                                Ok(())
                            }
                            Err(e) => {
                                error!("WebSocket消息发送失败: {}", e);
                                Err(FlareError::NetworkError(std::io::Error::new(std::io::ErrorKind::Other, e)))
                            }
                        }
                    }
                }
            } else {
                Err(FlareError::ConnectionFailed("WebSocket流不可用".to_string()))
            }
        })
    }
    
    fn receive(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProtoMessage>> + Send + '_>> {
        // 简化：直接返回错误，因为消息通过事件回调处理
        Box::pin(async move {
            Err(FlareError::ConnectionFailed("消息通过事件回调处理".to_string()))
        })
    }
    
    fn close(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
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
            if let Some(mut stream_enum) = ws_stream.lock().await.take() {
                let close_msg = tokio_tungstenite::tungstenite::Message::Close(None);
                match &mut stream_enum {
                    WebSocketStreamEnum::MaybeTls(ws_stream) => {
                        if let Err(e) = ws_stream.send(close_msg).await {
                            error!("发送WebSocket关闭消息失败: {}", e);
                        }
                    }
                    WebSocketStreamEnum::Plain(ws_stream) => {
                        if let Err(e) = ws_stream.send(close_msg).await {
                            error!("发送WebSocket关闭消息失败: {}", e);
                        }
                    }
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
            receive_task: Arc::clone(&self.receive_task),
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
    ) -> WebSocketConnection {
        WebSocketConnection::from_tungstenite_stream(config, ws_stream)
    }
    
    /// 从现有的tokio-tungstenite连接创建WebSocket连接（纯TcpStream）
    pub fn from_tungstenite_stream_plain(
        config: ConnectionConfig,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    ) -> WebSocketConnection {
        WebSocketConnection::from_tungstenite_stream_plain(config, ws_stream)
    }
    
    /// 创建新的WebSocket连接（用于测试）
    pub fn create_connection(config: ConnectionConfig) -> WebSocketConnection {
        WebSocketConnection::new(config)
    }
} 