//! Flare IM 服务端连接管理器模块
//!
//! 提供连接管理器的trait定义和基于内存的实现

use std::collections::HashMap;
use async_trait::async_trait;

use crate::common::{conn::Connection, Result, TransportProtocol, types::Platform, UnifiedProtocolMessage, ConnectionStatus};

/// 连接管理器配置
#[derive(Debug, Clone)]
pub struct ServerConnectionManagerConfig {
    /// 最大连接数
    pub max_connections: usize,
    /// 连接超时时间（毫秒）
    pub connection_timeout_ms: u64,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 心跳超时时间（毫秒）
    pub heartbeat_timeout_ms: u64,
    /// 最大心跳丢失次数
    pub max_missed_heartbeats: u64,
    /// 清理间隔（毫秒）
    pub cleanup_interval_ms: u64,
    /// 是否启用自动重连
    pub enable_auto_reconnect: bool,
    /// 最大重连次数
    pub max_reconnect_attempts: u32,
    /// 重连延迟（毫秒）
    pub reconnect_delay_ms: u64,
}

impl Default for ServerConnectionManagerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1_000_000, // 百万级连接
            connection_timeout_ms: 300_000, // 5分钟
            heartbeat_interval_ms: 30_000, // 30秒
            heartbeat_timeout_ms: 60_000, // 60秒
            max_missed_heartbeats: 3,
            cleanup_interval_ms: 60_000, // 1分钟
            enable_auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_delay_ms: 1_000, // 1秒
        }
    }
}

/// 连接信息
#[derive(Debug, Clone)]
pub struct ServerConnectionInfo {
    /// 连接ID
    pub id: String,
    /// 用户ID
    pub user_id: String,
    /// 会话ID
    pub session_id: String,
    /// 远程地址
    pub remote_addr: String,
    /// 平台
    pub platform: Platform,
    /// 协议
    pub protocol: TransportProtocol,
    /// 连接状态
    pub state: ConnectionStatus,
    /// 连接时间
    pub connected_at: chrono::DateTime<chrono::Utc>,
    /// 最后活动时间
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// 最后心跳时间
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    /// 心跳计数
    pub heartbeat_count: u64,
    /// 丢失心跳次数
    pub missed_heartbeats: u64,
    /// 重连次数
    pub reconnect_count: u32,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl ServerConnectionInfo {
    /// 创建新的连接信息
    pub fn new(
        id: String,
        user_id: String,
        session_id: String,
        remote_addr: String,
        platform: Platform,
        protocol: TransportProtocol,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            user_id,
            session_id,
            remote_addr,
            platform,
            protocol,
            state: ConnectionStatus::Connected,
            connected_at: now,
            last_activity: now,
            last_heartbeat: now,
            heartbeat_count: 0,
            missed_heartbeats: 0,
            reconnect_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// 检查连接是否健康
    pub fn is_healthy(&self, config: &ServerConnectionManagerConfig) -> bool {
        let now = chrono::Utc::now();
        let time_since_heartbeat = now.signed_duration_since(self.last_heartbeat);
        let timeout_duration = chrono::Duration::milliseconds(config.heartbeat_timeout_ms as i64);
        
        time_since_heartbeat < timeout_duration && 
        self.missed_heartbeats < config.max_missed_heartbeats
    }

    /// 更新心跳
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = chrono::Utc::now();
        self.last_activity = chrono::Utc::now();
        self.heartbeat_count += 1;
        self.missed_heartbeats = 0;
    }

    /// 标记心跳丢失
    pub fn mark_heartbeat_missed(&mut self) {
        self.missed_heartbeats += 1;
    }

    /// 更新活动时间
    pub fn update_activity(&mut self) {
        self.last_activity = chrono::Utc::now();
    }
}

/// 连接管理器统计信息
#[derive(Debug, Clone)]
pub struct ServerConnectionManagerStats {
    /// 总连接数
    pub total_connections: usize,
    /// 活跃连接数
    pub active_connections: usize,
    /// 断开连接数
    pub disconnected_connections: usize,
    /// 在线用户数
    pub online_users: usize,
    /// 总消息数
    pub total_messages: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for ServerConnectionManagerStats {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            disconnected_connections: 0,
            online_users: 0,
            total_messages: 0,
            total_bytes: 0,
            last_updated: chrono::Utc::now(),
        }
    }
}

/// 连接事件回调
pub type ConnectionEventCallback = Box<dyn Fn(String, String) + Send + Sync>;

/// 服务端连接管理器trait
/// 
/// 管理所有客户端连接，提供连接生命周期管理、心跳处理、消息路由等功能
#[async_trait]
pub trait ServerConnectionManager: Send + Sync {
    /// 启动连接管理器
    async fn start(&self) -> Result<()>;
    
    /// 停止连接管理器
    async fn stop(&self) -> Result<()>;
    
