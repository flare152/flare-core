//! Flare IM - 高性能、可扩展的即时通讯工具包
//!
//! ## 功能特性
//!
//! - 🚀 **高性能**: 支持百万级并发连接，基于 Rust 零成本抽象
//! - 🔄 **协议竞速**: 优先使用 QUIC，自动降级到 WebSocket
//! - 🛠️ **可扩展**: 基于 trait 的模块化设计，支持自定义实现
//! - 🔧 **灵活配置**: 提供默认实现，支持 Redis、数据库等扩展
//!
//! ## 使用方式
//!
//! ```toml
//! [dependencies]
//! flare-core = { version = "0.1", features = ["client", "server"] }
//! ```
//!

// 公共模块
pub mod common;

// 客户端模块 (需要 client feature)
#[cfg(feature = "client")]
pub mod client;

// 服务端模块 (需要 server feature)
#[cfg(feature = "server")]
pub mod server;

// 重新导出常用类型
pub use common::{
    TransportProtocol,Result, FlareError, UnifiedProtocolMessage
};

#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub use server::{FlareIMServer, FlareIMServerBuilder};

// 版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME"); 