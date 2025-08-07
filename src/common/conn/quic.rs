//! Flare IM QUIC连接模块
//!
//! 提供基于QUIC协议的连接实现

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex, mpsc};
use tokio::time::{Duration, timeout};
use tracing::{info, error, debug};
use uuid::Uuid;
use quinn::{Connection as QuinnConnection, SendStream, RecvStream};

use crate::common::{
    conn::{Connection, ConnectionConfig, ConnectionStats}, 
    Result, FlareError,
    MessageParser, MessageCallback
};
use crate::common::types::{Platform, ConnectionStatus};
use crate::common::protocol::UnifiedProtocolMessage;
use crate::TransportProtocol;

/// QUIC连接实现
/// 
/// 提供基于QUIC协议的连接功能，包括：
/// - 消息发送和接收
/// - 连接状态管理
/// - 心跳保活
/// - 元数据管理
/// - 异步任务管理
pub struct QuicConnection {
    /// 连接配置
    config: ConnectionConfig,
    /// QUIC连接
    quinn_connection: Arc<Mutex<Option<QuinnConnection>>>,
    /// 发送流
    send_stream: Arc<Mutex<Option<SendStream>>>,
    /// 接收流
    recv_stream: Arc<Mutex<Option<RecvStream>>>,
    /// 连接状态
    state: Arc<RwLock<ConnectionStatus>>,
    /// 连接统计信息
    stats: Arc<RwLock<ConnectionStats>>,
    /// 消息发送通道
    message_sender: Option<mpsc::Sender<UnifiedProtocolMessage>>,
    /// 消息接收通道
    message_receiver: Option<mpsc::Receiver<UnifiedProtocolMessage>>,
    /// 会话ID
    session_id: String,
    /// 元数据
    metadata: Arc<RwLock<HashMap<String, String>>>,
    /// 心跳任务句柄
    heartbeat_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 接收任务句柄
    receive_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
    /// 消息解析器
    message_parser: Arc<MessageParser>,
}

impl QuicConnection {
    /// 创建新的QUIC连接
    pub fn new(config: ConnectionConfig) -> Self {
        let session_id = Uuid::new_v4().to_string();
        
        debug!("创建新的QUIC连接: {} - {}", config.id, session_id);
        
        Self {
            config,
            quinn_connection: Arc::new(Mutex::new(None)),
            send_stream: Arc::new(Mutex::new(None)),
            recv_stream: Arc::new(Mutex::new(None)),
            state: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            message_sender: None,
            message_receiver: None,
            session_id,
            metadata: Arc::new(RwLock::new(HashMap::new())),
            heartbeat_task: Arc::new(Mutex::new(None)),
            receive_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
            message_parser: Arc::new(MessageParser::with_default_callbacks()),
        }
    }

    /// 设置QUIC连接
    pub async fn set_quinn_connection(
        &mut self,
        connection: QuinnConnection,
        send_stream: SendStream,
        recv_stream: RecvStream,
    ) {
        debug!("设置QUIC连接: {}", self.session_id);
        
        {
            let mut conn = self.quinn_connection.lock().await;
            *conn = Some(connection);
        }
        
        {
            let mut send = self.send_stream.lock().await;
            *send = Some(send_stream);
        }
        
        {
            let mut recv = self.recv_stream.lock().await;
            *recv = Some(recv_stream);
        }
        
        // 更新连接状态
        {
            let mut state = self.state.write().await;
            *state = ConnectionStatus::Connected;
        }
        
        info!("QUIC连接已设置: {}", self.session_id);
    }

    /// 初始化消息通道
    pub async fn init_message_channels(&mut self) {
        let (sender, receiver) = mpsc::channel(1000);
        self.message_sender = Some(sender);
        self.message_receiver = Some(receiver);
    }


    // /// 启动心跳任务
    // pub async fn start_heartbeat_task(&mut self) -> Result<()> {
    //     let interval_ms = self.config.heartbeat_interval_ms;
    //     let send_stream = Arc::clone(&self.send_stream);
    //     let _session_id = self.session_id.clone();
    //     let running = Arc::clone(&self.running);
    //
    //     let task = tokio::spawn(async move {
    //         let mut interval = interval(Duration::from_millis(interval_ms));
    //
    //         while *running.read().await {
    //             interval.tick().await;
    //
    //             // 创建心跳消息
    //             let heartbeat_msg = ProtoMessage::new(
    //                 Uuid::new_v4().to_string(),
    //                 "heartbeat".to_string(),
    //                 vec![],
    //             );
    //
    //             // 发送心跳
    //             let mut stream_guard = send_stream.lock().await;
    //             if let Some(stream) = &mut *stream_guard {
    //                 if let Ok(data) = serde_json::to_vec(&heartbeat_msg) {
    //                     if let Err(e) = stream.write_all(&data).await {
    //                         error!("发送心跳失败: {}", e);
    //                         break;
    //                     }
    //                 }
    //             }
    //         }
    //     });
    //
    //     {
    //         let mut task_guard = self.heartbeat_task.lock().await;
    //         *task_guard = Some(task);
    //     }
    //
    //     Ok(())
    // }

