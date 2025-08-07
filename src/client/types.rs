//! 客户端类型定义模块
//!
//! 定义客户端相关的类型和结构

use crate::common::{TransportProtocol, UnifiedProtocolMessage};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 客户端状态枚举
/// 
/// 定义了客户端连接的生命周期状态
/// 用于跟踪客户端的连接状态变化
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientStatus {
    /// 未连接状态，客户端尚未建立连接
    Disconnected,
    /// 正在连接状态，客户端正在尝试建立连接
    Connecting,
    /// 已连接状态，客户端已成功连接到服务器
    Connected,
    /// 重连中状态，连接断开后正在尝试重新连接
    Reconnecting,
    /// 连接失败状态，连接尝试失败且不再重试
    Failed,
}

/// 消息发送结果结构体
/// 
/// 记录消息发送的详细结果信息
/// 包括发送状态、时间戳、错误信息和重试次数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    /// 被发送消息的唯一标识符
    pub message_id: String,
    /// 消息发送的时间戳
    pub send_time: chrono::DateTime<chrono::Utc>,
    /// 发送是否成功
    pub success: bool,
    /// 如果发送失败，包含错误信息
    pub error_message: Option<String>,
    /// 发送失败后的重试次数
    pub retry_count: u32,
}

impl SendResult {
    /// 创建成功结果
    pub fn success(message_id: String) -> Self {
        Self {
            message_id,
            send_time: chrono::Utc::now(),
            success: true,
            error_message: None,
            retry_count: 0,
        }
    }

    /// 创建失败结果
    pub fn failure(message_id: String, error_message: String) -> Self {
        Self {
            message_id,
            send_time: chrono::Utc::now(),
            success: false,
            error_message: Some(error_message),
            retry_count: 0,
        }
    }

    /// 创建重试结果
    pub fn retry(message_id: String, error_message: String, retry_count: u32) -> Self {
        Self {
            message_id,
            send_time: chrono::Utc::now(),
            success: false,
            error_message: Some(error_message),
            retry_count,
        }
    }
}

/// 客户端连接统计信息结构体
/// 
/// 记录客户端连接的各种统计指标
/// 包括消息传输、连接时间、重连次数和错误统计等
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    /// 已发送的消息总数
    pub messages_sent: u64,
    /// 已接收的消息总数
    pub messages_received: u64,
    /// 已发送的字节总数
    pub bytes_sent: u64,
    /// 已接收的字节总数
    pub bytes_received: u64,
    /// 连接建立以来的总时间
    pub connection_time: Duration,
    /// 最后一次数据传输的时间戳
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// 重连尝试的总次数
    pub reconnect_count: u32,
    /// 发送心跳消息的总次数
    pub heartbeat_count: u64,
    /// 发生错误的总次数
    pub error_count: u64,
}

impl Default for ConnectionStats {
    fn default() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connection_time: Duration::ZERO,
            last_activity: chrono::Utc::now(),
            reconnect_count: 0,
            heartbeat_count: 0,
            error_count: 0,
        }
    }
}

/// 协议性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
/// 协议性能指标结构体
/// 
/// 记录特定传输协议的性能测试结果
/// 用于协议竞速和性能比较
pub struct ProtocolMetrics {
    /// 被测试的协议类型
    pub protocol: TransportProtocol,
    /// 连接建立的延迟时间（毫秒）
    pub connection_latency_ms: u64,
    /// 消息传输的平均延迟时间（毫秒）
    pub message_latency_ms: u64,
    /// 消息吞吐量，每秒处理的消息数量
    pub throughput_msgs_per_sec: f64,
    /// 操作成功率，0.0 到 1.0 之间
    pub success_rate: f64,
    /// 性能测试的执行时间戳
    pub test_time: chrono::DateTime<chrono::Utc>,
}

impl ProtocolMetrics {
    /// 创建新的性能指标
    pub fn new(protocol: TransportProtocol) -> Self {
        Self {
            protocol,
            connection_latency_ms: 0,
            message_latency_ms: 0,
            throughput_msgs_per_sec: 0.0,
            success_rate: 0.0,
            test_time: chrono::Utc::now(),
        }
    }

    /// 计算成功率
    pub fn calculate_success_rate(&mut self, total_attempts: u64, successful_attempts: u64) {
        if total_attempts > 0 {
            self.success_rate = successful_attempts as f64 / total_attempts as f64;
        }
    }

    /// 计算吞吐量
    pub fn calculate_throughput(&mut self, message_count: u64, time_duration_ms: u64) {
        if time_duration_ms > 0 {
            self.throughput_msgs_per_sec = (message_count as f64 * 1000.0) / time_duration_ms as f64;
        }
    }
}

/// 消息队列项
#[derive(Debug, Clone)]
pub struct MessageQueueItem {
    /// 消息ID
    pub message_id: String,
    /// 消息
    pub message: UnifiedProtocolMessage,
    /// 目标用户ID
    pub target_user_id: String,
    /// 消息类型
    pub message_type: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 优先级
    pub priority: MessagePriority,
}

impl MessageQueueItem {
    /// 创建新的队列项
    pub fn new(
        message_id: String,
        message: UnifiedProtocolMessage,
        target_user_id: String,
        message_type: String,
        max_retries: u32,
        priority: MessagePriority,
    ) -> Self {
        Self {
            message_id,
            message,
            target_user_id,
            message_type,
            created_at: chrono::Utc::now(),
            retry_count: 0,
            max_retries,
            priority,
        }
    }

    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// 增加重试次数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// 消息优先级
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize, Ord, Eq)]
pub enum MessagePriority {
    /// 低优先级
    Low = 0,
    /// 普通优先级
    Normal = 1,
    /// 高优先级
    High = 2,
    /// 紧急优先级
    Urgent = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// 连接事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientEvent {
    /// 连接建立
    Connected(TransportProtocol),
    /// 连接断开
    Disconnected,
    /// 重连开始
    Reconnecting,
    /// 重连成功
    Reconnected(TransportProtocol),
    /// 重连失败
    ReconnectFailed,
    /// 消息接收
    MessageReceived(UnifiedProtocolMessage),
    /// 消息发送
    MessageSent(String),
    /// 消息发送失败
    MessageFailed(String, String),
    /// 心跳
    Heartbeat,
    /// 错误
    Error(String),
    /// 协议切换
    ProtocolSwitched(TransportProtocol),
}

/// 客户端事件回调
pub type ClientEventCallback = Box<dyn Fn(ClientEvent) + Send + Sync>;

/// 消息处理器
pub type MessageHandler = Box<dyn Fn(UnifiedProtocolMessage) + Send + Sync>;

/// 错误处理器
pub type ErrorHandler = Box<dyn Fn(String) + Send + Sync>; 