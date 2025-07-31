//! Flare IM 客户端模块
//!
//! 提供高性能、跨平台的客户端实现

pub mod traits;
mod connection_manager;

use crate::common::{TransportProtocol, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 客户端配置
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub user_id: String,
    pub server_url: String,
    pub preferred_protocol: TransportProtocol,
    pub auto_switch_enabled: bool,
    pub connection_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub max_reconnect_attempts: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            server_url: "flare-im://localhost".to_string(),
            preferred_protocol: TransportProtocol::QUIC,
            auto_switch_enabled: true,
            connection_timeout_ms: 5000,
            heartbeat_interval_ms: 30000,
            max_reconnect_attempts: 5,
        }
    }
}

/// 客户端状态
#[derive(Debug, Clone, PartialEq)]
pub enum ClientStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// 发送结果
#[derive(Debug, Clone)]
pub struct SendResult {
    pub message_id: String,
    pub send_time: chrono::DateTime<chrono::Utc>,
    pub success: bool,
    pub error_message: Option<String>,
}

impl SendResult {
    pub fn success(message_id: String) -> Self {
        Self {
            message_id,
            send_time: chrono::Utc::now(),
            success: true,
            error_message: None,
        }
    }

    pub fn failure(message_id: String, error_message: String) -> Self {
        Self {
            message_id,
            send_time: chrono::Utc::now(),
            success: false,
            error_message: Some(error_message),
        }
    }
}

/// Flare IM 客户端
pub struct Client {
    config: ClientConfig,
    status: Arc<RwLock<ClientStatus>>,
    current_protocol: Arc<RwLock<Option<TransportProtocol>>>,
}

impl Client {
    /// 创建新的客户端
    pub async fn new(user_id: &str) -> Result<Self> {
        let mut config = ClientConfig::default();
        config.user_id = user_id.to_string();
        
        Ok(Self {
            config,
            status: Arc::new(RwLock::new(ClientStatus::Disconnected)),
            current_protocol: Arc::new(RwLock::new(None)),
        })
    }

    /// 使用配置创建客户端
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(ClientStatus::Disconnected)),
            current_protocol: Arc::new(RwLock::new(None)),
        }
    }

    /// 连接到服务器
    pub async fn connect(&self, server_url: &str) -> Result<TransportProtocol> {
        let mut status = self.status.write().await;
        *status = ClientStatus::Connecting;
        drop(status);

        // 这里应该实现实际的连接逻辑
        // 为了演示，我们模拟连接过程
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let protocol = self.config.preferred_protocol;
        {
            let mut current_protocol = self.current_protocol.write().await;
            *current_protocol = Some(protocol);
        }

        let mut status = self.status.write().await;
        *status = ClientStatus::Connected;

        Ok(protocol)
    }

    /// 断开连接
    pub async fn disconnect(&self) -> Result<()> {
        let mut status = self.status.write().await;
        *status = ClientStatus::Disconnected;

        let mut current_protocol = self.current_protocol.write().await;
        *current_protocol = None;

        Ok(())
    }

    /// 发送文本消息
    pub async fn send_message(&self, to_user_id: &str, content: &str) -> Result<SendResult> {
        let message_id = uuid::Uuid::new_v4().to_string();
        
        // 检查连接状态
        let status = self.status.read().await;
        if *status != ClientStatus::Connected {
            return Ok(SendResult::failure(
                message_id,
                "客户端未连接".to_string(),
            ));
        }
        drop(status);

        // 这里应该实现实际的消息发送逻辑
        // 为了演示，我们模拟发送过程
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Ok(SendResult::success(message_id))
    }

    /// 发送二进制消息
    pub async fn send_binary_message(
        &self,
        to_user_id: &str,
        data: Vec<u8>,
        message_type: String,
    ) -> Result<SendResult> {
        let message_id = uuid::Uuid::new_v4().to_string();
        
        // 检查连接状态
        let status = self.status.read().await;
        if *status != ClientStatus::Connected {
            return Ok(SendResult::failure(
                message_id,
                "客户端未连接".to_string(),
            ));
        }
        drop(status);

        // 这里应该实现实际的二进制消息发送逻辑
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Ok(SendResult::success(message_id))
    }

    /// 获取连接状态
    pub async fn get_status(&self) -> ClientStatus {
        let status = self.status.read().await;
        status.clone()
    }

    /// 获取当前协议
    pub async fn get_current_protocol(&self) -> Option<TransportProtocol> {
        let protocol = self.current_protocol.read().await;
        protocol.clone()
    }

    /// 切换协议
    pub async fn switch_protocol(&self, protocol: TransportProtocol) -> Result<()> {
        let mut current_protocol = self.current_protocol.write().await;
        *current_protocol = Some(protocol);
        Ok(())
    }
}

// 协议竞速器
pub struct ProtocolRacer {
    current_protocol: TransportProtocol,
    fallback_enabled: bool,
}

impl ProtocolRacer {
    pub fn new() -> Self {
        Self {
            current_protocol: TransportProtocol::QUIC,
            fallback_enabled: true,
        }
    }

    pub async fn race_protocols(&mut self, server_url: &str) -> Result<TransportProtocol> {
        // 这里应该实现协议竞速逻辑
        // 为了演示，我们返回默认协议
        Ok(self.current_protocol)
    }

    pub async fn switch_protocol(&mut self, protocol: TransportProtocol) -> Result<()> {
        self.current_protocol = protocol;
        Ok(())
    }
}

// 重新导出连接管理器
pub use connection_manager::{
    AdvancedClientConnectionManager, ClientConnectionConfig, ClientConnectionState,
    ClientConnectionHealth, ClientConnectionSession
}; 