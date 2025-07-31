use serde::{Deserialize, Serialize};
use std::fmt;

/// 传输协议枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportProtocol {
    QUIC,
    WebSocket,
}

impl fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportProtocol::QUIC => write!(f, "QUIC"),
            TransportProtocol::WebSocket => write!(f, "WebSocket"),
        }
    }
}

/// 连接状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// 连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub user_id: String,
    pub session_id: String,
    pub remote_addr: String,
    pub protocol: TransportProtocol,
    pub status: ConnectionStatus,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
} 