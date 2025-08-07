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
use crate::common::protocol::UnifiedProtocolMessage;
use crate::common::types::Platform;

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
    fn protocol(&self) -> TransportProtocol;
    
    /// 检查连接是否活跃
    /// 
    /// # 参数
    /// * `timeout` - 超时时间，如果最后活动时间超过此值则认为连接不活跃
    /// 
    /// # 返回
    /// * `bool` - true 表示连接活跃，false 表示连接不活跃
    async fn is_active(&self, timeout: Duration) -> bool;
    
    /// 发送消息
    fn send(&self, msg: UnifiedProtocolMessage) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// 启动接收消息任务
    /// 
    /// # 参数
    /// * `callback` - 消息接收回调函数，当收到消息时会调用此函数
    /// 
    /// # 返回
    /// * `Result<()>` - 启动成功返回 Ok(()), 失败返回错误
    fn start_receive_task(&self, callback: Box<dyn Fn(UnifiedProtocolMessage) + Send + Sync>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// 关闭连接
    fn close(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// 克隆
    fn clone_box(&self) -> Box<dyn Connection>;
    
    /// 更新最后活跃时间
    fn update_last_activity(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
    
    /// 获取最后活跃时间
    fn get_last_activity(&self) -> Pin<Box<dyn Future<Output = chrono::DateTime<chrono::Utc>> + Send + '_>>;
}


/// 连接统计信息结构体
/// 
/// 记录连接的各种统计指标，用于监控连接的性能和状态
/// 包括数据传输量、消息数量、连接时间和重连次数等信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    /// 已发送的字节总数
    pub bytes_sent: u64,
    /// 已接收的字节总数
    pub bytes_received: u64,
    /// 已发送的消息总数
    pub messages_sent: u64,
    /// 已接收的消息总数
    pub messages_received: u64,
    /// 连接建立以来的总时间（毫秒）
    pub connection_time_ms: u64,
    /// 最后一次活动的时间戳
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// 重连尝试的次数
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

/// 连接配置结构体
/// 
/// 定义了建立连接所需的各种配置参数
/// 包括连接标识、远程地址、平台类型、协议类型和各种超时设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// 连接的唯一标识符
    pub id: String,
    /// 远程服务器的地址和端口
    /// 格式：host:port 或 ws://host:port 或 wss://host:port
    pub remote_addr: String,
    /// 客户端平台类型
    pub platform: Platform,
    /// 传输协议类型（QUIC 或 WebSocket）
    pub protocol: TransportProtocol,
    /// 连接超时时间（毫秒）
    pub timeout_ms: u64,
    /// 最大重连尝试次数
    pub max_reconnect_attempts: u32,
    /// 重连延迟时间（毫秒）
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
            max_reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
        }
    }
}

/// 连接构建器结构体
/// 
/// 提供链式调用的方式来构建连接配置
/// 使用 Builder 模式，可以逐步设置各种连接参数
pub struct ConnectionBuilder {
    /// 内部存储的连接配置
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
