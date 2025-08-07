//! Flare IM 类型定义
//!
//! 提供客户端和服务端共享的类型定义

use serde::{Deserialize, Serialize};

/// 传输协议类型枚举
/// 
/// 定义了支持的传输协议类型，用于指定连接使用的网络协议
/// 支持 WebSocket 和 QUIC 两种协议，以及自动选择模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportProtocol {
    /// WebSocket 协议，基于 HTTP 的 WebSocket 升级
    /// 适用于浏览器环境和需要代理的场景
    WebSocket,
    /// QUIC 协议，基于 UDP 的可靠传输协议
    /// 提供更低的延迟和更好的性能
    QUIC,
    /// 自动选择模式，优先使用 QUIC，失败时回退到 WebSocket
    /// 提供最佳的兼容性和性能平衡
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

/// 协议选择配置枚举
/// 
/// 定义了客户端连接时的协议选择策略
/// 可以指定使用单一协议或多种协议的组合
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolSelection {
    /// 仅使用 WebSocket 协议
    /// 适用于需要最大兼容性的场景
    WebSocketOnly,
    /// 仅使用 QUIC 协议
    /// 适用于追求最佳性能的场景
    QuicOnly,
    /// 同时使用 WebSocket 和 QUIC 协议
    /// 提供最大的连接成功率和冗余性
    Both,
    /// 自动选择模式，优先使用 QUIC，失败时回退到 WebSocket
    /// 在性能和兼容性之间取得平衡
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

/// 连接状态枚举
/// 
/// 定义了连接的生命周期状态
/// 用于跟踪和管理连接的当前状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// 未连接状态，连接尚未建立或已断开
    Disconnected,
    /// 正在连接状态，正在进行连接握手
    Connecting,
    /// 已连接状态，连接已建立并可以传输数据
    Connected,
    /// 重连中状态，连接断开后正在尝试重新连接
    Reconnecting,
    /// 连接失败状态，连接尝试失败且不再重试
    Failed,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Disconnected
    }
}

/// 连接信息结构体
/// 
/// 包含连接的详细信息和统计数据
/// 用于监控和管理连接的状态和性能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// 连接的唯一标识符
    pub id: String,
    /// 远程服务器的地址和端口
    pub remote_addr: String,
    /// 当前使用的传输协议类型
    pub protocol: TransportProtocol,
    /// 连接的当前状态
    pub status: ConnectionStatus,
    /// 连接建立的时间戳，None 表示未连接
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 最后一次数据传输的时间戳，None 表示无活动
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    /// 已发送的字节总数
    pub bytes_sent: u64,
    /// 已接收的字节总数
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

/// 平台类型枚举
///
/// 定义了支持的各种客户端平台类型，用于标识连接的来源平台
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Platform {
    /// Android 移动设备平台
    Android,
    /// iOS 移动设备平台
    IOS,
    /// Web 浏览器平台
    Web,
    /// 桌面应用程序平台
    Desktop,
    /// 服务器端平台
    Server,
    /// WebSocket 专用平台
    WebSocket,
    /// 未知或未识别的平台
    Unknown,
}

impl Default for Platform {
    fn default() -> Self {
        Platform::Unknown
    }
}