    /// 解析消息
    fn parse_message(data: &[u8]) -> Result<UnifiedProtocolMessage> {
        serde_json::from_slice(data)
            .map_err(|e| FlareError::protocol_error(format!("解析QUIC消息失败: {}", e)))
    }

    /// 序列化消息
    fn serialize_message(message: &UnifiedProtocolMessage) -> Result<Vec<u8>> {
        serde_json::to_vec(message)
            .map_err(|e| FlareError::protocol_error(format!("序列化QUIC消息失败: {}", e)))
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
    
    /// 设置元数据
    pub async fn set_metadata(&self, key: String, value: String) {
        let mut metadata = self.metadata.write().await;
        metadata.insert(key, value);
    }
    
    /// 获取元数据
    pub async fn get_metadata(&self, key: &str) -> Option<String> {
        let metadata = self.metadata.read().await;
        metadata.get(key).cloned()
    }
    
    /// 获取所有元数据
    pub async fn get_all_metadata(&self) -> HashMap<String, String> {
        let metadata = self.metadata.read().await;
        metadata.clone()
    }
    
    /// 更新最后活跃时间
    async fn update_last_activity_internal(&self) {
        let mut stats = self.stats.write().await;
        stats.last_activity = chrono::Utc::now();
    }
    
    /// 设置消息解析器
    pub async fn set_message_parser(&mut self, parser: Arc<MessageParser>) {
        self.message_parser = parser;
    }
}

impl std::fmt::Debug for QuicConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuicConnection")
            .field("config", &self.config)
            .field("session_id", &self.session_id)
            .field("quinn_connection", &"<Arc<Mutex<Option<QuinnConnection>>>>")
            .field("send_stream", &"<Arc<Mutex<Option<SendStream>>>>")
            .field("recv_stream", &"<Arc<Mutex<Option<RecvStream>>>>")
            .field("state", &"<Arc<RwLock<ConnectionStatus>>>")
            .field("stats", &"<Arc<RwLock<ConnectionStats>>>")
            .field("message_sender", &self.message_sender.is_some())
            .field("message_receiver", &self.message_receiver.is_some())
            .field("metadata", &"<Arc<RwLock<HashMap<String, String>>>>")
            .field("heartbeat_task", &"<Arc<Mutex<Option<JoinHandle<()>>>>>")
            .field("receive_task", &"<Arc<Mutex<Option<JoinHandle<()>>>>>")
            .field("running", &"<Arc<RwLock<bool>>>")
            .field("message_parser", &"<Arc<MessageParser>>")
            .finish()
    }
}

#[async_trait::async_trait]
impl Connection for QuicConnection {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn remote_addr(&self) -> &str {
        &self.config.remote_addr
    }

    fn platform(&self) -> Platform {
        self.config.platform.clone()
    }

    fn protocol(&self) -> TransportProtocol {
        TransportProtocol::QUIC
    }

    async fn is_active(&self, timeout: Duration) -> bool {
        // 检查连接状态
        let state = self.state.read().await;
        if *state != ConnectionStatus::Connected {
            return false;
        }
        
        // 检查运行状态
        let running = self.running.read().await;
        if !*running {
            return false;
        }
        
        // 检查QUIC连接是否有效
        let conn_guard = self.quinn_connection.lock().await;
        if let Some(connection) = &*conn_guard {
            // 检查连接状态
            match connection.close_reason() {
                None => true, // 连接正常
                Some(_) => false, // 连接已关闭
            }
        } else {
            false // 连接未设置
        }
    }

