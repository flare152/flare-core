//! Flare IM 消息解析器模块
//!
//! 提供可扩展的消息解析器插件系统，专注于消息解析功能

use crate::common::{
    protocol::{UnifiedProtocolMessage, MessageType},
    callback::{
        MessageCallback, EventCallback, NotificationCallback, ErrorCallback, RestCallback,
        DefaultMessageCallback, DefaultEventCallback, DefaultNotificationCallback, DefaultErrorCallback, DefaultRestCallback,
        converters
    }
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

/// 消息解析结果
#[derive(Debug, Clone)]
pub enum ParseResult {
    /// 解析成功
    Success(UnifiedProtocolMessage),
    /// 解析失败
    Error(String),
    /// 跳过此消息
    Skip,
}

/// 自定义消息解析器trait
#[async_trait]
pub trait CustomParser: Send + Sync {
    /// 解析器名称
    fn name(&self) -> &str;
    
    /// 解析消息
    async fn parse(&self, data: &[u8]) -> Result<ParseResult, String>;
    
    /// 检查是否支持此数据格式
    fn can_parse(&self, data: &[u8]) -> bool;
}

/// 消息解析器管理器
pub struct MessageParser {
    /// 消息回调处理器
    message_callback: Arc<dyn MessageCallback>,
    /// 事件回调处理器
    event_callback: Arc<dyn EventCallback>,
    /// 通知回调处理器
    notification_callback: Arc<dyn NotificationCallback>,
    /// 错误回调处理器
    error_callback: Arc<dyn ErrorCallback>,
    /// REST 回调处理器
    rest_callback: Arc<dyn RestCallback>,
    /// 自定义解析器
    custom_parsers: Arc<RwLock<HashMap<String, Arc<dyn CustomParser>>>>,
}

impl MessageParser {
    /// 创建新的消息解析器
    pub fn new(
        message_callback: Arc<dyn MessageCallback>,
        event_callback: Arc<dyn EventCallback>,
        notification_callback: Arc<dyn NotificationCallback>,
        error_callback: Arc<dyn ErrorCallback>,
        rest_callback: Arc<dyn RestCallback>,
    ) -> Self {
        Self {
            message_callback,
            event_callback,
            notification_callback,
            error_callback,
            rest_callback,
            custom_parsers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 使用默认回调创建消息解析器
    pub fn with_default_callbacks() -> Self {
        Self::new(
            Arc::new(DefaultMessageCallback),
            Arc::new(DefaultEventCallback),
            Arc::new(DefaultNotificationCallback),
            Arc::new(DefaultErrorCallback),
            Arc::new(DefaultRestCallback),
        )
    }
    
    /// 使用自定义消息回调创建解析器
    pub fn with_custom_message_callback(message_callback: Arc<dyn MessageCallback>) -> Self {
        Self::new(
            message_callback,
            Arc::new(DefaultEventCallback),
            Arc::new(DefaultNotificationCallback),
            Arc::new(DefaultErrorCallback),
            Arc::new(DefaultRestCallback),
        )
    }
    
    /// 使用自定义事件回调创建解析器
    pub fn with_custom_event_callback(event_callback: Arc<dyn EventCallback>) -> Self {
        Self::new(
            Arc::new(DefaultMessageCallback),
            event_callback,
            Arc::new(DefaultNotificationCallback),
            Arc::new(DefaultErrorCallback),
            Arc::new(DefaultRestCallback),
        )
    }
    
    /// 使用自定义通知回调创建解析器
    pub fn with_custom_notification_callback(notification_callback: Arc<dyn NotificationCallback>) -> Self {
        Self::new(
            Arc::new(DefaultMessageCallback),
            Arc::new(DefaultEventCallback),
            notification_callback,
            Arc::new(DefaultErrorCallback),
            Arc::new(DefaultRestCallback),
        )
    }
    
    /// 使用自定义错误回调创建解析器
    pub fn with_custom_error_callback(error_callback: Arc<dyn ErrorCallback>) -> Self {
        Self::new(
            Arc::new(DefaultMessageCallback),
            Arc::new(DefaultEventCallback),
            Arc::new(DefaultNotificationCallback),
            error_callback,
            Arc::new(DefaultRestCallback),
        )
    }
    
    /// 使用自定义 REST 回调创建解析器
    pub fn with_custom_rest_callback(rest_callback: Arc<dyn RestCallback>) -> Self {
        Self::new(
            Arc::new(DefaultMessageCallback),
            Arc::new(DefaultEventCallback),
            Arc::new(DefaultNotificationCallback),
            Arc::new(DefaultErrorCallback),
            rest_callback,
        )
    }
    
    /// 注册自定义解析器
    pub async fn register_parser(&self, parser: Arc<dyn CustomParser>) {
        let mut parsers = self.custom_parsers.write().await;
        parsers.insert(parser.name().to_string(), parser);
    }
    
    /// 移除自定义解析器
    pub async fn unregister_parser(&self, name: &str) -> bool {
        let mut parsers = self.custom_parsers.write().await;
        parsers.remove(name).is_some()
    }
    
    /// 解析消息
    pub async fn parse_message(&self, data: &[u8]) -> Result<UnifiedProtocolMessage, String> {
        // 首先尝试自定义解析器
        let parsers = self.custom_parsers.read().await;
        for parser in parsers.values() {
            if parser.can_parse(data) {
                match parser.parse(data).await {
                    Ok(ParseResult::Success(message)) => return Ok(message),
                    Ok(ParseResult::Skip) => continue,
                    Ok(ParseResult::Error(e)) => return Err(e),
                    Err(e) => return Err(e),
                }
            }
        }
        
        // 使用默认JSON解析器
        self.parse_json_message(data).await
    }
    
    /// 处理消息（解析并调用相应回调）
    pub async fn handle_message(&self,session_id:String, data: &[u8]) -> Result<(), String> {
        let message = self.parse_message(data).await?;
        self.dispatch_message(session_id,&message).await
    }
    
    /// 分发消息到相应的回调
    pub async fn dispatch_message(&self,session_id:String, message: &UnifiedProtocolMessage) -> Result<(), String> {
        match &message.t {
            // ==================== 消息类型 ====================
            MessageType::Text => {
                if let Some(text_message) = converters::to_text_message(session_id.clone(), message) {
                    self.message_callback.on_text(&text_message).await
                } else {
                    Err("无法解析文本消息".to_string())
                }
            },
            MessageType::Binary => {
                if let Some(binary_message) = converters::to_binary_message(session_id.clone(), message) {
                    self.message_callback.on_binary(&binary_message).await
                } else {
                    Err("无法解析二进制消息".to_string())
                }
            },
            MessageType::CustomMessage(_) => {
                if let Some(custom_message) = converters::to_custom_message(session_id.clone(), message) {
                    self.message_callback.on_custom_message(&custom_message).await
                } else {
                    Err("无法解析自定义消息".to_string())
                }
            },
            MessageType::RestRequest => {
                if let Some(rest_request) = converters::to_rest_request(session_id.clone(), message) {
                    self.rest_callback.on_rest_request(&rest_request).await
                } else {
                    Err("无法解析 REST 请求".to_string())
                }
            },
            MessageType::RestResponse => {
                if let Some(rest_response) = converters::to_rest_response(session_id.clone(), message) {
                    self.rest_callback.on_rest_response(&rest_response).await
                } else {
                    Err("无法解析 REST 响应".to_string())
                }
            },
            
            // ==================== 事件类型 ====================
            MessageType::Connect => {
                if let Some(connect_event) = converters::to_connect_event(session_id.clone(), message) {
                    self.event_callback.on_connect(&connect_event).await
                } else {
                    Err("无法解析连接事件".to_string())
                }
            },
            MessageType::Disconnect => {
                if let Some(disconnect_event) = converters::to_disconnect_event(session_id.clone(), message) {
                    self.event_callback.on_disconnect(&disconnect_event).await
                } else {
                    Err("无法解析断开连接事件".to_string())
                }
            },
            MessageType::Heartbeat => {
                if let Some(heartbeat_event) = converters::to_heartbeat_event(session_id.clone(), message) {
                    self.event_callback.on_heartbeat(&heartbeat_event).await
                } else {
                    Err("无法解析心跳事件".to_string())
                }
            },
            MessageType::HeartbeatAck => {
                if let Some(heartbeat_event) = converters::to_heartbeat_event(session_id.clone(), message) {
                    self.event_callback.on_heartbeat_ack(&heartbeat_event).await
                } else {
                    Err("无法解析心跳确认事件".to_string())
                }
            },
            MessageType::CustomEvent(_) => {
                if let Some(custom_event) = converters::to_custom_event(session_id.clone(), message) {
                    self.event_callback.on_custom_event(&custom_event).await
                } else {
                    Err("无法解析自定义事件".to_string())
                }
            },
            
            // ==================== 通知类型 ====================
            MessageType::Notification => {
                if let Some(notification_message) = converters::to_notification_message(session_id.clone(), message) {
                    self.notification_callback.on_notification(&notification_message).await
                } else {
                    Err("无法解析通知消息".to_string())
                }
            },
            
            // ==================== 错误类型 ====================
            MessageType::Error => {
                if let Some(error_message) = converters::to_error_message(session_id.clone(), message) {
                    self.error_callback.on_error(&error_message).await
                } else {
                    Err("无法解析错误消息".to_string())
                }
            },
        }
    }
    
    /// 默认JSON解析器
    async fn parse_json_message(&self, data: &[u8]) -> Result<UnifiedProtocolMessage, String> {
        let json_str = String::from_utf8(data.to_vec())
            .map_err(|e| format!("Invalid UTF-8: {}", e))?;
        
        serde_json::from_str(&json_str)
            .map_err(|e| format!("JSON parse error: {}", e))
    }
}

/// 构建器模式 - 用于创建消息解析器
pub struct MessageParserBuilder {
    message_callback: Option<Arc<dyn MessageCallback>>,
    event_callback: Option<Arc<dyn EventCallback>>,
    notification_callback: Option<Arc<dyn NotificationCallback>>,
    error_callback: Option<Arc<dyn ErrorCallback>>,
    rest_callback: Option<Arc<dyn RestCallback>>,
    parsers: Vec<Arc<dyn CustomParser>>,
}

impl MessageParserBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            message_callback: None,
            event_callback: None,
            notification_callback: None,
            error_callback: None,
            rest_callback: None,
            parsers: Vec::new(),
        }
    }
    
    /// 设置消息回调
    pub fn with_message_callback(mut self, callback: Arc<dyn MessageCallback>) -> Self {
        self.message_callback = Some(callback);
        self
    }
    
    /// 设置事件回调
    pub fn with_event_callback(mut self, callback: Arc<dyn EventCallback>) -> Self {
        self.event_callback = Some(callback);
        self
    }
    
    /// 设置通知回调
    pub fn with_notification_callback(mut self, callback: Arc<dyn NotificationCallback>) -> Self {
        self.notification_callback = Some(callback);
        self
    }
    
    /// 设置错误回调
    pub fn with_error_callback(mut self, callback: Arc<dyn ErrorCallback>) -> Self {
        self.error_callback = Some(callback);
        self
    }
    
    /// 设置 REST 回调
    pub fn with_rest_callback(mut self, callback: Arc<dyn RestCallback>) -> Self {
        self.rest_callback = Some(callback);
        self
    }
    
    /// 添加自定义解析器
    pub fn with_parser(mut self, parser: Arc<dyn CustomParser>) -> Self {
        self.parsers.push(parser);
        self
    }
    
    /// 构建消息解析器
    pub async fn build(self) -> MessageParser {
        let message_callback = self.message_callback.unwrap_or_else(|| {
            Arc::new(DefaultMessageCallback)
        });
        let event_callback = self.event_callback.unwrap_or_else(|| {
            Arc::new(DefaultEventCallback)
        });
        let notification_callback = self.notification_callback.unwrap_or_else(|| {
            Arc::new(DefaultNotificationCallback)
        });
        let error_callback = self.error_callback.unwrap_or_else(|| {
            Arc::new(DefaultErrorCallback)
        });
        
        let rest_callback = self.rest_callback.unwrap_or_else(|| {
            Arc::new(DefaultRestCallback)
        });
        
        let parser = MessageParser::new(
            message_callback,
            event_callback,
            notification_callback,
            error_callback,
            rest_callback,
        );
        
        // 注册自定义解析器
        for custom_parser in self.parsers {
            parser.register_parser(custom_parser).await;
        }
        
        parser
    }
}

