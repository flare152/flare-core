//! Flare IM 回调系统模块
//!
//! 定义四种独立的回调接口：消息回调、事件回调、通知回调和错误回调

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::debug;
use crate::common::protocol::UnifiedProtocolMessage;

/// 消息元数据
#[derive(Debug, Clone)]
pub struct MessageMetadata {
    /// 用户ID
    pub user_id: String,
    /// 会话ID
    pub session_id: String,
    /// 发送者ID
    pub sender_id: Option<String>,
    /// 目标ID
    pub target_id: Option<String>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 消息类型标识
    pub message_type: String,
}

// ==================== 消息相关结构 ====================

/// 文本消息内容
#[derive(Debug, Clone)]
pub struct TextMessage {
    /// 文本内容
    pub content: String,
    /// 元数据
    pub metadata: MessageMetadata,
}

/// 二进制消息内容
#[derive(Debug, Clone)]
pub struct BinaryMessage {
    /// 二进制数据
    pub data: Vec<u8>,
    /// 元数据
    pub metadata: MessageMetadata,
}

/// 自定义消息内容
#[derive(Debug, Clone)]
pub struct CustomMessage {
    /// 消息类型
    pub message_type: String,
    /// 消息数据
    pub data: Vec<u8>,
    /// 元数据
    pub metadata: MessageMetadata,
}

/// REST 请求内容（带ID）
#[derive(Debug, Clone)]
pub struct RestRequest {
    /// 请求ID
    pub id: String,
    /// HTTP 方法
    pub method: String,
    /// 请求路径
    pub path: String,
    /// 请求体
    pub body: Vec<u8>,
    /// 元数据
    pub metadata: MessageMetadata,
}

/// REST 响应内容（带ID）
#[derive(Debug, Clone)]
pub struct RestResponse {
    /// 请求ID（对应请求的ID）
    pub id: String,
    /// HTTP 状态码
    pub status_code: u16,
    /// 响应体
    pub body: Vec<u8>,
    /// 元数据
    pub metadata: MessageMetadata,
}

// ==================== 事件相关结构 ====================

/// 连接事件内容
#[derive(Debug, Clone)]
pub struct ConnectEvent {
    /// 事件元数据
    pub metadata: MessageMetadata,
}

/// 断开连接事件内容
#[derive(Debug, Clone)]
pub struct DisconnectEvent {
    /// 断开原因
    pub reason: String,
    /// 事件元数据
    pub metadata: MessageMetadata,
}

/// 心跳事件内容
#[derive(Debug, Clone)]
pub struct HeartbeatEvent {
    /// 事件元数据
    pub metadata: MessageMetadata,
}

/// 自定义事件内容
#[derive(Debug, Clone)]
pub struct CustomEvent {
    /// 事件名称
    pub event_name: String,
    /// 事件数据
    pub data: Vec<u8>,
    /// 元数据
    pub metadata: MessageMetadata,
}

// ==================== 通知相关结构 ====================

/// 通知消息内容
#[derive(Debug, Clone)]
pub struct NotificationMessage {
    /// 通知标题
    pub title: String,
    /// 通知内容
    pub content: Vec<u8>,
    /// 元数据
    pub metadata: MessageMetadata,
}

/// 自定义通知内容
#[derive(Debug, Clone)]
pub struct CustomNotification {
    /// 通知类型
    pub notification_type: String,
    /// 通知数据
    pub data: Vec<u8>,
    /// 元数据
    pub metadata: MessageMetadata,
}

// ==================== 错误相关结构 ====================

/// 错误消息内容
#[derive(Debug, Clone)]
pub struct ErrorMessage {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 元数据
    pub metadata: MessageMetadata,
}

// ==================== 四种回调Trait ====================

/// 消息回调trait - 处理各种类型的消息
#[async_trait]
pub trait MessageCallback: Send + Sync {
    /// 处理文本消息
    async fn on_text(&self, message: &TextMessage) -> Result<(), String>;
    
    /// 处理二进制消息
    async fn on_binary(&self, message: &BinaryMessage) -> Result<(), String>;
    
