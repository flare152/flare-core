//! Flare IM 协议模块
//!
//! 定义消息协议和序列化格式

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 协议消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub message_type: String,
    pub payload: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

impl Message {
    pub fn new(message_type: String, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type,
            payload,
            timestamp: chrono::Utc::now(),
            user_id: None,
            session_id: None,
            metadata: None,
        }
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 协议消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    /// 连接消息
    Connect(ConnectMessage),
    /// 断开连接消息
    Disconnect(DisconnectMessage),
    /// 心跳消息
    Heartbeat(HeartbeatMessage),
    /// 心跳确认消息
    HeartbeatAck(HeartbeatAckMessage),
    /// 数据消息
    Data(DataMessage),
    /// 错误消息
    Error(ErrorMessage),
}

/// 连接消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectMessage {
    pub user_id: String,
    pub session_id: String,
    pub protocol: crate::common::types::TransportProtocol,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// 断开连接消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisconnectMessage {
    pub user_id: String,
    pub session_id: String,
    pub reason: String,
}

/// 心跳消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub user_id: String,
    pub session_id: String,
    pub sequence: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 心跳确认消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatAckMessage {
    pub user_id: String,
    pub session_id: String,
    pub sequence: u64,
    pub latency_ms: Option<u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 数据消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMessage {
    pub user_id: String,
    pub session_id: String,
    pub data_type: String,
    pub payload: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 错误消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub error_code: String,
    pub error_message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
} 