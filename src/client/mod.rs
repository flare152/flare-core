//! 客户端模块
//!
//! 提供完整的客户端功能，包括连接管理、消息处理、事件回调等

pub mod callbacks;
pub mod config;
pub mod connection_manager;
pub mod events;
pub mod message_manager;
pub mod protocol_racer;
pub mod quic_connector;
pub mod traits;
pub mod types;
pub mod websocket_connector;

use crate::common::{
    error::Result,
    protocol::UnifiedProtocolMessage,
    TransportProtocol,
};
use crate::client::{
    config::ClientConfig,
    connection_manager::ConnectionManager,
    events::UnifiedClientEventHandler,
    message_manager::MessageManager,
    protocol_racer::ProtocolRacer,
    types::{MessagePriority, SendResult},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::info;

/// 客户端主结构体
/// 
/// 统一管理客户端的各种功能模块
pub struct Client {
    /// 配置
    config: ClientConfig,
    /// 事件处理器
    event_handler: Arc<UnifiedClientEventHandler>,
    /// 连接管理器
    connection_manager: Arc<Mutex<ConnectionManager>>,
    /// 消息管理器
    message_manager: Arc<Mutex<MessageManager>>,
    /// 协议竞速器
    protocol_racer: Arc<ProtocolRacer>,
}

impl Client {
    /// 创建新的客户端实例
    pub fn new(config: ClientConfig) -> Self {
        let event_handler = Arc::new(UnifiedClientEventHandler::default());
        
        let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(
            config.clone(),
            Arc::new(crate::common::MessageParser::with_custom_event_callback(
                event_handler.get_common_event_callback().clone()
            )),
        )));
        
        let message_manager = Arc::new(Mutex::new(
            MessageManager::new(
                config.clone(),
                Arc::clone(&connection_manager),
            ).with_event_callback(event_handler.get_common_event_callback().clone()),
        ));
        
        let protocol_racer = Arc::new(ProtocolRacer::new());
        
        Self {
            config,
            event_handler,
            connection_manager,
            message_manager,
            protocol_racer,
        }
    }

    /// 连接到服务器
    pub async fn connect(&mut self) -> Result<()> {
        info!("开始连接到服务器...");
        
        // 选择最佳协议
        let protocol = self.protocol_racer.get_best_protocol().await
            .ok_or_else(|| crate::common::FlareError::general_error("无法获取最佳协议".to_string()))?;
        
        match protocol {
            TransportProtocol::WebSocket => {
                let connector = websocket_connector::WebSocketConnector::new(
                    self.config.clone(),
                    Arc::new(crate::common::MessageParser::with_custom_event_callback(
                        self.event_handler.get_common_event_callback().clone()
                    )),
                );
                let server_url = self.config.server_addresses.get_protocol_url(TransportProtocol::WebSocket)
                    .ok_or_else(|| crate::common::FlareError::InvalidConfiguration("WebSocket服务器地址未配置".to_string()))?;
                let _connection = connector.create_connection(server_url, Duration::from_millis(self.config.connection_timeout_ms)).await?;
                Ok(())
            }
            TransportProtocol::QUIC => {
                let connector = quic_connector::QuicConnector::new(
                    self.config.clone(),
                    Arc::new(crate::common::MessageParser::with_custom_event_callback(
                        self.event_handler.get_common_event_callback().clone()
                    )),
                );
                let server_url = self.config.server_addresses.get_protocol_url(TransportProtocol::QUIC)
                    .ok_or_else(|| crate::common::FlareError::InvalidConfiguration("QUIC服务器地址未配置".to_string()))?;
                let _connection = connector.create_connection(server_url, Duration::from_millis(self.config.connection_timeout_ms)).await?;
                Ok(())
            }
            _ => {
                info!("使用协议竞速选择最佳协议");
                let mut racer = crate::client::protocol_racer::ProtocolRacer::new();
                let _protocol = racer.race_protocols(&self.config).await?;
                Ok(())
            }
        }
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        info!("断开连接...");
        
        let mut conn_manager = self.connection_manager.lock().await;
        conn_manager.disconnect().await?;
        
        info!("连接已断开");
        Ok(())
    }

    /// 发送消息
    pub async fn send_message(
        &self,
        message: UnifiedProtocolMessage,
        priority: MessagePriority,
        session_id: String,
    ) -> Result<SendResult> {
        let msg_manager = self.message_manager.lock().await;
        msg_manager.send_message(message, priority, session_id).await
    }

    /// 接收消息
    pub async fn receive_message(&self, message: UnifiedProtocolMessage) -> Result<()> {
        let msg_manager = self.message_manager.lock().await;
        msg_manager.receive_message(message).await
    }

    /// 获取连接状态
    pub async fn is_connected(&self) -> bool {
        let conn_manager = self.connection_manager.lock().await;
        conn_manager.is_connected().await
    }

    /// 获取事件处理器
    pub fn get_event_handler(&self) -> Arc<UnifiedClientEventHandler> {
        Arc::clone(&self.event_handler)
    }
}

 
 