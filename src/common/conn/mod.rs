//! Flare IM 连接模块
//!
//! 提供统一的连接接口和客户端、服务端连接实现

// 导出子模块
pub mod quic;
pub mod websocket;

// 重新导出公共类型
pub use quic::*;
pub use websocket::*; 

use std::pin::Pin;
use std::future::Future;
use std::time::Duration;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::{Result, TransportProtocol};

/// 平台类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Platform {
    Android,
    IOS,
    Web,
    Desktop,
    Server,
    WebSocket,
    Unknown,
}

impl Default for Platform {
    fn default() -> Self {
        Platform::Unknown
    }
}

/// 协议消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtoMessage {
    pub id: String,
    pub message_type: String,
    pub payload: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ProtoMessage {
    pub fn new(id: String, message_type: String, payload: Vec<u8>) -> Self {
        Self {
            id,
            message_type,
            payload,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// 连接标准接口
#[async_trait]
pub trait Connection: Send + Sync + std::fmt::Debug {
    /// 获取连接ID
    fn id(&self) -> &str;
    
    /// 远程地址
    fn remote_addr(&self) -> &str;
    
    /// 平台
    fn platform(&self) -> Platform;
    
    /// 协议名称
    fn protocol(&self) -> &str;
    
    /// 检查连接是否活跃
    /// 
    /// # 参数
    /// * `timeout` - 超时时间，如果最后活动时间超过此值则认为连接不活跃
    /// 
    /// # 返回
    /// * `bool` - true 表示连接活跃，false 表示连接不活跃
    async fn is_active(&self, timeout: Duration) -> bool;
    
    /// 发送消息
    fn send(&self, msg: ProtoMessage) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// 接收消息
    fn receive(&self) -> Pin<Box<dyn Future<Output = Result<ProtoMessage>> + Send + '_>>;
    
    /// 关闭连接
    fn close(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// 克隆
    fn clone_box(&self) -> Box<dyn Connection>;
}

/// 连接状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// 连接统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub connection_time_ms: u64,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub reconnect_count: u32,
}

impl Default for ConnectionStats {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
            connection_time_ms: 0,
            last_activity: chrono::Utc::now(),
            reconnect_count: 0,
        }
    }
}

/// 连接事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionEvent {
    Connected,
    Disconnected,
    Reconnecting,
    Failed(String),
    MessageReceived(ProtoMessage),
    MessageSent(ProtoMessage),
    Heartbeat,
    Error(String),
}

/// 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub remote_addr: String,
    pub platform: Platform,
    pub protocol: TransportProtocol,
    pub timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            remote_addr: String::new(),
            platform: Platform::Unknown,
            protocol: TransportProtocol::QUIC,
            timeout_ms: 10000,
            heartbeat_interval_ms: 30000,
            max_reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
        }
    }
}

/// 连接构建器
pub struct ConnectionBuilder {
    config: ConnectionConfig,
}

impl ConnectionBuilder {
    /// 创建新的连接构建器
    pub fn new() -> Self {
        Self {
            config: ConnectionConfig::default(),
        }
    }
    
    /// 设置连接ID
    pub fn id(mut self, id: String) -> Self {
        self.config.id = id;
        self
    }
    
    /// 设置远程地址
    pub fn remote_addr(mut self, remote_addr: String) -> Self {
        self.config.remote_addr = remote_addr;
        self
    }
    
    /// 设置平台
    pub fn platform(mut self, platform: Platform) -> Self {
        self.config.platform = platform;
        self
    }
    
    /// 设置协议
    pub fn protocol(mut self, protocol: TransportProtocol) -> Self {
        self.config.protocol = protocol;
        self
    }
    
    /// 设置超时时间
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.config.timeout_ms = timeout_ms;
        self
    }
    
    /// 设置心跳间隔
    pub fn heartbeat_interval_ms(mut self, heartbeat_interval_ms: u64) -> Self {
        self.config.heartbeat_interval_ms = heartbeat_interval_ms;
        self
    }
    
    /// 设置最大重连次数
    pub fn max_reconnect_attempts(mut self, max_reconnect_attempts: u32) -> Self {
        self.config.max_reconnect_attempts = max_reconnect_attempts;
        self
    }
    
    /// 设置重连延迟
    pub fn reconnect_delay_ms(mut self, reconnect_delay_ms: u64) -> Self {
        self.config.reconnect_delay_ms = reconnect_delay_ms;
        self
    }
    
    /// 构建连接配置
    pub fn build(self) -> ConnectionConfig {
        self.config
    }
}

impl Default for ConnectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
