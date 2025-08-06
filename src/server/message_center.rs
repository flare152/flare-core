//! Flare IM 消息处理中心模块
//!
//! 提供统一的消息处理逻辑，支持QUIC和WebSocket协议

use std::sync::Arc;
use tracing::{debug, error};

use crate::common::{
    conn::{ConnectionEvent, ProtoMessage},
    Result,
};

use super::{
    handlers::{AuthHandler, MessageHandler, EventHandler},
    conn_manager::{MemoryServerConnectionManager, ServerConnectionManager},
};

/// 消息处理中心
/// 
/// 负责统一处理来自不同协议的消息，包括：
/// - 消息路由和分发
/// - 调用消息处理器
/// - 触发相关事件
/// - 心跳管理
pub struct MessageProcessingCenter {
    connection_manager: Arc<MemoryServerConnectionManager>,
    auth_handler: Option<Arc<dyn AuthHandler>>,
    message_handler: Option<Arc<dyn MessageHandler>>,
    event_handler: Option<Arc<dyn EventHandler>>,
}

impl MessageProcessingCenter {
    /// 创建新的消息处理中心
    pub fn new(
        connection_manager: Arc<MemoryServerConnectionManager>,
        auth_handler: Option<Arc<dyn AuthHandler>>,
        message_handler: Option<Arc<dyn MessageHandler>>,
        event_handler: Option<Arc<dyn EventHandler>>,
    ) -> Self {
        Self {
            connection_manager,
            auth_handler,
            message_handler,
            event_handler,
        }
    }

    /// 统一消息处理入口
    pub async fn process_message(
        &self,
        user_id: &str,
        session_id: &str,
        message: ProtoMessage,
    ) -> Result<ProtoMessage> {
        debug!("处理消息: 用户 {} 类型 {}", user_id, message.message_type);
        
        // 更新连接心跳
        ServerConnectionManager::update_heartbeat(&*self.connection_manager, user_id, session_id).await?;
        
        // 触发消息接收事件
        self.trigger_message_received_event(user_id, &message).await?;
        
        // 根据消息类型处理
        let response = match message.message_type.as_str() {
            "message" => self.handle_user_message(user_id, message).await?,
            "heartbeat" => self.handle_heartbeat_message(user_id, session_id).await?,
            "ping" => self.handle_ping_message(user_id, message).await?,
            "auth" => self.handle_auth_message(user_id, message).await?,
            _ => self.handle_unknown_message(user_id, message).await?,
        };
        
        // 触发消息发送事件
        self.trigger_message_sent_event(user_id, &response).await?;
        
        Ok(response)
    }

    /// 处理用户消息
    async fn handle_user_message(&self, user_id: &str, message: ProtoMessage) -> Result<ProtoMessage> {
        // 调用消息处理器
        if let Some(handler) = &self.message_handler {
            let response = handler.handle_message(user_id, message).await?;
            return Ok(response);
        }
        
        // 默认echo响应
        Ok(ProtoMessage::new(
            uuid::Uuid::new_v4().to_string(),
            "echo".to_string(),
            message.payload,
        ))
    }

    /// 处理心跳消息
    async fn handle_heartbeat_message(&self, user_id: &str, session_id: &str) -> Result<ProtoMessage> {
        // 调用消息处理器
        if let Some(handler) = &self.message_handler {
            handler.handle_heartbeat(user_id, session_id).await?;
        }
        
        // 触发心跳事件
        self.trigger_heartbeat_event(user_id).await?;
        
        // 返回心跳响应
        Ok(ProtoMessage::new(
            uuid::Uuid::new_v4().to_string(),
            "heartbeat_ack".to_string(),
            vec![],
        ))
    }

    /// 处理ping消息
    async fn handle_ping_message(&self, _user_id: &str, message: ProtoMessage) -> Result<ProtoMessage> {
        // 返回pong响应
        Ok(ProtoMessage::new(
            uuid::Uuid::new_v4().to_string(),
            "pong".to_string(),
            message.payload,
        ))
    }

    /// 处理认证消息
    async fn handle_auth_message(&self, user_id: &str, message: ProtoMessage) -> Result<ProtoMessage> {
        // 解析token
        let token = String::from_utf8_lossy(&message.payload);
        let token = token.trim_matches('"');
        
        // 验证token
        if let Some(handler) = &self.auth_handler {
            if let Some(validated_user_id) = handler.validate_token(token).await? {
                // 认证成功
                Ok(ProtoMessage::new(
                    uuid::Uuid::new_v4().to_string(),
                    "auth_success".to_string(),
                    validated_user_id.as_bytes().to_vec(),
                ))
            } else {
                // 认证失败
                Ok(ProtoMessage::new(
                    uuid::Uuid::new_v4().to_string(),
                    "auth_failed".to_string(),
                    "Invalid token".as_bytes().to_vec(),
                ))
            }
        } else {
            // 没有认证处理器，默认成功
            Ok(ProtoMessage::new(
                uuid::Uuid::new_v4().to_string(),
                "auth_success".to_string(),
                user_id.as_bytes().to_vec(),
            ))
        }
    }

    /// 处理未知消息类型
    async fn handle_unknown_message(&self, user_id: &str, message: ProtoMessage) -> Result<ProtoMessage> {
        error!("未知消息类型: {}", message.message_type);
        
        // 调用消息处理器
        if let Some(handler) = &self.message_handler {
            handler.handle_message(user_id, message).await
        } else {
            // 默认echo响应
            Ok(ProtoMessage::new(
                uuid::Uuid::new_v4().to_string(),
                "echo".to_string(),
                message.payload,
            ))
        }
    }

    /// 触发消息接收事件
    async fn trigger_message_received_event(&self, user_id: &str, message: &ProtoMessage) -> Result<()> {
        if let Some(handler) = &self.event_handler {
            handler.handle_connection_event(user_id, ConnectionEvent::MessageReceived(message.clone())).await?;
        }
        Ok(())
    }

    /// 触发消息发送事件
    async fn trigger_message_sent_event(&self, user_id: &str, message: &ProtoMessage) -> Result<()> {
        if let Some(handler) = &self.event_handler {
            handler.handle_connection_event(user_id, ConnectionEvent::MessageSent(message.clone())).await?;
        }
        Ok(())
    }

    /// 触发心跳事件
    async fn trigger_heartbeat_event(&self, user_id: &str) -> Result<()> {
        if let Some(handler) = &self.event_handler {
            handler.handle_connection_event(user_id, ConnectionEvent::Heartbeat).await?;
        }
        Ok(())
    }

    /// 获取连接管理器
    pub fn get_connection_manager(&self) -> Arc<MemoryServerConnectionManager> {
        Arc::clone(&self.connection_manager)
    }

    /// 获取认证处理器
    pub fn get_auth_handler(&self) -> Option<Arc<dyn AuthHandler>> {
        self.auth_handler.clone()
    }

    /// 获取消息处理器
    pub fn get_message_handler(&self) -> Option<Arc<dyn MessageHandler>> {
        self.message_handler.clone()
    }

    /// 获取事件处理器
    pub fn get_event_handler(&self) -> Option<Arc<dyn EventHandler>> {
        self.event_handler.clone()
    }
} 