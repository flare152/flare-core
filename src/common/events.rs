//! 事件定义模块
//!
//! 定义常用的系统事件类型，如心跳、连接、断开、踢下线等

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::common::conn::ProtoMessage;

/// 连接事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEvent {
    pub user_id: String,
    pub session_id: String,
    pub event_type: ConnectionEventType,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// 连接事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionEventType {
    /// 连接建立
    Connected,
    /// 连接断开
    Disconnected,
    /// 消息接收
    MessageReceived(ProtoMessage),
    /// 消息发送
    MessageSent(ProtoMessage),
    /// 心跳
    Heartbeat,
    /// 错误
    Error(String),
}

/// 消息事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEvent {
    pub message_id: String,
    pub user_id: String,
    pub session_id: String,
    pub message_type: String,
    pub payload: Vec<u8>,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// 认证事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthEvent {
    pub user_id: String,
    pub session_id: String,
    pub auth_type: AuthEventType,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// 认证事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthEventType {
    /// 登录
    Login,
    /// 登出
    Logout,
    /// 令牌验证
    TokenValidation,
    /// 令牌刷新
    TokenRefresh,
}

/// 错误事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub error_type: ErrorEventType,
    pub error_message: String,
    pub error_code: Option<String>,
    pub stack_trace: Option<String>,
    pub error_time: DateTime<Utc>,
}

/// 错误事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorEventType {
    /// 认证错误
    Authentication,
    /// 授权错误
    Authorization,
    /// 网络错误
    Network,
    /// 协议错误
    Protocol,
    /// 业务错误
    Business,
    /// 系统错误
    System,
    /// 未知错误
    Unknown,
}

/// 性能事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceEvent {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub event_type: PerformanceEventType,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// 性能事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceEventType {
    /// 消息处理
    MessageProcessing,
    /// 连接建立
    ConnectionEstablishment,
    /// 认证处理
    Authentication,
    /// 心跳处理
    Heartbeat,
    /// 自定义性能事件
    Custom(String),
}

/// 系统事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub event_type: SystemEventType,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// 系统事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEventType {
    /// 服务启动
    ServiceStarted,
    /// 服务停止
    ServiceStopped,
    /// 配置更新
    ConfigUpdated,
    /// 资源清理
    ResourceCleanup,
    /// 健康检查
    HealthCheck,
    /// 自定义系统事件
    Custom(String),
} 