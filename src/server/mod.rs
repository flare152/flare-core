//! Flare IM 服务端模块
//!
//! 提供高性能、可扩展的服务端实现

pub mod conn_manager;
pub mod config;
pub mod websocket_server;
pub mod quic_server;
pub mod message_processor;

mod core;

pub use core::{FlareIMServer, FlareIMServerBuilder};
pub use conn_manager::{
    ServerConnectionManager, ServerConnectionManagerConfig, ServerConnectionInfo,
    ServerConnectionManagerStats, ConnectionEventCallback, MemoryServerConnectionManager
};

// 重新导出配置和处理器
pub use config::{
    ServerConfig, ServerProtocolConfig, WebSocketServerConfig, QuicServerConfig,
    ConnectionManagerConfig, AuthConfig, LoggingConfig, AuthMethod, ServerConfigBuilder
};

// 重新导出消息处理器
pub use message_processor::{MessageProcessor, MessageProcessorBuilder, MessageStats}; 