    /// 处理自定义消息
    async fn on_custom_message(&self, message: &CustomMessage) -> Result<(), String>;
    
    /// 处理未知消息类型
    async fn on_unknown_message(&self, message_type: &str, data: &[u8], metadata: &MessageMetadata) -> Result<(), String>;
}

/// 事件回调trait - 处理各种类型的事件
#[async_trait]
pub trait EventCallback: Send + Sync {
    /// 处理连接事件
    async fn on_connect(&self, event: &ConnectEvent) -> Result<(), String>;
    
    /// 处理断开连接事件
    async fn on_disconnect(&self, event: &DisconnectEvent) -> Result<(), String>;
    
    /// 处理心跳事件
    async fn on_heartbeat(&self, event: &HeartbeatEvent) -> Result<(), String>;
    
    /// 处理心跳确认事件
    async fn on_heartbeat_ack(&self, event: &HeartbeatEvent) -> Result<(), String>;
    
    /// 处理自定义事件
    async fn on_custom_event(&self, event: &CustomEvent) -> Result<(), String>;
    
    /// 处理未知事件类型
    async fn on_unknown_event(&self, event_type: &str, data: &[u8], metadata: &MessageMetadata) -> Result<(), String>;
}

/// 通知回调trait - 处理各种类型的通知
#[async_trait]
pub trait NotificationCallback: Send + Sync {
    /// 处理通知消息
    async fn on_notification(&self, notification: &NotificationMessage) -> Result<(), String>;
    
    /// 处理自定义通知
    async fn on_custom_notification(&self, notification: &CustomNotification) -> Result<(), String>;
    
    /// 处理未知通知类型
    async fn on_unknown_notification(&self, notification_type: &str, data: &[u8], metadata: &MessageMetadata) -> Result<(), String>;
}

/// REST 回调trait - 处理 REST 请求和响应
#[async_trait]
pub trait RestCallback: Send + Sync {
    /// 处理 REST 请求
    async fn on_rest_request(&self, request: &RestRequest) -> Result<(), String>;
    
    /// 处理 REST 响应
    async fn on_rest_response(&self, response: &RestResponse) -> Result<(), String>;
}

/// 错误回调trait - 处理各种类型的错误
#[async_trait]
pub trait ErrorCallback: Send + Sync {
    /// 处理错误消息
    async fn on_error(&self, error: &ErrorMessage) -> Result<(), String>;
    
    /// 处理未知错误类型
    async fn on_unknown_error(&self, error_type: &str, data: &[u8], metadata: &MessageMetadata) -> Result<(), String>;
}

// ==================== 默认回调实现 ====================

/// 默认消息回调实现
pub struct DefaultMessageCallback;

#[async_trait]
impl MessageCallback for DefaultMessageCallback {
    async fn on_text(&self, message: &TextMessage) -> Result<(), String> {
        print!("收到文本消息: {} (来自: {:?})",
            message.content, 
            message.metadata.sender_id
        );
        Ok(())
    }
    
    async fn on_binary(&self, message: &BinaryMessage) -> Result<(), String> {
        debug!("收到二进制消息: {} bytes (来自: {:?})",
            message.data.len(), 
            message.metadata.sender_id
        );
        Ok(())
    }
    
    async fn on_custom_message(&self, message: &CustomMessage) -> Result<(), String> {
        debug!("收到自定义消息: {} ({} bytes) (来自: {:?})",
            message.message_type, message.data.len(), message.metadata.sender_id
        );
        Ok(())
    }
    
    async fn on_unknown_message(&self, message_type: &str, data: &[u8], metadata: &MessageMetadata) -> Result<(), String> {
        debug!("收到未知消息类型: {} ({} bytes) (来自: {:?})",
            message_type, data.len(), metadata.sender_id
        );
        Ok(())
    }
}

/// 默认事件回调实现
pub struct DefaultEventCallback;

