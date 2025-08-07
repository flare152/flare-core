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