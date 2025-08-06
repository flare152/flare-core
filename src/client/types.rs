//! 客户端类型定义模块
//!
//! 定义客户端相关的类型和结构

use crate::common::{TransportProtocol, ProtoMessage};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 客户端状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientStatus {
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

/// 发送结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    /// 消息ID
    pub message_id: String,
    /// 发送时间
    pub send_time: chrono::DateTime<chrono::Utc>,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error_message: Option<String>,
    /// 重试次数
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

/// 连接统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    /// 发送的消息数
    pub messages_sent: u64,
    /// 接收的消息数
    pub messages_received: u64,
    /// 发送的字节数
    pub bytes_sent: u64,
    /// 接收的字节数
    pub bytes_received: u64,
    /// 连接时间
    pub connection_time: Duration,
    /// 最后活动时间
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// 重连次数
    pub reconnect_count: u32,
    /// 心跳次数
    pub heartbeat_count: u64,
    /// 错误次数
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
pub struct ProtocolMetrics {
    /// 协议类型
    pub protocol: TransportProtocol,
    /// 连接延迟（毫秒）
    pub connection_latency_ms: u64,
    /// 消息延迟（毫秒）
    pub message_latency_ms: u64,
    /// 吞吐量（消息/秒）
    pub throughput_msgs_per_sec: f64,
    /// 成功率
    pub success_rate: f64,
    /// 测试时间
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
    /// 消息
    pub message: ProtoMessage,
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
        message: ProtoMessage,
        target_user_id: String,
        message_type: String,
        max_retries: u32,
        priority: MessagePriority,
    ) -> Self {
        Self {
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
    MessageReceived(ProtoMessage),
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
pub type MessageHandler = Box<dyn Fn(ProtoMessage) + Send + Sync>;

/// 错误处理器
pub type ErrorHandler = Box<dyn Fn(String) + Send + Sync>; 