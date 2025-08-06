use async_trait::async_trait;
use std::collections::HashMap;
use crate::common::{
    conn::{ProtoMessage, Platform},
    Result, TransportProtocol,
};

/// 服务端会话管理器 trait
/// 管理服务端的会话状态和连接
#[async_trait]
pub trait SessionManager: Send + Sync {
    /// 创建新会话
    async fn create_session(&self, user_id: &str, protocol: TransportProtocol) -> Result<String>;
    
    /// 验证会话
    async fn validate_session(&self, session_id: &str) -> Result<bool>;
    
    /// 获取会话信息
    async fn get_session_info(&self, session_id: &str) -> Result<Option<SessionInfo>>;
    
    /// 更新会话状态
    async fn update_session_status(&self, session_id: &str, status: SessionStatus) -> Result<()>;
    
    /// 清理过期会话
    async fn cleanup_expired_sessions(&self, timeout_secs: u64) -> Result<usize>;
    
    /// 获取活跃会话数量
    async fn get_active_session_count(&self) -> Result<usize>;
}

/// 会话信息
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: String,
    pub protocol: TransportProtocol,
    pub status: SessionStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

/// 会话状态
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Active,
    Idle,
    Suspended,
    Terminated,
}

/// 路由统计
#[derive(Debug, Clone)]
pub struct RoutingStats {
    pub total_routed_messages: u64,
    pub successful_routes: u64,
    pub failed_routes: u64,
    pub avg_routing_time_ms: u64,
    pub unreachable_users: u64,
}

/// 服务端认证管理器 trait
/// 处理用户认证和授权
#[async_trait]
pub trait AuthManager: Send + Sync {
    /// 验证访问令牌
    async fn validate_access_token(&self, token: &str) -> Result<Option<String>>;
}

/// 服务端限流管理器 trait
/// 控制消息发送频率和连接数
#[async_trait]
pub trait RateLimitManager: Send + Sync {
    /// 检查用户是否被限流
    async fn is_rate_limited(&self, user_id: &str) -> Result<bool>;
    
    /// 记录用户活动
    async fn record_activity(&self, user_id: &str) -> Result<()>;
    
    /// 获取用户限流状态
    async fn get_rate_limit_status(&self, user_id: &str) -> Result<RateLimitStatus>;
    
    /// 重置用户限流状态
    async fn reset_rate_limit(&self, user_id: &str) -> Result<()>;
}

/// 限流状态
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub is_limited: bool,
    pub remaining_requests: u32,
    pub reset_time: chrono::DateTime<chrono::Utc>,
    pub limit_type: RateLimitType,
}

/// 限流类型
#[derive(Debug, Clone)]
pub enum RateLimitType {
    MessageRate,
    ConnectionRate,
    AuthRate,
    Custom(String),
}

/// 服务端路由管理器 trait
/// 负责消息路由和分发
#[async_trait]
pub trait RoutingManager: Send + Sync {
    /// 路由消息到用户
    async fn route_message(&self, user_id: &str, message: &ProtoMessage) -> Result<RoutingResult>;
    
    /// 广播消息到多个用户
    async fn broadcast_message(&self, user_ids: &[String], message: &ProtoMessage) -> Result<BroadcastResult>;
    
    /// 获取路由统计信息
    async fn get_routing_stats(&self) -> Result<RoutingStats>;
    
    /// 检查用户是否可达
    async fn is_user_reachable(&self, user_id: &str) -> Result<bool>;
}

/// 路由结果
#[derive(Debug, Clone)]
pub struct RoutingResult {
    pub success: bool,
    pub delivered_to: Vec<String>,
    pub failed_users: Vec<String>,
    pub routing_time_ms: u64,
}

/// 广播结果
#[derive(Debug, Clone)]
pub struct BroadcastResult {
    pub total_users: usize,
    pub successful_deliveries: usize,
    pub failed_deliveries: usize,
    pub avg_routing_time_ms: u64,
}

/// 服务端监控管理器 trait
/// 提供系统监控和指标收集
#[async_trait]
pub trait MonitoringManager: Send + Sync {
    /// 获取系统状态
    async fn get_system_status(&self) -> Result<SystemStatus>;
    
    /// 获取性能指标
    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics>;
    
    /// 获取连接统计
    async fn get_connection_stats(&self) -> Result<ConnectionStats>;
    
    /// 获取错误统计
    async fn get_error_stats(&self) -> Result<ErrorStats>;
}

/// 系统状态
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub uptime_secs: u64,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
    pub active_connections: usize,
    pub total_requests: u64,
    pub error_rate: f64,
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub avg_response_time_ms: u64,
    pub requests_per_second: f64,
    pub throughput_mbps: f64,
    pub connection_latency_ms: u64,
    pub message_processing_time_ms: u64,
}

/// 连接统计
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub quic_connections: usize,
    pub websocket_connections: usize,
    pub avg_heartbeat_interval_ms: u64,
}

/// 错误统计
#[derive(Debug, Clone)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub auth_errors: u64,
    pub network_errors: u64,
    pub protocol_errors: u64,
    pub business_errors: u64,
    pub error_rate: f64,
} 