use async_trait::async_trait;
use crate::{TransportProtocol, Result, FlareError};
use crate::common::types::ConnectionInfo;

/// 连接管理器 trait
/// 负责管理所有客户端连接
#[async_trait]
pub trait ConnectionManager: Send + Sync {
    /// 添加新连接
    async fn add_connection(&self, info: ConnectionInfo) -> Result<()>;
    
    /// 移除连接
    async fn remove_connection(&self, user_id: &str, session_id: &str) -> Result<()>;
    
    /// 获取用户连接信息
    async fn get_connection(&self, user_id: &str) -> Result<Option<ConnectionInfo>>;
    
    /// 获取用户所有会话
    async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<ConnectionInfo>>;
    
    /// 检查用户是否在线
    async fn is_user_online(&self, user_id: &str) -> Result<bool>;
    
    /// 获取在线用户数量
    async fn get_online_count(&self) -> Result<usize>;
    
    /// 清理过期连接
    async fn cleanup_expired_connections(&self, timeout_secs: u64) -> Result<usize>;
    
    /// 更新心跳时间
    async fn update_heartbeat(&self, user_id: &str, session_id: &str) -> Result<()>;
}

/// 连接统计信息
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub quic_connections: usize,
    pub websocket_connections: usize,
    pub avg_heartbeat_interval_ms: u64,
}

/// 消息处理器 trait
/// 负责处理消息的路由和分发
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// 处理接收到的消息
    async fn handle_message(&self, message: crate::common::protocol::Message) -> Result<()>;
    
    /// 广播消息给多个用户
    async fn broadcast_message(&self, message: crate::common::protocol::Message, user_ids: Vec<String>) -> Result<()>;
}

/// 事件监听器 trait
/// 用于监听系统事件
#[async_trait]
pub trait EventListener: Send + Sync {
    /// 用户连接事件
    async fn on_user_connected(&self, user_id: &str, session_id: &str) -> Result<()>;
    
    /// 用户断开事件
    async fn on_user_disconnected(&self, user_id: &str, session_id: &str) -> Result<()>;
    
    /// 消息接收事件
    async fn on_message_received(&self, message: &crate::common::protocol::Message) -> Result<()>;
    
    /// 协议切换事件
    async fn on_protocol_switched(&self, from: TransportProtocol, to: TransportProtocol) -> Result<()>;
    
    /// 错误事件
    async fn on_error(&self, error: &FlareError) -> Result<()>;
}

/// 认证处理器 trait
/// 处理用户认证
#[async_trait]
pub trait AuthHandler: Send + Sync {
    /// 验证用户认证
    async fn authenticate(&self, user_id: &str, token: &str) -> Result<bool>;
    
    /// 生成认证令牌
    async fn generate_token(&self, user_id: &str) -> Result<String>;
    
    /// 验证令牌
    async fn validate_token(&self, token: &str) -> Result<Option<String>>;
} 