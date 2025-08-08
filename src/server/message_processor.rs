//! Flare IM 消息处理器模块
//!
//! 提供丰富的消息发送功能，支持各种消息类型

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::common::{
    protocol::UnifiedProtocolMessage,
    Result, FlareError,
};
use crate::server::conn_manager::ServerConnectionManager;

/// 消息处理器
/// 
/// 提供丰富的消息发送功能，支持各种消息类型
pub struct MessageProcessor<CM>
where
    CM: ServerConnectionManager + 'static,
{
    connection_manager: Arc<CM>,
    message_stats: Arc<RwLock<MessageStats>>,
}

/// 消息统计信息
#[derive(Debug, Clone)]
pub struct MessageStats {
    pub total_messages_sent: u64,
    pub text_messages_sent: u64,
    pub binary_messages_sent: u64,
    pub notifications_sent: u64,
    pub errors_sent: u64,
    pub custom_messages_sent: u64,
    pub rest_responses_sent: u64,
    pub broadcast_messages_sent: u64,
}

impl Default for MessageStats {
    fn default() -> Self {
        Self {
            total_messages_sent: 0,
            text_messages_sent: 0,
            binary_messages_sent: 0,
            notifications_sent: 0,
            errors_sent: 0,
            custom_messages_sent: 0,
            rest_responses_sent: 0,
            broadcast_messages_sent: 0,
        }
    }
}