impl Default for MessageParserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 二进制消息解析器（用于处理二进制格式的消息）
pub struct BinaryMessageParser;

#[async_trait]
impl CustomParser for BinaryMessageParser {
    fn name(&self) -> &str {
        "binary_parser"
    }
    
    fn can_parse(&self, data: &[u8]) -> bool {
        // 检查是否为二进制格式（这里可以根据实际协议定义来判断）
        data.len() >= 2 && data[0] == 0x01 && data[1] == 0x02
    }
    
    async fn parse(&self, data: &[u8]) -> Result<ParseResult, String> {
        // 这里实现二进制格式的解析逻辑
        // 示例：假设二进制格式为 [0x01, 0x02, message_type, content...]
        if data.len() < 3 {
            return Ok(ParseResult::Error("数据长度不足".to_string()));
        }
        
        let message_type = data[2];
        let content = data[3..].to_vec();
        
        let msg_type = match message_type {
            0x01 => MessageType::Text,
            0x02 => MessageType::Binary,
            0x03 => MessageType::Connect,
            0x04 => MessageType::Disconnect,
            0x05 => MessageType::Heartbeat,
            0x06 => MessageType::HeartbeatAck,
            0x07 => MessageType::Notification,
            0x08 => MessageType::Error,
            _ => return Ok(ParseResult::Error("未知消息类型".to_string())),
        };
        
        let message = UnifiedProtocolMessage::new(msg_type, content);
        Ok(ParseResult::Success(message))
    }
}