#[async_trait]
impl EventCallback for DefaultEventCallback {
    async fn on_connect(&self, event: &ConnectEvent) -> Result<(), String> {
        debug!("建立连接: {} (会话: {}) ",
                   event.metadata.user_id, event.metadata.session_id
        );
        Ok(())
    }
    
    async fn on_disconnect(&self, event: &DisconnectEvent) -> Result<(), String> {
        debug!("用户断开: {} (会话: {}) 原因: {}",
            event.metadata.user_id, event.metadata.session_id, event.reason
        );
        Ok(())
    }
    
    async fn on_heartbeat(&self, _event: &HeartbeatEvent) -> Result<(), String> {
        debug!("收到心跳");
        Ok(())
    }
    
    async fn on_heartbeat_ack(&self, _event: &HeartbeatEvent) -> Result<(), String> {
        debug!("收到心跳确认");
        Ok(())
    }
    
    async fn on_custom_event(&self, event: &CustomEvent) -> Result<(), String> {
        debug!("收到自定义事件: {} ({} bytes) (来自: {:?})",
            event.event_name, event.data.len(), event.metadata.sender_id
        );
        Ok(())
    }
    
    async fn on_unknown_event(&self, event_type: &str, data: &[u8], metadata: &MessageMetadata) -> Result<(), String> {
        debug!("收到未知事件类型: {} ({} bytes) (来自: {:?})",
            event_type, data.len(), metadata.sender_id
        );
        Ok(())
    }
}

/// 默认通知回调实现
pub struct DefaultNotificationCallback;

#[async_trait]
impl NotificationCallback for DefaultNotificationCallback {
    async fn on_notification(&self, notification: &NotificationMessage) -> Result<(), String> {
        if let Ok(content) = String::from_utf8(notification.content.clone()) {
            debug!("收到通知: {} - {} (来自: {:?})",
                notification.title, content, notification.metadata.sender_id
            );
        } else {
            debug!("收到通知: {} - {} bytes (来自: {:?})",
                notification.title, notification.content.len(), notification.metadata.sender_id
            );
        }
        Ok(())
    }
    
    async fn on_custom_notification(&self, notification: &CustomNotification) -> Result<(), String> {
        debug!("收到自定义通知: {} ({} bytes) (来自: {:?})",
            notification.notification_type, notification.data.len(), notification.metadata.sender_id
        );
        Ok(())
    }
    
    async fn on_unknown_notification(&self, notification_type: &str, data: &[u8], metadata: &MessageMetadata) -> Result<(), String> {
        debug!("收到未知通知类型: {} ({} bytes) (来自: {:?})",
            notification_type, data.len(), metadata.sender_id
        );
        Ok(())
    }
}

/// 默认 REST 回调实现
pub struct DefaultRestCallback;

#[async_trait]
impl RestCallback for DefaultRestCallback {
    async fn on_rest_request(&self, request: &RestRequest) -> Result<(), String> {
        debug!("收到 REST 请求 [ID: {}]: {} {} ({} bytes) (来自: {:?})",
            request.id, request.method, request.path, request.body.len(), request.metadata.sender_id
        );
        Ok(())
    }
    
    async fn on_rest_response(&self, response: &RestResponse) -> Result<(), String> {
        debug!("收到 REST 响应 [ID: {}]: {} ({} bytes) (来自: {:?})",
            response.id, response.status_code, response.body.len(), response.metadata.sender_id
        );
        Ok(())
    }
}

/// 默认错误回调实现
pub struct DefaultErrorCallback;

#[async_trait]
impl ErrorCallback for DefaultErrorCallback {
    async fn on_error(&self, error: &ErrorMessage) -> Result<(), String> {
        debug!("收到错误: {} - {} (来自: {:?})",
            error.code, error.message, error.metadata.sender_id
        );
        Ok(())
    }
    
    async fn on_unknown_error(&self, error_type: &str, data: &[u8], metadata: &MessageMetadata) -> Result<(), String> {
        debug!("收到未知错误类型: {} ({} bytes) (来自: {:?})",
            error_type, data.len(), metadata.sender_id
        );
        Ok(())
    }
}