    fn send(&self, msg: UnifiedProtocolMessage) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let send_stream = Arc::clone(&self.send_stream);
        let stats = Arc::clone(&self.stats);
        Box::pin(async move {
            let mut stream_guard = send_stream.lock().await;
            if let Some(stream) = &mut *stream_guard {
                let data = Self::serialize_message(&msg)?;
                stream.write_all(&data).await
                    .map_err(|e| FlareError::message_send_failed(format!("发送消息失败: {}", e)))?;
                
                // 更新最后活跃时间和统计信息
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.last_activity = chrono::Utc::now();
                    stats_guard.bytes_sent += data.len() as u64;
                    stats_guard.messages_sent += 1;
                }
                
                debug!("QUIC消息已发送: {:?}", msg);
                Ok(())
            } else {
                Err(FlareError::connection_failed("QUIC发送流未设置"))
            }
        })
    }

    fn start_receive_task(&self, callback: Box<dyn Fn(UnifiedProtocolMessage) + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let recv_stream = Arc::clone(&self.recv_stream);
        let running = Arc::clone(&self.running);
        let stats = Arc::clone(&self.stats);
        let message_parser = Arc::clone(&self.message_parser);
        let session_id = self.session_id.clone();
  
        
        Box::pin(async move {
            let mut running_guard = running.write().await;
            if *running_guard {
                debug!("QUIC接收任务已在运行: {}", session_id);
                return Ok(()); // 任务已经在运行
            }
            *running_guard = true;
            drop(running_guard);
            
            let task = tokio::spawn(async move {
                debug!("QUIC接收任务开始运行: {}", session_id);
                while *running.read().await {
                    let mut stream_guard = recv_stream.lock().await;
                    if let Some(stream) = &mut *stream_guard {
                        match timeout(Duration::from_millis(100), stream.read_chunk(1024, true)).await {
                            Ok(Ok(Some(chunk))) => {
                                // 更新最后活跃时间和统计信息
                                {
                                    let mut stats_guard = stats.write().await;
                                    stats_guard.last_activity = chrono::Utc::now();
                                    stats_guard.bytes_received += chunk.bytes.len() as u64;
                                    stats_guard.messages_received += 1;
                                }
                                
                                // 使用消息解析器处理消息
                                match message_parser.handle_message(session_id.clone(),&chunk.bytes).await {
                                    Ok(()) => {
                                        debug!("QUIC消息已通过解析器处理");
                                    }
                                    Err(e) => {
                                        error!("QUIC消息解析失败: {}", e);
                                        // 如果解析失败，创建自定义消息
                                        let custom_msg = UnifiedProtocolMessage::custom_message(
                                            "binary_data".to_string(),
                                            chunk.bytes.to_vec()
                                        );
                                        debug!("QUIC接收到自定义二进制消息");
                                        callback(custom_msg);
                                    }
                                }
                            }
                            Ok(Ok(None)) => {
                                // 流结束
                                debug!("QUIC接收流结束: {}", session_id);
                                break;
                            }
                            Ok(Err(e)) => {
                                error!("读取QUIC流失败: {} - {}", session_id, e);
                                break;
                            }
                            Err(_) => {
                                // 超时，继续循环
                                continue;
                            }
                        }
                    } else {
                        error!("QUIC接收流未设置: {}", session_id);
                        break;
                    }
                }
                debug!("QUIC接收任务结束: {}", session_id);
            });
            {
                let mut task_guard = self.receive_task.lock().await;
                *task_guard = Some(task);
            }
            Ok(())
        })
    }

    fn close(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let running = Arc::clone(&self.running);
        let heartbeat_task = Arc::clone(&self.heartbeat_task);
        let receive_task = Arc::clone(&self.receive_task);
        let state = Arc::clone(&self.state);
        let config_id = self.config.id.clone();
        let session_id = self.session_id.clone();
        
        Box::pin(async move {
            debug!("开始关闭QUIC连接: {} - {}", config_id, session_id);
            
            // 发送连接关闭事件
            let unified_msg = crate::common::protocol::UnifiedProtocolMessage::disconnect(
                config_id.clone(),
                session_id.clone(),
                "Connection closed by client".to_string()
            );

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
                let mut task_guard = heartbeat_task.lock().await;
                if let Some(task) = task_guard.take() {
                    task.abort();
                }
            }
            
            {
                let mut task_guard = receive_task.lock().await;
                if let Some(task) = task_guard.take() {
                    task.abort();
                }
            }
            
            info!("QUIC连接已断开: {} - {}", config_id, session_id);
            Ok(())
        })
    }

    fn clone_box(&self) -> Box<dyn Connection> {
        Box::new(QuicConnection {
            config: self.config.clone(),
            quinn_connection: Arc::clone(&self.quinn_connection),
            send_stream: Arc::clone(&self.send_stream),
            recv_stream: Arc::clone(&self.recv_stream),
            state: Arc::clone(&self.state),
            stats: Arc::clone(&self.stats),
            message_sender: self.message_sender.clone(),
            message_receiver: None, // 不能克隆接收器
            session_id: self.session_id.clone(),
            metadata: Arc::clone(&self.metadata),
            heartbeat_task: Arc::clone(&self.heartbeat_task),
            receive_task: Arc::clone(&self.receive_task),
            running: Arc::clone(&self.running),
            message_parser: Arc::clone(&self.message_parser),
        })
    }
    
    fn update_last_activity(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let stats = Arc::clone(&self.stats);
        Box::pin(async move {
            let mut stats_guard = stats.write().await;
            stats_guard.last_activity = chrono::Utc::now();
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

/// QUIC连接工厂
pub struct QuicConnectionFactory;

impl QuicConnectionFactory {
    /// 从QUIC连接创建连接实例
    pub async fn from_quinn_connection(
        config: ConnectionConfig,
        connection: QuinnConnection,
        send_stream: SendStream,
        recv_stream: RecvStream,
    ) -> QuicConnection {
        let mut quic_conn = QuicConnection::new(config);
        
        // 设置QUIC连接和流
        quic_conn.set_quinn_connection(connection, send_stream, recv_stream).await;
        
        // 初始化消息通道
        quic_conn.init_message_channels().await;
        
        quic_conn
    }
} 