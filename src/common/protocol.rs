//! Flare IM 协议模块
//!
//! 定义二进制消息协议格式

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 二进制协议消息
///
/// 使用二进制格式，最大化传输效率
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedProtocolMessage {
    /// 消息类型
    pub t: MessageType,
    /// 消息内容（二进制数据）
    pub c: Vec<u8>,
    /// 发送者ID（可选）
    pub s: Option<String>,
    /// 目标ID（可选）
    pub d: Option<String>,
    /// 时间戳（可选）
    pub ts: Option<DateTime<Utc>>,
}

/// 消息类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// 文本消息
    #[serde(rename = "t")]
    Text,
    /// 二进制消息
    #[serde(rename = "b")]
    Binary,
    /// 连接事件
    #[serde(rename = "c")]
    Connect,
    /// 断开连接事件
    #[serde(rename = "d")]
    Disconnect,
    /// 心跳
    #[serde(rename = "h")]
    Heartbeat,
    /// 心跳确认
    #[serde(rename = "ha")]
    HeartbeatAck,
    /// 通知
    #[serde(rename = "n")]
    Notification,
    /// 错误
    #[serde(rename = "e")]
    Error,
    /// 自定义事件
    #[serde(rename = "ce")]
    CustomEvent(String),
    /// 自定义消息
    #[serde(rename = "cm")]
    CustomMessage(String),
    /// REST 请求（带ID）
    #[serde(rename = "rq")]
    RestRequest,
    /// REST 响应（带ID）
    #[serde(rename = "rs")]
    RestResponse,
}

impl MessageType {
    /// 转换为简短字符串表示（用于序列化）
    pub fn to_string(&self) -> &str {
        match self {
            MessageType::Text => "t".as_ref(),
            MessageType::Binary => "b".as_ref(),
            MessageType::Connect => "c".as_ref(),
            MessageType::Disconnect => "d".as_ref(),
            MessageType::Heartbeat => "h".as_ref(),
            MessageType::HeartbeatAck => "ha".as_ref(),
            MessageType::Notification => "n".as_ref(),
            MessageType::Error => "e".as_ref(),
            MessageType::CustomEvent(_) => "ce".as_ref(),
            MessageType::CustomMessage(_) => "cm".as_ref(),
            MessageType::RestRequest => "rq".as_ref(),
            MessageType::RestResponse => "rs".as_ref(),
        }
    }
    
    /// 从简短字符串创建 MessageType
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "t" => Some(MessageType::Text),
            "b" => Some(MessageType::Binary),
            "c" => Some(MessageType::Connect),
            "d" => Some(MessageType::Disconnect),
            "h" => Some(MessageType::Heartbeat),
            "ha" => Some(MessageType::HeartbeatAck),
            "n" => Some(MessageType::Notification),
            "e" => Some(MessageType::Error),
            "rq" => Some(MessageType::RestRequest),
            "rs" => Some(MessageType::RestResponse),
            "ce" => Some(MessageType::CustomEvent("".to_string())),
            "cm" => Some(MessageType::CustomMessage("".to_string())),
            _ => None,
        }
    }
    
    /// 获取消息类型的描述
    pub fn description(&self) -> &'static str {
        match self {
            MessageType::Text => "文本消息",
            MessageType::Binary => "二进制消息",
            MessageType::Connect => "连接事件",
            MessageType::Disconnect => "断开连接事件",
            MessageType::Heartbeat => "心跳",
            MessageType::HeartbeatAck => "心跳确认",
            MessageType::Notification => "通知",
            MessageType::Error => "错误",
            MessageType::CustomEvent(_) => "自定义事件",
            MessageType::CustomMessage(_) => "自定义消息",
            MessageType::RestRequest => "REST 请求",
            MessageType::RestResponse => "REST 响应",
        }
    }
    
    /// 检查是否为系统消息类型
    pub fn is_system_message(&self) -> bool {
        matches!(self, 
            MessageType::Connect | 
            MessageType::Disconnect | 
            MessageType::Heartbeat | 
            MessageType::HeartbeatAck
        )
    }
    
    /// 检查是否为用户消息类型
    pub fn is_user_message(&self) -> bool {
        matches!(self, 
            MessageType::Text | 
            MessageType::Binary | 
            MessageType::CustomMessage(_) |
            MessageType::RestRequest |
            MessageType::RestResponse
        )
    }
    
    /// 检查是否为事件类型
    pub fn is_event(&self) -> bool {
        matches!(self, 
            MessageType::Connect | 
            MessageType::Disconnect | 
            MessageType::Heartbeat | 
            MessageType::HeartbeatAck |
            MessageType::CustomEvent(_)
        )
    }
}

