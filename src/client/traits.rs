use async_trait::async_trait;
use std::collections::HashMap;
use crate::common::{
    Message, Result, FlareError, TransportProtocol,
    ConnectionInfo
};

/// 客户端连接管理器 trait
/// 管理客户端与服务端的连接
#[async_trait]
pub trait ClientConnectionManager: Send + Sync {
    /// 连接到服务器
    async fn connect(&mut self, server_url: &str) -> Result<()>;
    
    /// 断开连接
    async fn disconnect(&mut self) -> Result<()>;
    
    /// 检查连接状态
    async fn is_connected(&self) -> bool;
    
    /// 重新连接
    async fn reconnect(&mut self) -> Result<()>;
    
    /// 获取连接信息
    async fn get_connection_info(&self) -> Result<Option<ConnectionInfo>>;
    
    /// 发送心跳
    async fn send_heartbeat(&self) -> Result<()>;
    
    /// 设置连接状态回调
    fn set_connection_callback(&mut self, callback: Box<dyn Fn(crate::common::ConnectionStatus) + Send + Sync>);
}

/// 客户端消息发送器 trait
/// 处理消息的发送和接收
#[async_trait]
pub trait MessageSender: Send + Sync {
    /// 发送消息
    async fn send_message(&self, to_user: &str, content: &str) -> Result<Message>;
    
    /// 发送消息到群组
    async fn send_group_message(&self, group_id: &str, content: &str) -> Result<Message>;
    
    /// 发送消息并等待确认
    async fn send_message_with_ack(&self, to_user: &str, content: &str) -> Result<Message>;
    
    /// 设置消息接收回调
    fn set_message_callback(&mut self, callback: Box<dyn Fn(Message) + Send + Sync>);
    
    /// 获取消息发送统计
    async fn get_send_stats(&self) -> Result<SendStats>;
}

/// 发送统计
#[derive(Debug, Clone)]
pub struct SendStats {
    pub total_sent: u64,
    pub successful_sends: u64,
    pub failed_sends: u64,
    pub avg_send_time_ms: u64,
    pub last_sent_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 客户端协议竞速器 trait
/// 管理协议选择和切换
#[async_trait]
pub trait ProtocolRacer: Send + Sync {
    /// 执行协议竞速
    async fn race_protocols(&mut self, server_url: &str) -> Result<TransportProtocol>;
    
    /// 切换协议
    async fn switch_protocol(&mut self, protocol: TransportProtocol) -> Result<()>;
    
    /// 获取当前协议
    fn get_current_protocol(&self) -> TransportProtocol;
    
    /// 获取协议性能指标
    async fn get_protocol_metrics(&self) -> Result<ProtocolMetrics>;
    
    /// 设置协议切换回调
    fn set_protocol_change_callback(&mut self, callback: Box<dyn Fn(TransportProtocol, TransportProtocol) + Send + Sync>);
    
    /// 启用自动协议切换
    fn enable_auto_switch(&mut self, enabled: bool);
    
    /// 获取协议竞速统计
    async fn get_racing_stats(&self) -> Result<RacingStats>;
}

/// 协议性能指标
#[derive(Debug, Clone)]
pub struct ProtocolMetrics {
    pub latency_ms: u64,
    pub throughput_mbps: f64,
    pub connection_stability: f64,
    pub error_rate: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 竞速统计
#[derive(Debug, Clone)]
pub struct RacingStats {
    pub quic_attempts: u64,
    pub websocket_attempts: u64,
    pub quic_successes: u64,
    pub websocket_successes: u64,
    pub avg_quic_connection_time_ms: u64,
    pub avg_websocket_connection_time_ms: u64,
    pub protocol_switches: u64,
}

/// 用户状态
#[derive(Debug, Clone)]
pub struct UserState {
    pub user_id: String,
    pub status: UserStatus,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

/// 用户状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum UserStatus {
    Online,
    Offline,
    Away,
    Busy,
    DoNotDisturb,
}

/// 客户端状态管理器 trait
/// 管理客户端本地状态
#[async_trait]
pub trait ClientStateManager: Send + Sync {
    /// 设置用户状态
    async fn set_user_state(&self, state: UserState) -> Result<()>;
    
    /// 获取用户状态
    async fn get_user_state(&self) -> Result<Option<UserState>>;
    
    /// 更新用户状态
    async fn update_user_state(&self, status: UserStatus) -> Result<()>;
    
    /// 设置状态变更回调
    fn set_state_change_callback(&mut self, callback: Box<dyn Fn(UserState) + Send + Sync>);
    