    /// 添加连接
    /// 
    /// # 参数
    /// * `connection` - 连接实例
    /// * `user_id` - 用户ID
    /// * `session_id` - 会话ID
    /// 
    /// # 返回
    /// * `Result<()>` - 操作结果
    async fn add_connection(
        &self,
        connection: Box<dyn Connection>,
        user_id: String,
        session_id: String,
    ) -> Result<()>;
    
    /// 移除连接
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// * `session_id` - 会话ID
    /// 
    /// # 返回
    /// * `Result<()>` - 操作结果
    async fn remove_connection(&self, user_id: &str, session_id: &str) -> Result<()>;
    
    /// 获取连接
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// * `session_id` - 会话ID
    /// 
    /// # 返回
    /// * `Result<Option<Box<dyn Connection>>>` - 连接实例
    async fn get_connection(&self, user_id: &str, session_id: &str) -> Result<Option<Box<dyn Connection>>>;
    
    /// 获取用户的所有连接
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// 
    /// # 返回
    /// * `Result<Vec<Box<dyn Connection>>>` - 连接列表
    async fn get_user_connections(&self, user_id: &str) -> Result<Vec<Box<dyn Connection>>>;
    
    /// 检查用户是否在线
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// 
    /// # 返回
    /// * `Result<bool>` - 是否在线
    async fn is_user_online(&self, user_id: &str) -> Result<bool>;
    
    /// 获取在线用户数量
    /// 
    /// # 返回
    /// * `Result<usize>` - 在线用户数
    async fn get_online_user_count(&self) -> Result<usize>;
    
    /// 获取连接数量
    /// 
    /// # 返回
    /// * `Result<usize>` - 连接数
    async fn get_connection_count(&self) -> Result<usize>;
    
    /// 发送消息给用户
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// * `message` - 消息内容
    /// 
    /// # 返回
    /// * `Result<usize>` - 成功发送的连接数
    async fn send_message_to_user(&self, user_id: &str, message: UnifiedProtocolMessage) -> Result<usize>;
    
    /// 发送消息给指定会话
    /// 
    /// # 参数
    /// * `session_id` - 会话ID
    /// * `message` - 消息内容
    /// 
    /// # 返回
    /// * `Result<bool>` - 是否发送成功
    async fn send_message_to_session(&self, session_id: &str, message: UnifiedProtocolMessage) -> Result<bool>;
    
    /// 广播消息给所有用户
    /// 
    /// # 参数
    /// * `message` - 消息内容
    /// 
    /// # 返回
    /// * `Result<usize>` - 成功发送的连接数
    async fn broadcast_message(&self, message: UnifiedProtocolMessage) -> Result<usize>;
    
    /// 更新心跳
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// * `session_id` - 会话ID
    /// 
    /// # 返回
    /// * `Result<()>` - 操作结果
    async fn update_heartbeat(&self, user_id: &str, session_id: &str) -> Result<()>;
    
    /// 清理过期连接
    /// 
    /// # 参数
    /// * `timeout_secs` - 超时时间（秒）
    /// 
    /// # 返回
    /// * `Result<usize>` - 清理的连接数
    async fn cleanup_expired_connections(&self, timeout_secs: u64) -> Result<usize>;
    
    /// 强制断开用户连接
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// * `reason` - 断开原因
    /// 
    /// # 返回
    /// * `Result<usize>` - 断开的连接数
    async fn force_disconnect_user(&self, user_id: &str, reason: Option<String>) -> Result<usize>;
    
    /// 获取连接信息
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// * `session_id` - 会话ID
    /// 
    /// # 返回
    /// * `Result<Option<ServerConnectionInfo>>` - 连接信息
    async fn get_connection_info(&self, user_id: &str, session_id: &str) -> Result<Option<ServerConnectionInfo>>;
    
    /// 获取用户的所有连接信息
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// 
    /// # 返回
    /// * `Result<Vec<ServerConnectionInfo>>` - 连接信息列表
    async fn get_user_connection_infos(&self, user_id: &str) -> Result<Vec<ServerConnectionInfo>>;
    
    /// 获取管理器统计信息
    /// 
    /// # 返回
    /// * `Result<ServerConnectionManagerStats>` - 统计信息
    async fn get_stats(&self) -> Result<ServerConnectionManagerStats>;
    
    /// 设置连接事件回调
    /// 
    /// # 参数
    /// * `callback` - 事件回调函数
    async fn set_connection_event_callback(&self, callback: ConnectionEventCallback);
    
    /// 获取连接管理器配置
    /// 
    /// # 返回
    /// * `ServerConnectionManagerConfig` - 配置信息
    fn get_config(&self) -> &ServerConnectionManagerConfig;
    
    /// 获取所有连接
    /// 
    /// # 返回
    /// * `Result<Vec<(String, ServerConnectionInfo)>>` - 所有连接信息
    async fn get_all_connections(&self) -> Result<Vec<(String, ServerConnectionInfo)>>;
}

// 导出子模块
pub mod memory;

// 重新导出公共类型
pub use memory::MemoryServerConnectionManager; 