impl<CM> MessageProcessor<CM>
where
    CM: ServerConnectionManager + 'static,
{
    /// 创建新的消息处理器
    pub fn new(connection_manager: Arc<CM>) -> Self {
        Self {
            connection_manager,
            message_stats: Arc::new(RwLock::new(MessageStats::default())),
        }
    }
    
    /// 获取消息统计信息
    pub async fn get_stats(&self) -> MessageStats {
        self.message_stats.read().await.clone()
    }
    
    /// 重置统计信息
    pub async fn reset_stats(&self) {
        let mut stats = self.message_stats.write().await;
        *stats = MessageStats::default();
    }
    
    // ==================== 基础消息发送 ====================
    
    /// 发送消息给指定用户
    pub async fn send(&self, user_id: &str, message: UnifiedProtocolMessage) -> Result<bool> {
        self.send_message_to_user(user_id, message).await
    }
    
    /// 广播消息给所有用户
    pub async fn broadcast(&self, message: UnifiedProtocolMessage) -> Result<usize> {
        self.broadcast_message(message).await
    }
    
    /// 发送消息给多个用户
    pub async fn send_to_multiple(&self, user_ids: &[String], message: UnifiedProtocolMessage) -> Result<usize> {
        let mut success_count = 0;
        for user_id in user_ids {
            if self.send_message_to_user(user_id, message.clone()).await? {
                success_count += 1;
            }
        }
        Ok(success_count)
    }
    
    // ==================== 文本消息 ====================
    
    /// 发送文本消息
    pub async fn send_text(&self, user_id: &str, text: String) -> Result<bool> {
        let message = UnifiedProtocolMessage::text(text);
        self.send_message_to_user(user_id, message).await
    }
    
    /// 广播文本消息
    pub async fn broadcast_text(&self, text: String) -> Result<usize> {
        let message = UnifiedProtocolMessage::text(text);
        self.broadcast_message(message).await
    }
    
    /// 发送文本消息给多个用户
    pub async fn send_text_to_multiple(&self, user_ids: &[String], text: String) -> Result<usize> {
        let message = UnifiedProtocolMessage::text(text);
        self.send_to_multiple(user_ids, message).await
    }
    
    // ==================== 二进制消息 ====================
    
    /// 发送二进制消息
    pub async fn send_binary(&self, user_id: &str, data: Vec<u8>) -> Result<bool> {
        let message = UnifiedProtocolMessage::binary(data);
        self.send_message_to_user(user_id, message).await
    }
    
    /// 广播二进制消息
    pub async fn broadcast_binary(&self, data: Vec<u8>) -> Result<usize> {
        let message = UnifiedProtocolMessage::binary(data);
        self.broadcast_message(message).await
    }
    
    // ==================== 通知消息 ====================
    
    /// 发送通知
    pub async fn send_notification(&self, user_id: &str, title: String, content: String) -> Result<bool> {
        let message = UnifiedProtocolMessage::notification_text(title, content);
        self.send_message_to_user(user_id, message).await
    }
    
    /// 广播通知
    pub async fn broadcast_notification(&self, title: String, content: String) -> Result<usize> {
        let message = UnifiedProtocolMessage::notification_text(title, content);
        self.broadcast_message(message).await
    }
    
    /// 发送通知给多个用户
    pub async fn send_notification_to_multiple(&self, user_ids: &[String], title: String, content: String) -> Result<usize> {
        let message = UnifiedProtocolMessage::notification_text(title, content);
        self.send_to_multiple(user_ids, message).await
    }
    
    // ==================== 错误消息 ====================
    
    /// 发送错误消息
    pub async fn send_error(&self, user_id: &str, code: String, message: String) -> Result<bool> {
        let error_msg = UnifiedProtocolMessage::error(code, message);
        self.send_message_to_user(user_id, error_msg).await
    }
    
    /// 广播错误消息
    pub async fn broadcast_error(&self, code: String, message: String) -> Result<usize> {
        let error_msg = UnifiedProtocolMessage::error(code, message);
        self.broadcast_message(error_msg).await
    }
    
    // ==================== 自定义消息 ====================
    
    /// 发送自定义消息
    pub async fn send_custom(&self, user_id: &str, message_type: String, data: Vec<u8>) -> Result<bool> {
        let message = UnifiedProtocolMessage::custom_message(message_type, data);
        self.send_message_to_user(user_id, message).await
    }
    
    /// 广播自定义消息
    pub async fn broadcast_custom(&self, message_type: String, data: Vec<u8>) -> Result<usize> {
        let message = UnifiedProtocolMessage::custom_message(message_type, data);
        self.broadcast_message(message).await
    }
    
    // ==================== 自定义事件 ====================
    
    /// 发送自定义事件
    pub async fn send_event(&self, user_id: &str, event_name: String, data: Vec<u8>) -> Result<bool> {
        let message = UnifiedProtocolMessage::custom_event(event_name, data);
        self.send_message_to_user(user_id, message).await
    }
    
    /// 广播自定义事件
    pub async fn broadcast_event(&self, event_name: String, data: Vec<u8>) -> Result<usize> {
        let message = UnifiedProtocolMessage::custom_event(event_name, data);
        self.broadcast_message(message).await
    }
    
    // ==================== REST 响应 ====================
    
    /// 发送 REST 响应
    pub async fn send_res(&self, session_id: &str, id: String, status_code: u16, body: Vec<u8>) -> Result<bool> {
        let message = UnifiedProtocolMessage::rest_response(id, status_code, body);
        self.send_message_to_session(session_id, message).await
    }
    
    /// 发送 JSON REST 响应
    pub async fn send_rest_json(&self, session_id: &str, id: String, status_code: u16, json_body: serde_json::Value) -> Result<bool> {
        let message = UnifiedProtocolMessage::rest_response_json(id, status_code, json_body);
        self.send_message_to_session(session_id, message).await
    }
    
    // ==================== 系统消息 ====================
    
    /// 发送心跳确认
    pub async fn send_heartbeat_ack(&self, user_id: &str) -> Result<bool> {
        let message = UnifiedProtocolMessage::heartbeat_ack();
        self.send_message_to_user(user_id, message).await
    }
    
    /// 发送连接事件
    pub async fn send_connect_event(&self, user_id: &str, session_id: &str) -> Result<bool> {
        let message = UnifiedProtocolMessage::connect(user_id.to_string(), session_id.to_string());
        self.send_message_to_user(user_id, message).await
    }
    
    /// 发送断开连接事件
    pub async fn send_disconnect_event(&self, user_id: &str, session_id: &str, reason: String) -> Result<bool> {
        let message = UnifiedProtocolMessage::disconnect(user_id.to_string(), session_id.to_string(), reason);
        self.send_message_to_user(user_id, message).await
    }
    
    // ==================== 内部方法 ====================
    
    /// 发送消息给指定用户（内部方法）
    async fn send_message_to_user(&self, user_id: &str, message: UnifiedProtocolMessage) -> Result<bool> {
        // 获取用户连接
        let connection = self.connection_manager.get_connection(user_id, "").await?;
        if let Some(conn) = connection {
            // 克隆消息用于统计
            let message_for_stats = message.clone();
            match conn.send(message).await {
                Ok(_) => {
                    self.update_stats(&message_for_stats).await;
                    debug!("发送消息成功: 用户 {}", user_id);
                    Ok(true)
                }
                Err(e) => {
                    error!("发送消息失败: 用户 {} 错误: {}", user_id, e);
                    Ok(false)
                }
            }
        } else {
            warn!("用户未连接: {}", user_id);
            Ok(false)
        }
    }
    
    /// 发送消息给指定会话（内部方法）
    async fn send_message_to_session(&self, session_id: &str, message: UnifiedProtocolMessage) -> Result<bool> {
        // 使用连接管理器的发送方法
        self.connection_manager.send_message_to_session(session_id, message).await
    }
    
    /// 广播消息给所有用户（内部方法）
    async fn broadcast_message(&self, message: UnifiedProtocolMessage) -> Result<usize> {
        // 发送给所有连接
        let connections = self.connection_manager.get_all_connections().await?;
        let mut sent_count = 0;
        
        for (user_id, conn_info) in connections {
            // 获取连接实例
            if let Some(connection) = self.connection_manager.get_connection(&user_id, &conn_info.session_id).await? {
                match connection.send(message.clone()).await {
                    Ok(_) => {
                        sent_count += 1;
                        debug!("广播消息成功: 用户 {}", user_id);
                    }
                    Err(e) => {
                        error!("广播消息失败: 用户 {} 错误: {}", user_id, e);
                    }
                }
            }
        }
        
        self.update_stats(&message).await;
        info!("广播消息完成: 发送给 {} 个用户", sent_count);
        Ok(sent_count)
    }
    
    /// 更新统计信息（内部方法）
    async fn update_stats(&self, message: &UnifiedProtocolMessage) {
        let mut stats = self.message_stats.write().await;
        stats.total_messages_sent += 1;
        
        match &message.t {
            crate::common::protocol::MessageType::Text => {
                stats.text_messages_sent += 1;
            }
            crate::common::protocol::MessageType::Binary => {
                stats.binary_messages_sent += 1;
            }
            crate::common::protocol::MessageType::Notification => {
                stats.notifications_sent += 1;
            }
            crate::common::protocol::MessageType::Error => {
                stats.errors_sent += 1;
            }
            crate::common::protocol::MessageType::CustomMessage(_) => {
                stats.custom_messages_sent += 1;
            }
            crate::common::protocol::MessageType::RestResponse => {
                stats.rest_responses_sent += 1;
            }
            _ => {}
        }
    }
}

/// 消息处理器构建器
pub struct MessageProcessorBuilder<CM>
where
    CM: ServerConnectionManager + 'static,
{
    connection_manager: Option<Arc<CM>>,
}

impl<CM> MessageProcessorBuilder<CM>
where
    CM: ServerConnectionManager + 'static,
{
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            connection_manager: None,
        }
    }
    
    /// 设置连接管理器
    pub fn with_connection_manager(mut self, connection_manager: Arc<CM>) -> Self {
        self.connection_manager = Some(connection_manager);
        self
    }
    
    /// 构建消息处理器
    pub fn build(self) -> Result<MessageProcessor<CM>> {
        let connection_manager = self.connection_manager
            .ok_or_else(|| FlareError::InvalidConfiguration("连接管理器未设置".to_string()))?;
        
        Ok(MessageProcessor::new(connection_manager))
    }
}

impl<CM> Default for MessageProcessorBuilder<CM>
where
    CM: ServerConnectionManager + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

// 为方便使用，提供类型别名
pub type DefaultMessageProcessor = MessageProcessor<crate::server::conn_manager::MemoryServerConnectionManager>; 