/// 协议缓冲区解析器（用于处理protobuf格式的消息）
pub struct ProtobufMessageParser;

#[async_trait]
impl CustomParser for ProtobufMessageParser {
    fn name(&self) -> &str {
        "protobuf_parser"
    }
    
    async fn parse(&self, _data: &[u8]) -> Result<ParseResult, String> {
        // 这里实现protobuf格式的解析逻辑
        // 由于没有实际的protobuf依赖，这里只是示例
        Ok(ParseResult::Skip) // 暂时跳过
    }
    
    fn can_parse(&self, _data: &[u8]) -> bool {
        // 检查是否为protobuf格式（这里可以根据实际协议定义来判断）
        _data.len() >= 1 && _data[0] == 0x08
    }
}

/// 消息解析器工厂
pub struct MessageParserFactory;

impl MessageParserFactory {
    /// 创建QUIC消息解析器
    pub async fn create_quic_parser(
        message_callback: Arc<dyn MessageCallback>,
        event_callback: Arc<dyn EventCallback>,
        notification_callback: Arc<dyn NotificationCallback>,
        error_callback: Arc<dyn ErrorCallback>,
        rest_callback: Arc<dyn RestCallback>,
    ) -> MessageParser {
        MessageParserBuilder::new()
            .with_message_callback(message_callback)
            .with_event_callback(event_callback)
            .with_notification_callback(notification_callback)
            .with_error_callback(error_callback)
            .with_rest_callback(rest_callback)
            .with_parser(Arc::new(BinaryMessageParser))
            .build()
            .await
    }
    