    /// 获取状态历史
    async fn get_state_history(&self, limit: Option<usize>) -> Result<Vec<UserState>>;
}

/// 客户端缓存管理器 trait
/// 管理本地消息和状态缓存
#[async_trait]
pub trait CacheManager: Send + Sync {
    /// 缓存消息
    async fn cache_message(&self, message: &Message) -> Result<()>;
    
    /// 获取缓存的消息
    async fn get_cached_messages(&self, user_id: &str, limit: Option<usize>) -> Result<Vec<Message>>;
    
    /// 清理过期缓存
    async fn cleanup_expired_cache(&self, max_age_hours: u64) -> Result<usize>;
    
    /// 获取缓存统计
    async fn get_cache_stats(&self) -> Result<CacheStats>;
    
    /// 清空缓存
    async fn clear_cache(&self) -> Result<()>;
}

/// 缓存统计
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_cached_messages: usize,
    pub cache_size_bytes: usize,
    pub cache_hit_rate: f64,
    pub oldest_message_age_hours: u64,
    pub last_cleanup: Option<chrono::DateTime<chrono::Utc>>,
}

/// 客户端重连管理器 trait
/// 管理连接断开和重连
#[async_trait]
pub trait ReconnectionManager: Send + Sync {
    /// 启用自动重连
    fn enable_auto_reconnect(&mut self, enabled: bool);
    
    /// 设置重连策略
    fn set_reconnection_strategy(&mut self, strategy: ReconnectionStrategy);
    
    /// 执行重连
    async fn reconnect(&mut self) -> Result<()>;
    
    /// 获取重连统计
    async fn get_reconnection_stats(&self) -> Result<ReconnectionStats>;
    
    /// 设置重连回调
    fn set_reconnection_callback(&mut self, callback: Box<dyn Fn(ReconnectionEvent) + Send + Sync>);
}

/// 重连策略
#[derive(Debug, Clone)]
pub struct ReconnectionStrategy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter_enabled: bool,
}

/// 重连事件
#[derive(Debug, Clone)]
pub enum ReconnectionEvent {
    Started,
    Attempting { attempt: u32, delay_ms: u64 },
    Succeeded,
    Failed { reason: String },
    Aborted,
}

/// 重连统计
#[derive(Debug, Clone)]
pub struct ReconnectionStats {
    pub total_reconnections: u64,
    pub successful_reconnections: u64,
    pub failed_reconnections: u64,
    pub avg_reconnection_time_ms: u64,
    pub last_reconnection: Option<chrono::DateTime<chrono::Utc>>,
}

/// 客户端网络监控器 trait
/// 监控网络状态和性能
#[async_trait]
pub trait NetworkMonitor: Send + Sync {
    /// 开始网络监控
    async fn start_monitoring(&mut self) -> Result<()>;
    
    /// 停止网络监控
    async fn stop_monitoring(&mut self) -> Result<()>;
    
    /// 获取网络状态
    async fn get_network_status(&self) -> Result<NetworkStatus>;
    
    /// 获取网络性能指标
    async fn get_network_metrics(&self) -> Result<NetworkMetrics>;
    
    /// 设置网络状态变更回调
    fn set_network_change_callback(&mut self, callback: Box<dyn Fn(NetworkStatus) + Send + Sync>);
}

/// 网络状态
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkStatus {
    Connected,
    Disconnected,
    Poor,
    Excellent,
}

/// 网络指标
#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    pub latency_ms: u64,
    pub bandwidth_bps: f64,
    pub packet_loss_rate: f64,
    pub connection_quality: f64, // 0.0 to 1.0
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 客户端错误处理器 trait
/// 处理客户端错误和异常
#[async_trait]
pub trait ErrorHandler: Send + Sync {
    /// 处理错误
    async fn handle_error(&self, error: &FlareError) -> Result<()>;
    
    /// 获取错误统计
    async fn get_error_stats(&self) -> Result<ErrorStats>;
    
    /// 设置错误回调
    fn set_error_callback(&mut self, callback: Box<dyn Fn(FlareError) + Send + Sync>);
    
    /// 清除错误历史
    async fn clear_error_history(&self) -> Result<()>;
}

/// 错误统计
#[derive(Debug)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub errors_by_type: HashMap<String, u64>,
    pub last_error: Option<String>, // 存储错误消息而不是FlareError
    pub error_rate: f64, // errors per minute
    pub timestamp: chrono::DateTime<chrono::Utc>,
} 