//! Flare IM 类型定义
//!
//! 提供客户端和服务端共享的类型定义

use serde::{Deserialize, Serialize};

/// 传输协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportProtocol {
    /// WebSocket协议
    WebSocket,
    /// QUIC协议
    QUIC,
    /// 自动选择（优先QUIC，回退WebSocket）
    Auto,
}

impl Default for TransportProtocol {
    fn default() -> Self {
        TransportProtocol::Auto
    }
}

impl std::fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportProtocol::WebSocket => write!(f, "WebSocket"),
            TransportProtocol::QUIC => write!(f, "QUIC"),
            TransportProtocol::Auto => write!(f, "Auto"),
        }
    }
}

/// 协议选择配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolSelection {
    /// 仅使用WebSocket
    WebSocketOnly,
    /// 仅使用QUIC
    QuicOnly,
    /// 同时使用WebSocket和QUIC
    Both,
    /// 自动选择（优先QUIC，回退WebSocket）
    Auto,
}

impl Default for ProtocolSelection {
    fn default() -> Self {
        ProtocolSelection::Both
    }
}

impl std::fmt::Display for ProtocolSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolSelection::WebSocketOnly => write!(f, "WebSocket Only"),
            ProtocolSelection::QuicOnly => write!(f, "QUIC Only"),
            ProtocolSelection::Both => write!(f, "Both"),
            ProtocolSelection::Auto => write!(f, "Auto"),
        }
    }
}

/// 连接状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// 未连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 重连中
    Reconnecting,
    /// 连接失败
    Failed,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Disconnected
    }
}

/// 连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// 连接ID
    pub id: String,
    /// 远程地址
    pub remote_addr: String,
    /// 传输协议
    pub protocol: TransportProtocol,
    /// 连接状态
    pub status: ConnectionStatus,
    /// 连接时间
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 最后活动时间
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    /// 发送字节数
    pub bytes_sent: u64,
    /// 接收字节数
    pub bytes_received: u64,
}

impl Default for ConnectionInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            remote_addr: String::new(),
            protocol: TransportProtocol::Auto,
            status: ConnectionStatus::Disconnected,
            connected_at: None,
            last_activity: None,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

 