    /// 创建WebSocket消息解析器
    pub async fn create_websocket_parser(
        message_callback: Arc<dyn MessageCallback>,
        event_callback: Arc<dyn EventCallback>,
        notification_callback: Arc<dyn NotificationCallback>,
        error_callback: Arc<dyn ErrorCallback>,
        rest_callback: Arc<dyn RestCallback>,
    ) -> MessageParser {
        MessageParserBuilder::new()
            .with_message_callback(message_callback)
            .with_event_callback(event_callback)
            .with_notification_callback(notification_callback)
            .with_error_callback(error_callback)
            .with_rest_callback(rest_callback)
            .with_parser(Arc::new(BinaryMessageParser))
            .build()
            .await
    }
    
    /// 创建通用消息解析器
    pub async fn create_generic_parser(
        message_callback: Arc<dyn MessageCallback>,
        event_callback: Arc<dyn EventCallback>,
        notification_callback: Arc<dyn NotificationCallback>,
        error_callback: Arc<dyn ErrorCallback>,
        rest_callback: Arc<dyn RestCallback>,
    ) -> MessageParser {
        MessageParserBuilder::new()
            .with_message_callback(message_callback)
            .with_event_callback(event_callback)
            .with_notification_callback(notification_callback)
            .with_error_callback(error_callback)
            .with_rest_callback(rest_callback)
            .with_parser(Arc::new(BinaryMessageParser))
            .with_parser(Arc::new(ProtobufMessageParser))
            .build()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        protocol::UnifiedProtocolMessage,
        callback::{TextMessage, MessageMetadata, DefaultMessageCallback, DefaultEventCallback}
    };
    use chrono::Utc;
    
    #[tokio::test]
    async fn test_message_parser() {
        let parser = MessageParser::with_default_callbacks();
        
        // 测试JSON消息解析
        let json_data = r#"{"t":"t","c":"Hello World","s":"user1","d":"user2"}"#.as_bytes();
        let result = parser.parse_message(json_data).await;
        assert!(result.is_ok());
        
        let message = result.unwrap();
        assert!(message.is_text());
        assert_eq!(message.as_text().unwrap(), "Hello World");
    }
    
    #[tokio::test]
    async fn test_custom_parser() {
        let parser = MessageParser::with_default_callbacks();
        
        // 注册自定义解析器
        parser.register_parser(Arc::new(BinaryMessageParser)).await;
        
        // 测试二进制消息解析
        let binary_data = vec![0x01, 0x02, 0x01, 0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
        let result = parser.parse_message(&binary_data).await;
        assert!(result.is_ok());
        
        let message = result.unwrap();
        assert!(message.is_text());
    }
    
    #[tokio::test]
    async fn test_message_dispatch() {
        let parser = MessageParser::with_default_callbacks();
        
        // 测试消息分发
        let message = UnifiedProtocolMessage::text("Test message".to_string());
        let result = parser.dispatch_message("session_123".to_string(), &message).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_callback_integration() {
        let message_callback = DefaultMessageCallback;
        let _event_callback = DefaultEventCallback;
        
        // 测试回调集成
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
        
        let result = message_callback.on_text(&text_message).await;
        assert!(result.is_ok());
    }
} 