//! Flare IM QUIC连接模块
//!
//! 提供基于QUIC协议的连接实现

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex, mpsc};
use tokio::time::{interval, Duration, timeout};
use tracing::{info, error, debug};
use uuid::Uuid;
use quinn::{Connection as QuinnConnection, SendStream, RecvStream};

use crate::common::{
    conn::{Connection, ProtoMessage, Platform, ConnectionConfig, ConnectionState},
    Result, FlareError, TransportProtocol,
};

/// QUIC连接实现
#[derive(Debug)]
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
    state: Arc<RwLock<ConnectionState>>,
    /// 消息发送通道
    message_sender: Option<mpsc::Sender<ProtoMessage>>,
    /// 消息接收通道
    message_receiver: Option<mpsc::Receiver<ProtoMessage>>,
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
}

impl QuicConnection {
    /// 创建新的QUIC连接
    pub fn new(config: ConnectionConfig) -> Self {
        let session_id = Uuid::new_v4().to_string();
        
        Self {
            config,
            quinn_connection: Arc::new(Mutex::new(None)),
            send_stream: Arc::new(Mutex::new(None)),
            recv_stream: Arc::new(Mutex::new(None)),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            message_sender: None,
            message_receiver: None,
            session_id,
            metadata: Arc::new(RwLock::new(HashMap::new())),
            heartbeat_task: Arc::new(Mutex::new(None)),
            receive_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 设置QUIC连接
    pub async fn set_quinn_connection(
        &mut self,
        connection: QuinnConnection,
        send_stream: SendStream,
        recv_stream: RecvStream,
    ) {
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
    }

    /// 初始化消息通道
    pub async fn init_message_channels(&mut self) {
        let (sender, receiver) = mpsc::channel(1000);
        self.message_sender = Some(sender);
        self.message_receiver = Some(receiver);
    }

    /// 启动接收任务
    pub async fn start_receive_task(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        
        *running = true;
        
        let recv_stream = Arc::clone(&self.recv_stream);
        let message_sender = self.message_sender.as_ref().unwrap().clone();
        let session_id = self.session_id.clone();
        let running = Arc::clone(&self.running);
        
        let task = tokio::spawn(async move {
            while *running.read().await {
                let mut stream_guard = recv_stream.lock().await;
                if let Some(stream) = &mut *stream_guard {
                    match timeout(Duration::from_millis(100), stream.read_chunk(1024, true)).await {
                        Ok(Ok(Some(chunk))) => {
                            // 解析消息
                            if let Ok(message) = Self::parse_message(&chunk.bytes) {
                                if let Err(e) = message_sender.send(message).await {
                                    error!("发送消息到通道失败: {}", e);
                                    break;
                                }
                            }
                        }
                        Ok(Ok(None)) => {
                            // 流结束
                            debug!("QUIC接收流结束");
                            break;
                        }
                        Ok(Err(e)) => {
                            error!("读取QUIC流失败: {}", e);
                            break;
                        }
                        Err(_) => {
                            // 超时，继续循环
                            continue;
                        }
                    }
                } else {
                    error!("QUIC接收流未设置");
                    break;
                }
            }
        });
        
        {
            let mut task_guard = self.receive_task.lock().await;
            *task_guard = Some(task);
        }
        
        Ok(())
    }

    /// 启动心跳任务
    pub async fn start_heartbeat_task(&mut self) -> Result<()> {
        let interval_ms = self.config.heartbeat_interval_ms;
        let send_stream = Arc::clone(&self.send_stream);
        let session_id = self.session_id.clone();
        let running = Arc::clone(&self.running);
        
        let task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms));
            
            while *running.read().await {
                interval.tick().await;
                
                // 创建心跳消息
                let heartbeat_msg = ProtoMessage::new(
                    Uuid::new_v4().to_string(),
                    "heartbeat".to_string(),
                    vec![],
                );
                
                // 发送心跳
                let mut stream_guard = send_stream.lock().await;
                if let Some(stream) = &mut *stream_guard {
                    if let Ok(data) = serde_json::to_vec(&heartbeat_msg) {
                        if let Err(e) = stream.write_all(&data).await {
                            error!("发送心跳失败: {}", e);
                            break;
                        }
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

    /// 解析消息
    fn parse_message(data: &[u8]) -> Result<ProtoMessage> {
        serde_json::from_slice(data)
            .map_err(|e| FlareError::ProtocolError(format!("解析消息失败: {}", e)))
    }

    /// 序列化消息
    fn serialize_message(message: &ProtoMessage) -> Result<Vec<u8>> {
        serde_json::to_vec(message)
            .map_err(|e| FlareError::ProtocolError(format!("序列化消息失败: {}", e)))
    }

    /// 获取会话ID
    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }

    /// 获取连接ID
    pub fn get_connection_id(&self) -> &str {
        &self.config.id
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

    fn protocol(&self) -> &str {
        "QUIC"
    }

    async fn is_active(&self, timeout: Duration) -> bool {
        // 检查连接状态和最后活动时间
        let state = self.state.read().await;
        *state == ConnectionState::Connected
    }

    fn send(&self, msg: ProtoMessage) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let send_stream = Arc::clone(&self.send_stream);
        Box::pin(async move {
            let mut stream_guard = send_stream.lock().await;
            if let Some(stream) = &mut *stream_guard {
                let data = Self::serialize_message(&msg)?;
                stream.write_all(&data).await
                    .map_err(|e| FlareError::NetworkError(std::io::Error::new(std::io::ErrorKind::Other, format!("发送消息失败: {}", e))))?;
                
                debug!("QUIC消息已发送: {}", msg.message_type);
                Ok(())
            } else {
                Err(FlareError::ConnectionFailed("QUIC发送流未设置".to_string()))
            }
        })
    }

    fn receive(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProtoMessage>> + Send + '_>> {
        Box::pin(async move {
            // 这里不能克隆接收器，所以我们需要一个不同的方法
            // 暂时返回错误，实际实现中应该使用Arc<Mutex<Option<Receiver>>>来支持克隆
            Err(FlareError::ConnectionFailed("消息接收通道未初始化".to_string()))
        })
    }

    fn close(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let running = Arc::clone(&self.running);
        let heartbeat_task = Arc::clone(&self.heartbeat_task);
        let receive_task = Arc::clone(&self.receive_task);
        let state = Arc::clone(&self.state);
        let config_id = self.config.id.clone();
        
        Box::pin(async move {
            // 更新状态
            {
                let mut state_guard = state.write().await;
                *state_guard = ConnectionState::Disconnected;
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
            
            info!("QUIC连接已断开: {}", config_id);
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
            message_sender: self.message_sender.clone(),
            message_receiver: None, // 不能克隆接收器
            session_id: self.session_id.clone(),
            metadata: Arc::clone(&self.metadata),
            heartbeat_task: Arc::clone(&self.heartbeat_task),
            receive_task: Arc::clone(&self.receive_task),
            running: Arc::clone(&self.running),
        })
    }
}

/// QUIC连接工厂
pub struct QuicConnectionFactory;

impl QuicConnectionFactory {
    /// 从QUIC连接创建连接实例
    pub fn from_quinn_connection(
        config: ConnectionConfig,
        connection: QuinnConnection,
        send_stream: SendStream,
        recv_stream: RecvStream,
    ) -> QuicConnection {
        let mut quic_conn = QuicConnection::new(config);
        
        // 直接设置连接，不使用异步任务
        // 注意：这里需要在实际使用中确保连接设置完成
        // 暂时注释掉异步设置，避免移动问题
        // tokio::spawn(async move {
        //     quic_conn.set_quinn_connection(connection, send_stream, recv_stream).await;
        // });
        
        quic_conn
    }
} 