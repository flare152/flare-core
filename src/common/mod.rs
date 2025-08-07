//! Flare IM 公共模块
//!
//! 提供客户端和服务端共享的类型定义、协议实现等

pub mod conn;
pub mod error;
pub mod protocol;
pub mod traits;
pub mod types;
pub mod tls;
pub mod message_parser;
pub mod callback;

// 重新导出常用类型
pub use conn::{Connection, ConnectionConfig};
pub use error::{Result, FlareError, ErrorCode, LocalizedError, ErrorBuilder};
pub use protocol::UnifiedProtocolMessage;
pub use message_parser::{
    MessageParser, CustomParser, ParseResult,
    MessageParserBuilder, MessageParserFactory
};
pub use callback::{
    MessageCallback, EventCallback, NotificationCallback, ErrorCallback, RestCallback,
    DefaultMessageCallback, DefaultEventCallback, DefaultNotificationCallback, DefaultErrorCallback, DefaultRestCallback,
    TextMessage, BinaryMessage, ConnectEvent, DisconnectEvent,
    HeartbeatEvent, NotificationMessage, ErrorMessage,
    CustomEvent, CustomMessage, CustomNotification, 
    RestRequest, RestResponse, MessageMetadata
};
pub use traits::{ConnectionManager, AuthHandler};
pub use types::{
    TransportProtocol, ConnectionStatus, ConnectionInfo, 
    ProtocolSelection, Platform
}; 