//! 客户端连接管理器
//!
//! 负责管理客户端连接，包括连接创建、重连、心跳等功能

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{info, error};

use crate::common::{
    conn::Connection,
    Result, FlareError, TransportProtocol,
};
use crate::server::conn_manager::memory::ConnectionEvent;

/// 连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// 连接管理器
pub struct ConnectionManager {
    config: crate::client::config::ClientConfig,
    state: Arc<RwLock<ConnectionState>>,
    current_connection: Arc<Mutex<Option<Box<dyn Connection + Send + Sync>>>>,
    last_connection_time: Arc<RwLock<Option<Instant>>>,
    reconnect_count: Arc<RwLock<u32>>,
    event_callback: Arc<Mutex<Option<Box<dyn Fn(ConnectionEvent) + Send + Sync>>>>,
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub fn new(config: crate::client::config::ClientConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            current_connection: Arc::new(Mutex::new(None)),
            last_connection_time: Arc::new(RwLock::new(None)),
            reconnect_count: Arc::new(RwLock::new(0)),
            event_callback: Arc::new(Mutex::new(None)),
        }
    }

    /// 设置事件回调
    pub async fn set_event_callback(&mut self, callback: Box<dyn Fn(ConnectionEvent) + Send + Sync>) {
        let mut callback_guard = self.event_callback.lock().await;
        *callback_guard = Some(callback);
    }

    /// 触发事件
    async fn trigger_event(&self, event: ConnectionEvent) {
        if let Some(callback) = &*self.event_callback.lock().await {
            callback(event);
        }
    }

    /// 连接到服务器
    pub async fn connect(&mut self) -> Result<()> {
        info!("开始连接到服务器");
        
        // 更新状态为连接中
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Connecting;
        }

        match self.config.protocol_selection_mode {
            crate::client::config::ProtocolSelectionMode::AutoRacing => {
                self.connect_with_racing().await
            }
            crate::client::config::ProtocolSelectionMode::Specific(protocol) => {
                self.connect_with_protocol(protocol).await
            }
            crate::client::config::ProtocolSelectionMode::Manual => {
                Err(FlareError::ConnectionFailed("手动模式需要用户选择协议".to_string()))
            }
        }
    }

    /// 使用协议竞速模式连接
    async fn connect_with_racing(&mut self) -> Result<()> {
        info!("使用协议竞速模式连接");
        
        let mut racer = crate::client::protocol_racer::ProtocolRacer::new();
        let best_protocol = racer.race_protocols(&self.config).await?;
        
        info!("协议竞速完成，选择协议: {:?}", best_protocol);
        self.connect_with_protocol(best_protocol).await
    }

    /// 使用指定协议连接
    async fn connect_with_protocol(&mut self, protocol: TransportProtocol) -> Result<()> {
        info!("使用协议连接: {:?}", protocol);
        
        let connection = match protocol {
            TransportProtocol::WebSocket => {
                self.create_websocket_connection().await?
            }
            TransportProtocol::QUIC => {
                self.create_quic_connection().await?
            }
            TransportProtocol::Auto => {
                // 自动选择：优先QUIC，回退WebSocket
                match self.create_quic_connection().await {
                    Ok(conn) => conn,
                    Err(_) => self.create_websocket_connection().await?,
                }
            }
        };

        // 设置连接
        {
            let mut conn_guard = self.current_connection.lock().await;
            *conn_guard = Some(connection);
        }

        // 更新状态和时间
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Connected;
        }
        {
            let mut time = self.last_connection_time.write().await;
            *time = Some(Instant::now());
        }

        // 触发连接成功事件
        self.trigger_event(ConnectionEvent::Connected).await;

        info!("连接成功建立");
        Ok(())
    }

    /// 创建 WebSocket 连接
    async fn create_websocket_connection(&self) -> Result<Box<dyn Connection + Send + Sync>> {
        info!("创建 WebSocket 连接");
        
        // 获取 WebSocket URL
        let ws_url = self.config.server_addresses.websocket_url.as_ref()
            .or_else(|| self.config.server_addresses.websocket_tls_url.as_ref())
            .ok_or_else(|| FlareError::ConnectionFailed("未配置 WebSocket 地址".to_string()))?;

        let connector = crate::client::websocket_connector::WebSocketConnector::new(self.config.clone());
        let timeout = Duration::from_millis(self.config.connection_timeout_ms);
        
        connector.create_connection(ws_url, timeout).await
    }

    /// 创建 QUIC 连接
    async fn create_quic_connection(&self) -> Result<Box<dyn Connection + Send + Sync>> {
        info!("创建 QUIC 连接");
        
        let quic_url = self.config.server_addresses.quic_url.as_ref()
            .ok_or_else(|| FlareError::ConnectionFailed("未配置 QUIC 地址".to_string()))?;

        let connector = crate::client::quic_connector::QuicConnector::new(self.config.clone());
        let timeout = Duration::from_millis(self.config.connection_timeout_ms);
        
        connector.create_connection(quic_url, timeout).await
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        info!("断开连接");
        
        // 关闭当前连接
        if let Some(connection) = self.current_connection.lock().await.take() {
            if let Err(e) = connection.close().await {
                error!("关闭连接时出错: {}", e);
            }
        }

        // 更新状态
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Disconnected;
        }

        // 触发断开连接事件
        self.trigger_event(ConnectionEvent::Disconnected).await;

        info!("连接已断开");
        Ok(())
    }

    /// 重新连接
    pub async fn reconnect(&mut self) -> Result<()> {
        info!("开始重新连接");
        
        // 检查重连次数限制
        {
            let reconnect_count = *self.reconnect_count.read().await;
            if reconnect_count >= self.config.max_reconnect_attempts {
                return Err(FlareError::ConnectionFailed("达到最大重连次数".to_string()));
            }
        }

        // 更新状态
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Reconnecting;
        }

        // 增加重连计数
        {
            let mut count = self.reconnect_count.write().await;
            *count += 1;
        }

        // 触发重连事件
        self.trigger_event(ConnectionEvent::Reconnecting).await;

        // 等待重连延迟
        tokio::time::sleep(Duration::from_millis(self.config.reconnect_delay_ms)).await;

        // 尝试重新连接
        let result = self.connect().await;

        if result.is_ok() {
            // 重置重连计数
            {
                let mut count = self.reconnect_count.write().await;
                *count = 0;
            }
        }

        result
    }

    /// 发送消息
    pub async fn send_message(&self, message: crate::common::protocol::UnifiedProtocolMessage) -> Result<()> {
        let connection_guard = self.current_connection.lock().await;
        if let Some(ref conn) = *connection_guard {
            conn.send(message).await
        } else {
            Err(FlareError::ConnectionFailed("未连接".to_string()))
        }
    }

    /// 检查是否已连接
    pub async fn is_connected(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, ConnectionState::Connected)
    }

    /// 获取连接状态
    pub async fn get_state(&self) -> ConnectionState {
        let state = self.state.read().await;
        state.clone()
    }

    /// 获取重连次数
    pub async fn get_reconnect_count(&self) -> u32 {
        let count = self.reconnect_count.read().await;
        *count
    }

    /// 获取最后连接时间
    pub async fn get_last_connection_time(&self) -> Option<Instant> {
        let time = self.last_connection_time.read().await;
        *time
    }
} 