impl UnifiedProtocolMessage {
    /// 创建新消息
    pub fn new(message_type: MessageType, content: Vec<u8>) -> Self {
        Self {
            t: message_type,
            c: content,
            s: None,
            d: None,
            ts: None,
        }
    }

    /// 设置发送者
    pub fn sender(mut self, sender: String) -> Self {
        self.s = Some(sender);
        self
    }

    /// 设置目标
    pub fn target(mut self, target: String) -> Self {
        self.d = Some(target);
        self
    }

    /// 设置时间戳
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.ts = Some(timestamp);
        self
    }

    // 便利构造方法
    /// 创建文本消息（UTF-8编码）
    pub fn text(text: String) -> Self {
        Self::new(MessageType::Text, text.into_bytes())
    }

    /// 创建二进制消息
    pub fn binary(data: Vec<u8>) -> Self {
        Self::new(MessageType::Binary, data)
    }

    /// 创建连接事件
    pub fn connect(user_id: String, session_id: String) -> Self {
        let user_id_clone = user_id.clone();
        let mut content = user_id.into_bytes();
        content.push(0); // 分隔符
        content.extend_from_slice(session_id.as_bytes());
        Self::new(MessageType::Connect, content)
            .sender(user_id_clone)
    }

    /// 创建带元数据的连接事件
    pub fn connect_with_metadata(user_id: String, session_id: String, metadata: Vec<u8>) -> Self {
        let user_id_clone = user_id.clone();
        let mut content = user_id.into_bytes();
        content.push(0); // 分隔符
        content.extend_from_slice(session_id.as_bytes());
        content.push(0); // 分隔符
        content.extend_from_slice(&metadata);
        Self::new(MessageType::Connect, content)
            .sender(user_id_clone)
    }

    /// 创建带JSON元数据的连接事件
    pub fn connect_with_json_metadata(user_id: String, session_id: String, metadata: serde_json::Value) -> Self {
        let metadata_bytes = serde_json::to_vec(&metadata).unwrap_or_default();
        Self::connect_with_metadata(user_id, session_id, metadata_bytes)
    }

    /// 创建带文本元数据的连接事件
    pub fn connect_with_text_metadata(user_id: String, session_id: String, metadata: String) -> Self {
        Self::connect_with_metadata(user_id, session_id, metadata.into_bytes())
    }

    /// 创建断开连接事件
    pub fn disconnect(user_id: String, session_id: String, reason: String) -> Self {
        let user_id_clone = user_id.clone();
        let mut content = user_id.into_bytes();
        content.push(0); // 分隔符
        content.extend_from_slice(session_id.as_bytes());
        content.push(0); // 分隔符
        content.extend_from_slice(reason.as_bytes());
        Self::new(MessageType::Disconnect, content)
            .sender(user_id_clone)
    }

    /// 创建心跳（空内容）
    pub fn heartbeat() -> Self {
        Self::new(MessageType::Heartbeat, vec![])
    }

    /// 创建心跳确认（空内容）
    pub fn heartbeat_ack() -> Self {
        Self::new(MessageType::HeartbeatAck, vec![])
    }

    /// 创建系统通知（二进制格式）
    pub fn notification(title: String, content: Vec<u8>) -> Self {
        let mut data = title.into_bytes();
        data.push(0); // 分隔符
        data.extend_from_slice(&content);
        Self::new(MessageType::Notification, data)
    }

    /// 创建文本通知（便利方法）
    pub fn notification_text(title: String, content: String) -> Self {
        Self::notification(title, content.into_bytes())
    }

    /// 创建错误（二进制格式）
    pub fn error(code: String, message: String) -> Self {
        let mut data = code.into_bytes();
        data.push(0); // 分隔符
        data.extend_from_slice(message.as_bytes());
        Self::new(MessageType::Error, data)
    }

    /// 创建自定义事件
    pub fn custom_event(event_name: String, data: Vec<u8>) -> Self {
        let event_name_clone = event_name.clone();
        let mut content = event_name.into_bytes();
        content.push(0); // 分隔符
        content.extend_from_slice(&data);
        Self::new(MessageType::CustomEvent(event_name_clone), content)
    }

    /// 创建自定义消息
    pub fn custom_message(message_type: String, data: Vec<u8>) -> Self {
        let message_type_clone = message_type.clone();
        let mut content = message_type.into_bytes();
        content.push(0); // 分隔符
        content.extend_from_slice(&data);
        Self::new(MessageType::CustomMessage(message_type_clone), content)
    }

    /// 创建 REST 请求（带ID）
    pub fn rest_request(id: String, method: String, path: String, body: Vec<u8>) -> Self {
        let mut content = id.into_bytes();
        content.push(0); // 分隔符
        content.extend_from_slice(method.as_bytes());
        content.push(0); // 分隔符
        content.extend_from_slice(path.as_bytes());
        content.push(0); // 分隔符
        content.extend_from_slice(&body);
        Self::new(MessageType::RestRequest, content)
    }
    
    /// 创建 REST 响应（带ID）
    pub fn rest_response(id: String, status_code: u16, body: Vec<u8>) -> Self {
        let mut content = id.into_bytes();
        content.push(0); // 分隔符
        content.extend_from_slice(status_code.to_string().as_bytes());
        content.push(0); // 分隔符
        content.extend_from_slice(&body);
        Self::new(MessageType::RestResponse, content)
    }
    /// 创建 JSON REST 请求（带ID）
    pub fn rest_request_json(id: String, method: String, path: String, json_body: serde_json::Value) -> Self {
        let body = serde_json::to_vec(&json_body).unwrap_or_default();
        Self::rest_request(id, method, path, body)
    }

    /// 创建 JSON REST 响应（带ID）
    pub fn rest_response_json(id: String, status_code: u16, json_body: serde_json::Value) -> Self {
        let body = serde_json::to_vec(&json_body).unwrap_or_default();
        Self::rest_response(id, status_code, body)
    }
    // 检查方法
    pub fn is_text(&self) -> bool {
        matches!(self.t, MessageType::Text)
    }

    pub fn is_binary(&self) -> bool {
        matches!(self.t, MessageType::Binary)
    }

    pub fn is_connect(&self) -> bool {
        matches!(self.t, MessageType::Connect)
    }

    pub fn is_disconnect(&self) -> bool {
        matches!(self.t, MessageType::Disconnect)
    }

    pub fn is_heartbeat(&self) -> bool {
        matches!(self.t, MessageType::Heartbeat)
    }

    pub fn is_heartbeat_ack(&self) -> bool {
        matches!(self.t, MessageType::HeartbeatAck)
    }

    pub fn is_notification(&self) -> bool {
        matches!(self.t, MessageType::Notification)
    }

    pub fn is_error(&self) -> bool {
        matches!(self.t, MessageType::Error)
    }

    pub fn is_custom_event(&self) -> bool {
        matches!(self.t, MessageType::CustomEvent(_))
    }

    pub fn is_rest_request(&self) -> bool {
        matches!(self.t, MessageType::RestRequest)
    }

    pub fn is_rest_response(&self) -> bool {
        matches!(self.t, MessageType::RestResponse)
    }

    // 私有辅助方法 - 解析分隔符分隔的内容
    fn parse_separated_content(&self, expected_type: bool) -> Option<(String, Vec<u8>)> {
        if !expected_type {
            return None;
        }
        
        if let Some(separator_pos) = self.c.iter().position(|&b| b == 0) {
            let first_part = String::from_utf8_lossy(&self.c[..separator_pos]).to_string();
            let second_part = self.c[separator_pos + 1..].to_vec();
            Some((first_part, second_part))
        } else {
            None
        }
    }

    // 获取方法
    /// 获取文本内容
    pub fn as_text(&self) -> Option<String> {
        if self.is_text() {
            String::from_utf8(self.c.clone()).ok()
        } else {
            None
        }
    }

    /// 获取二进制数据
    pub fn as_binary(&self) -> &Vec<u8> {
        &self.c
    }

    /// 获取连接信息
    pub fn as_connect_info(&self) -> Option<(String, String)> {
        if self.is_connect() {
            if let Some(first_sep) = self.c.iter().position(|&b| b == 0) {
                let user_id = String::from_utf8_lossy(&self.c[..first_sep]).to_string();
                let session_id = String::from_utf8_lossy(&self.c[first_sep + 1..]).to_string();
                Some((user_id, session_id))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取连接用户ID（向后兼容）
    pub fn as_connect_user(&self) -> Option<String> {
        if let Some((user_id, _)) = self.as_connect_info() {
            Some(user_id)
        } else {
            None
        }
    }

    /// 获取带元数据的连接信息
    pub fn as_connect_with_metadata(&self) -> Option<(String, String, Vec<u8>)> {
        if self.is_connect() {
            let mut parts = self.c.split(|&b| b == 0);
            if let (Some(user_id_bytes), Some(session_id_bytes), Some(metadata)) = 
                (parts.next(), parts.next(), parts.next()) {
                let user_id = String::from_utf8_lossy(user_id_bytes).to_string();
                let session_id = String::from_utf8_lossy(session_id_bytes).to_string();
                Some((user_id, session_id, metadata.to_vec()))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取带JSON元数据的连接信息
    pub fn as_connect_with_json_metadata(&self) -> Option<(String, String, serde_json::Value)> {
        if let Some((user_id, session_id, metadata)) = self.as_connect_with_metadata() {
            if let Ok(json_value) = serde_json::from_slice(&metadata) {
                Some((user_id, session_id, json_value))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取带文本元数据的连接信息
    pub fn as_connect_with_text_metadata(&self) -> Option<(String, String, String)> {
        if let Some((user_id, session_id, metadata)) = self.as_connect_with_metadata() {
            if let Ok(text) = String::from_utf8(metadata) {
                Some((user_id, session_id, text))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取断开连接信息
    pub fn as_disconnect_info(&self) -> Option<(String, String, String)> {
        if self.is_disconnect() {
            let mut parts = self.c.split(|&b| b == 0);
            if let (Some(user_id_bytes), Some(session_id_bytes), Some(reason_bytes)) = 
                (parts.next(), parts.next(), parts.next()) {
                let user_id = String::from_utf8_lossy(user_id_bytes).to_string();
                let session_id = String::from_utf8_lossy(session_id_bytes).to_string();
                let reason = String::from_utf8_lossy(reason_bytes).to_string();
                Some((user_id, session_id, reason))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取通知信息
    pub fn as_notification_info(&self) -> Option<(String, Vec<u8>)> {
        self.parse_separated_content(self.is_notification())
    }

    /// 获取文本通知信息
    pub fn as_notification_text(&self) -> Option<(String, String)> {
        if let Some((title, content)) = self.as_notification_info() {
            if let Ok(text) = String::from_utf8(content) {
                Some((title, text))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取错误信息
    pub fn as_error_info(&self) -> Option<(String, String)> {
        if let Some((code, message_bytes)) = self.parse_separated_content(self.is_error()) {
            if let Ok(message) = String::from_utf8(message_bytes) {
                Some((code, message))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取自定义事件信息
    pub fn as_custom_event_info(&self) -> Option<(String, Vec<u8>)> {
        self.parse_separated_content(self.is_custom_event())
    }

    /// 获取自定义消息信息
    pub fn as_custom_message_info(&self) -> Option<(String, Vec<u8>)> {
        self.parse_separated_content(matches!(self.t, MessageType::CustomMessage(_)))
    }

    /// 获取 REST 请求信息（带ID）
    pub fn as_rest_request_info(&self) -> Option<(String, String, String, Vec<u8>)> {
        if self.is_rest_request() {
            let mut parts = self.c.split(|&b| b == 0);
            if let (Some(id_bytes), Some(method_bytes), Some(path_bytes), Some(body)) = 
                (parts.next(), parts.next(), parts.next(), parts.next()) {
                let id = String::from_utf8_lossy(id_bytes).to_string();
                let method = String::from_utf8_lossy(method_bytes).to_string();
                let path = String::from_utf8_lossy(path_bytes).to_string();
                Some((id, method, path, body.to_vec()))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取 REST 响应信息（带ID）
    pub fn as_rest_response_info(&self) -> Option<(String, u16, Vec<u8>)> {
        if self.is_rest_response() {
            let mut parts = self.c.split(|&b| b == 0);
            if let (Some(id_bytes), Some(status_bytes), Some(body)) = 
                (parts.next(), parts.next(), parts.next()) {
                let id = String::from_utf8_lossy(id_bytes).to_string();
                if let Ok(status_code) = String::from_utf8_lossy(status_bytes).parse::<u16>() {
                    Some((id, status_code, body.to_vec()))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取 JSON REST 请求信息（带ID）
    pub fn as_rest_request_json(&self) -> Option<(String, String, String, serde_json::Value)> {
        if let Some((id, method, path, body)) = self.as_rest_request_info() {
            if let Ok(json_value) = serde_json::from_slice(&body) {
                Some((id, method, path, json_value))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取 JSON REST 响应信息（带ID）
    pub fn as_rest_response_json(&self) -> Option<(String, u16, serde_json::Value)> {
        if let Some((id, status_code, body)) = self.as_rest_response_info() {
            if let Ok(json_value) = serde_json::from_slice(&body) {
                Some((id, status_code, json_value))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取时间戳
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.ts.unwrap_or_else(Utc::now)
    }

    /// 获取发送者ID
    pub fn sender_id(&self) -> Option<&str> {
        self.s.as_deref()
    }

    /// 获取目标ID
    pub fn target_id(&self) -> Option<&str> {
        self.d.as_deref()
    }
}









