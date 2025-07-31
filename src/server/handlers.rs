//! Flare IM 服务端处理器模块
//!
//! 提供认证、消息处理、事件处理等处理器接口和默认实现

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use async_trait::async_trait;
use tracing::{info, error, debug};
use uuid::Uuid;

use crate::common::{
    conn::{ConnectionEvent, ProtoMessage, Platform},
    Result,
};

/// 认证处理器trait
#[async_trait]
pub trait AuthHandler: Send + Sync {
    /// 验证令牌
    async fn validate_token(&self, token: &str) -> Result<Option<String>>;
}

/// 消息处理器trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// 处理消息
    async fn handle_message(&self, user_id: &str, message: ProtoMessage) -> Result<ProtoMessage>;
    
    /// 处理用户连接
    async fn handle_user_connect(&self, user_id: &str, session_id: &str, platform: Platform) -> Result<()>;
    
    /// 处理用户断开
    async fn handle_user_disconnect(&self, user_id: &str, session_id: &str) -> Result<()>;
    
    /// 处理心跳
    async fn handle_heartbeat(&self, user_id: &str, session_id: &str) -> Result<()>;
}

/// 事件处理器trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// 处理连接事件
    async fn handle_connection_event(&self, user_id: &str, event: ConnectionEvent) -> Result<()>;
    
    /// 处理错误
    async fn handle_error(&self, error: &str) -> Result<()>;
}

/// 默认认证处理器
pub struct DefaultAuthHandler {
    tokens: Arc<RwLock<HashMap<String, TokenInfo>>>, // token -> TokenInfo
}

/// 令牌信息
#[derive(Debug, Clone)]
struct TokenInfo {
    user_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl DefaultAuthHandler {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 生成令牌
    pub async fn generate_token(&self, user_id: &str) -> Result<String> {
        let token = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::hours(24); // 24小时过期
        
        let token_info = TokenInfo {
            user_id: user_id.to_string(),
            created_at: now,
            expires_at,
        };
        
        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(token.clone(), token_info);
        }
        
        info!("为用户 {} 生成令牌", user_id);
        Ok(token)
    }
    
    /// 清理过期令牌
    async fn cleanup_expired_tokens(&self) -> Result<usize> {
        let now = chrono::Utc::now();
        let mut tokens = self.tokens.write().await;
        
        let expired_tokens: Vec<String> = tokens
            .iter()
            .filter(|(_, info)| info.expires_at < now)
            .map(|(token, _)| token.clone())
            .collect();
        
        for token in &expired_tokens {
            tokens.remove(token);
        }
        
        if !expired_tokens.is_empty() {
            debug!("清理了 {} 个过期令牌", expired_tokens.len());
        }
        
        Ok(expired_tokens.len())
    }
}

#[async_trait]
impl AuthHandler for DefaultAuthHandler {
    async fn validate_token(&self, token: &str) -> Result<Option<String>> {
        // 清理过期令牌
        self.cleanup_expired_tokens().await?;
        
        let tokens = self.tokens.read().await;
        if let Some(token_info) = tokens.get(token) {
            if token_info.expires_at > chrono::Utc::now() {
                debug!("令牌验证成功: 用户 {}", token_info.user_id);
                return Ok(Some(token_info.user_id.clone()));
            }
        }
        
        debug!("令牌验证失败: {}", token);
        Ok(None)
    }
}

impl Default for DefaultAuthHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 默认消息处理器
pub struct DefaultMessageHandler {
    message_count: Arc<RwLock<u64>>,
}

impl DefaultMessageHandler {
    pub fn new() -> Self {
        Self {
            message_count: Arc::new(RwLock::new(0)),
        }
    }
}