// ==================== 便利方法 ====================

/// 便利方法 - 从UnifiedProtocolMessage创建MessageMetadata
pub fn create_metadata_from_protocol_message(
    message_type: &str,
    sender_id: Option<String>,
    target_id: Option<String>,
    timestamp: Option<DateTime<Utc>>,
    session_id: String,
) -> MessageMetadata {
    MessageMetadata {
        user_id: "".to_string(),
        session_id,
        sender_id,
        target_id,
        timestamp: timestamp.unwrap_or_else(Utc::now),
        message_type: message_type.to_string(),
    }
}

/// 统一方法 - 从UnifiedProtocolMessage直接转换到MessageMetadata
pub fn protocol_message_to_metadata(
    protocol_message: &UnifiedProtocolMessage,
    session_id: String,
) -> MessageMetadata {
    MessageMetadata {
        user_id: "".to_string(),
        session_id,
        sender_id: protocol_message.sender_id().map(|s| s.to_string()),
        target_id: protocol_message.target_id().map(|s| s.to_string()),
        timestamp: protocol_message.timestamp(),
        message_type: protocol_message.t.to_string().to_string(),
    }
}

/// 便利方法 - 从UnifiedProtocolMessage创建各种消息内容
pub mod converters {
    use super::*;
    use crate::common::protocol::UnifiedProtocolMessage;
    
    // ==================== 消息转换器 ====================
    
