//! 客户端回调管理模块
//!
//! 定义客户端回调接口和默认实现

use crate::common::{
    callback::{
        MessageCallback, EventCallback, NotificationCallback, RestCallback, ErrorCallback,
    },
    protocol::UnifiedProtocolMessage,
};
use crate::client::events::ClientEventCallback;

/// 消息处理器回调
pub type MessageHandler = Box<dyn Fn(UnifiedProtocolMessage) + Send + Sync>;

/// 错误处理器回调
pub type ErrorHandler = Box<dyn Fn(String) + Send + Sync>;

/// 客户端回调管理器
/// 
/// 统一管理所有类型的回调函数，提供便捷的接口来设置和处理各种事件
pub struct ClientCallbackManager {
    /// 消息回调
    pub message_callback: Box<dyn MessageCallback + Send + Sync>,
    /// 事件回调
    pub event_callback: Box<dyn EventCallback + Send + Sync>,
    /// 通知回调
    pub notification_callback: Box<dyn NotificationCallback + Send + Sync>,
    /// REST 回调
    pub rest_callback: Box<dyn RestCallback + Send + Sync>,
    /// 错误回调
    pub error_callback: Box<dyn ErrorCallback + Send + Sync>,
}

impl Default for ClientCallbackManager {
    fn default() -> Self {
        Self {
            message_callback: Box::new(crate::common::callback::DefaultMessageCallback),
            event_callback: Box::new(crate::common::callback::DefaultEventCallback),
            notification_callback: Box::new(crate::common::callback::DefaultNotificationCallback),
            rest_callback: Box::new(crate::common::callback::DefaultRestCallback),
            error_callback: Box::new(crate::common::callback::DefaultErrorCallback),
        }
    }
}

impl ClientCallbackManager {
    /// 创建新的回调管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置消息回调
    pub fn with_message_callback(mut self, callback: Box<dyn MessageCallback + Send + Sync>) -> Self {
        self.message_callback = callback;
        self
    }

    /// 设置事件回调
    pub fn with_event_callback(mut self, callback: Box<dyn EventCallback + Send + Sync>) -> Self {
        self.event_callback = callback;
        self
    }

    /// 设置通知回调
    pub fn with_notification_callback(mut self, callback: Box<dyn NotificationCallback + Send + Sync>) -> Self {
        self.notification_callback = callback;
        self
    }

    /// 设置 REST 回调
    pub fn with_rest_callback(mut self, callback: Box<dyn RestCallback + Send + Sync>) -> Self {
        self.rest_callback = callback;
        self
    }

    /// 设置错误回调
    pub fn with_error_callback(mut self, callback: Box<dyn ErrorCallback + Send + Sync>) -> Self {
        self.error_callback = callback;
        self
    }

    /// 处理协议消息并分发给相应回调
    pub async fn handle_protocol_message(&self, message: &UnifiedProtocolMessage, session_id: String) -> Result<(), String> {
        use crate::common::callback::converters;
        use crate::common::protocol::MessageType;

        match message.t {
            MessageType::Text => {
                if let Some(text_message) = converters::to_text_message(session_id.clone(), message) {
                    self.message_callback.on_text(&text_message).await
                } else {
                    Err("无法解析文本消息".to_string())
                }
            }
            MessageType::Binary => {
                if let Some(binary_message) = converters::to_binary_message(session_id.clone(), message) {
                    self.message_callback.on_binary(&binary_message).await
                } else {
                    Err("无法解析二进制消息".to_string())
                }
            }
            MessageType::Connect => {
                if let Some(connect_event) = converters::to_connect_event(session_id.clone(), message) {
                    self.event_callback.on_connect(&connect_event).await
                } else {
                    Err("无法解析连接事件".to_string())
                }
            }
            MessageType::Disconnect => {
                if let Some(disconnect_event) = converters::to_disconnect_event(session_id.clone(), message) {
                    self.event_callback.on_disconnect(&disconnect_event).await
                } else {
                    Err("无法解析断开连接事件".to_string())
                }
            }
            MessageType::Heartbeat => {
                if let Some(heartbeat_event) = converters::to_heartbeat_event(session_id.clone(), message) {
                    self.event_callback.on_heartbeat(&heartbeat_event).await
                } else {
                    Err("无法解析心跳事件".to_string())
                }
            }
            MessageType::HeartbeatAck => {
                if let Some(heartbeat_event) = converters::to_heartbeat_event(session_id.clone(), message) {
                    self.event_callback.on_heartbeat_ack(&heartbeat_event).await
                } else {
                    Err("无法解析心跳确认事件".to_string())
                }
            }
            MessageType::Notification => {
                if let Some(notification) = converters::to_notification_message(session_id.clone(), message) {
                    self.notification_callback.on_notification(&notification).await
                } else {
                    Err("无法解析通知消息".to_string())
                }
            }
            MessageType::Error => {
                if let Some(error) = converters::to_error_message(session_id.clone(), message) {
                    self.error_callback.on_error(&error).await
                } else {
                    Err("无法解析错误消息".to_string())
                }
            }
            MessageType::RestRequest => {
                if let Some(request) = converters::to_rest_request(session_id.clone(), message) {
                    self.rest_callback.on_rest_request(&request).await
                } else {
                    Err("无法解析REST请求".to_string())
                }
            }
            MessageType::RestResponse => {
                if let Some(response) = converters::to_rest_response(session_id.clone(), message) {
                    self.rest_callback.on_rest_response(&response).await
                } else {
                    Err("无法解析REST响应".to_string())
                }
            }
            MessageType::CustomEvent(ref event_name) => {
                if let Some(custom_event) = converters::to_custom_event(session_id.clone(), message) {
                    self.event_callback.on_custom_event(&custom_event).await
                } else {
                    self.event_callback.on_unknown_event(event_name, &message.c, &crate::common::callback::protocol_message_to_metadata(message, session_id)).await
                }
            }
            MessageType::CustomMessage(ref message_type) => {
                if let Some(custom_message) = converters::to_custom_message(session_id.clone(), message) {
                    self.message_callback.on_custom_message(&custom_message).await
                } else {
                    self.message_callback.on_unknown_message(message_type, &message.c, &crate::common::callback::protocol_message_to_metadata(message, session_id)).await
                }
            }
        }
    }
}


