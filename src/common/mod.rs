//! Flare IM 公共模块
//!
//! 提供客户端和服务端共享的类型定义、协议实现等

pub mod conn;
pub mod error;
pub mod events;
pub mod protocol;
pub mod traits;
pub mod types;

// 重新导出常用类型
pub use conn::{Connection, ConnectionEvent, ProtoMessage, Platform, ConnectionConfig};
pub use error::{Result, FlareError};
pub use events::{ConnectionEvent as EventConnectionEvent, MessageEvent, AuthEvent, ErrorEvent, PerformanceEvent, SystemEvent};
pub use protocol::Message;
pub use traits::{ConnectionManager, MessageHandler, EventListener, AuthHandler};
pub use types::{TransportProtocol, ConnectionStatus, ConnectionInfo}; 