    /// 转换为文本消息
    pub fn to_text_message(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<TextMessage> {
        if let Some(text) = protocol_message.as_text() {
            let metadata = protocol_message_to_metadata(protocol_message, session_id);
            Some(TextMessage {
                content: text,
                metadata,
            })
        } else {
            None
        }
    }
    
    /// 转换为二进制消息
    pub fn to_binary_message(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<BinaryMessage> {
        if protocol_message.is_binary() {
            let metadata = protocol_message_to_metadata(protocol_message, session_id);
            Some(BinaryMessage {
                data: protocol_message.as_binary().clone(),
                metadata,
            })
        } else {
            None
        }
    }
    
    /// 转换为自定义消息
    pub fn to_custom_message(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<CustomMessage> {
        if let Some((message_type, data)) = protocol_message.as_custom_message_info() {
            let metadata = protocol_message_to_metadata(protocol_message, session_id);
            Some(CustomMessage {
                message_type,
                data,
                metadata,
            })
        } else {
            None
        }
    }
    
    /// 转换为 REST 请求
    pub fn to_rest_request(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<RestRequest> {
        if let Some((id, method, path, body)) = protocol_message.as_rest_request_info() {
            let metadata = protocol_message_to_metadata(protocol_message, session_id);
            Some(RestRequest {
                id,
                method,
                path,
                body,
                metadata,
            })
        } else {
            None
        }
    }
    
    /// 转换为 REST 响应
    pub fn to_rest_response(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<RestResponse> {
        if let Some((id, status_code, body)) = protocol_message.as_rest_response_info() {
            let metadata = protocol_message_to_metadata(protocol_message, session_id);
            Some(RestResponse {
                id,
                status_code,
                body,
                metadata,
            })
        } else {
            None
        }
    }
    
    // ==================== 事件转换器 ====================
    
    /// 转换为连接事件
    pub fn to_connect_event(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<ConnectEvent> {
        if let Some((user_id, _)) = protocol_message.as_connect_info() {
            Some(ConnectEvent {
                metadata: protocol_message_to_metadata(protocol_message, session_id),
            })
        } else {
            None
        }
    }
    
    /// 转换为断开连接事件
    pub fn to_disconnect_event(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<DisconnectEvent> {
        if let Some((user_id, _, reason)) = protocol_message.as_disconnect_info() {
            Some(DisconnectEvent {
                reason,
                metadata: protocol_message_to_metadata(protocol_message, session_id),
            })
        } else {
            None
        }
    }
    
    /// 转换为心跳事件
    pub fn to_heartbeat_event(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<HeartbeatEvent> {
        if protocol_message.is_heartbeat() || protocol_message.is_heartbeat_ack() {
            Some(HeartbeatEvent {
                metadata: protocol_message_to_metadata(protocol_message, session_id),
            })
        } else {
            None
        }
    }
    
    /// 转换为自定义事件
    pub fn to_custom_event(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<CustomEvent> {
        if let Some((event_name, data)) = protocol_message.as_custom_event_info() {
            Some(CustomEvent {
                event_name,
                data,
                metadata: protocol_message_to_metadata(protocol_message, session_id),
            })
        } else {
            None
        }
    }
    
    // ==================== 通知转换器 ====================
    
    /// 转换为通知消息
    pub fn to_notification_message(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<NotificationMessage> {
        if let Some((title, content)) = protocol_message.as_notification_info() {
            Some(NotificationMessage {
                title,
                content,
                metadata: protocol_message_to_metadata(protocol_message, session_id),
            })
        } else {
            None
        }
    }
    
    // ==================== 错误转换器 ====================
    
    /// 转换为错误消息
    pub fn to_error_message(session_id: String, protocol_message: &UnifiedProtocolMessage) -> Option<ErrorMessage> {
        if let Some((code, message)) = protocol_message.as_error_info() {
            Some(ErrorMessage {
                code,
                message,
                metadata: protocol_message_to_metadata(protocol_message, session_id),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::protocol::UnifiedProtocolMessage;
    
    #[tokio::test]
    async fn test_message_callback() {
        let callback = DefaultMessageCallback;
        
        // 测试文本消息回调
        let text_message = TextMessage {
            content: "Hello World".to_string(),
            metadata: MessageMetadata {
                user_id: "user1".to_string(),
                session_id: "session_123".to_string(),
                sender_id: Some("user1".to_string()),
                target_id: Some("user2".to_string()),
                timestamp: Utc::now(),
                message_type: "text".to_string(),
            },
        };
        
        let result = callback.on_text(&text_message).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_event_callback() {
        let callback = DefaultEventCallback;
        
        // 测试连接事件回调
        let connect_event = ConnectEvent {
            metadata: MessageMetadata {
                user_id: "user1".to_string(),
                session_id: "session_123".to_string(),
                sender_id: Some("user1".to_string()),
                target_id: None,
                timestamp: Utc::now(),
                message_type: "connect".to_string(),
            },
        };
        
        let result = callback.on_connect(&connect_event).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_notification_callback() {
        let callback = DefaultNotificationCallback;
        
        // 测试通知回调
        let notification = NotificationMessage {
            title: "系统通知".to_string(),
            content: "服务器维护".as_bytes().to_vec(),
            metadata: MessageMetadata {
                user_id: "system".to_string(),
                session_id: "session_456".to_string(),
                sender_id: Some("system".to_string()),
                target_id: None,
                timestamp: Utc::now(),
                message_type: "notification".to_string(),
            },
        };
        
        let result = callback.on_notification(&notification).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_error_callback() {
        let callback = DefaultErrorCallback;
        
        // 测试错误回调
        let error = ErrorMessage {
            code: "AUTH_FAILED".to_string(),
            message: "认证失败".to_string(),
            metadata: MessageMetadata {
                user_id: "user1".to_string(),
                session_id: "session_789".to_string(),
                sender_id: Some("system".to_string()),
                target_id: Some("user1".to_string()),
                timestamp: Utc::now(),
                message_type: "error".to_string(),
            },
        };
        
        let result = callback.on_error(&error).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_converters() {
        // 测试协议消息转换
        let protocol_message = UnifiedProtocolMessage::text("Test message".to_string())
            .sender("user1".to_string())
            .target("user2".to_string());
        
        let text_message = converters::to_text_message("session_123".to_string(), &protocol_message);
        assert!(text_message.is_some());
        
        let text_message = text_message.unwrap();
        assert_eq!(text_message.content, "Test message");
        assert_eq!(text_message.metadata.sender_id, Some("user1".to_string()));
        assert_eq!(text_message.metadata.target_id, Some("user2".to_string()));
    }
} 