#[async_trait]
impl MessageHandler for DefaultMessageHandler {
    async fn handle_message(&self, user_id: &str, message: ProtoMessage) -> Result<ProtoMessage> {
        // 更新消息计数
        {
            let mut count = self.message_count.write().await;
            *count += 1;
        }
        
        info!("处理消息: 用户 {} 类型 {} 长度 {}", user_id, message.message_type, message.payload.len());
        
        // 返回echo响应
        Ok(ProtoMessage::new(
            Uuid::new_v4().to_string(),
            "echo".to_string(),
            message.payload,
        ))
    }
    
    async fn handle_user_connect(&self, user_id: &str, session_id: &str, platform: Platform) -> Result<()> {
        info!("用户连接: {} 会话 {} 平台 {:?}", user_id, session_id, platform);
        Ok(())
    }
    
    async fn handle_user_disconnect(&self, user_id: &str, session_id: &str) -> Result<()> {
        info!("用户断开: {} 会话 {}", user_id, session_id);
        Ok(())
    }
    
    async fn handle_heartbeat(&self, user_id: &str, session_id: &str) -> Result<()> {
        debug!("用户心跳: {} 会话 {}", user_id, session_id);
        Ok(())
    }
}

impl Default for DefaultMessageHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 默认事件处理器
pub struct DefaultEventHandler {
    event_count: Arc<RwLock<u64>>,
}

impl DefaultEventHandler {
    pub fn new() -> Self {
        Self {
            event_count: Arc::new(RwLock::new(0)),
        }
    }
}

#[async_trait]
impl EventHandler for DefaultEventHandler {
    async fn handle_connection_event(&self, user_id: &str, event: ConnectionEvent) -> Result<()> {
        // 更新事件计数
        {
            let mut count = self.event_count.write().await;
            *count += 1;
        }
        
        match event {
            ConnectionEvent::Connected => {
                info!("连接事件: 用户 {} 已连接", user_id);
            }
            ConnectionEvent::Disconnected => {
                info!("连接事件: 用户 {} 已断开", user_id);
            }
            ConnectionEvent::Reconnecting => {
                debug!("连接事件: 用户 {} 重连中", user_id);
            }
            ConnectionEvent::Failed(reason) => {
                error!("连接事件: 用户 {} 连接失败: {}", user_id, reason);
            }
            ConnectionEvent::MessageReceived(msg) => {
                debug!("连接事件: 用户 {} 收到消息 类型 {}", user_id, msg.message_type);
            }
            ConnectionEvent::MessageSent(msg) => {
                debug!("连接事件: 用户 {} 发送消息 类型 {}", user_id, msg.message_type);
            }
            ConnectionEvent::Heartbeat => {
                debug!("连接事件: 用户 {} 心跳", user_id);
            }
            ConnectionEvent::Error(error_msg) => {
                error!("连接事件: 用户 {} 错误: {}", user_id, error_msg);
            }
        }
        
        Ok(())
    }
    
    async fn handle_error(&self, error: &str) -> Result<()> {
        error!("事件处理器错误: {}", error);
        Ok(())
    }
}

impl Default for DefaultEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// JWT认证处理器
pub struct JwtAuthHandler {
    secret: String,
    expiry_secs: u64,
}

impl JwtAuthHandler {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            expiry_secs: 3600, // 1小时
        }
    }
    
    pub fn with_expiry(mut self, expiry_secs: u64) -> Self {
        self.expiry_secs = expiry_secs;
        self
    }
}

#[async_trait]
impl AuthHandler for JwtAuthHandler {
    async fn validate_token(&self, token: &str) -> Result<Option<String>> {
        // 这里应该使用实际的JWT库来验证令牌
        // 为了简化，这里只是示例实现
        if token.starts_with("jwt_") {
            let user_id = token.trim_start_matches("jwt_");
            if !user_id.is_empty() {
                debug!("JWT令牌验证成功: 用户 {}", user_id);
                return Ok(Some(user_id.to_string()));
            }
        }
        
        debug!("JWT令牌验证失败: {}", token);
        Ok(None)
    }
} 