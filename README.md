# Flare IM - 高性能即时通讯工具包

[![Rust](https://img.shields.io/badge/Rust-1.70+-blue.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/flare-core)](https://crates.io/crates/flare-core)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/flare-core)

Flare IM 是一个基于 Rust 语言开发的高性能即时通讯工具包，采用 QUIC 协议和 WebSocket 技术，提供可扩展的架构设计。

## 🚀 特性

- **高性能**: 支持百万级并发连接，基于 Rust 零成本抽象
- **协议竞速**: 优先使用 QUIC，自动降级到 WebSocket
- **多平台**: 统一的代码库支持 Android、iOS、Web
- **可扩展**: 基于 trait 的模块化设计，支持自定义实现
- **灵活配置**: 提供默认实现，支持 Redis、数据库等扩展
- **多平台支持**: 支持移动端、桌面端、Web端等多种平台
- **智能同步**: 支持消息和状态的多端实时同步
- **统一消息格式**: 使用 `ProtoMessage` 二进制格式，支持任意类型的消息内容
- **用户自定义**: 消息格式完全由用户定义，提高灵活性

## 📦 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
flare-core = { version = "0.1", features = ["client", "server"] }
```

### 功能特性

- `client` - 启用客户端功能
- `server` - 启用服务端功能
- `common` - 启用公共功能（默认启用）

## 🛠️ 快速开始

### 客户端使用

```rust
use flare_im::client::Client;
use flare_im::common::TransportProtocol;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建客户端
    let client = Client::new("user123").await?;
    
    // 连接到服务器
    let protocol = client.connect("flare-core://localhost").await?;
    println!("连接成功，使用协议: {:?}", protocol);
    
    // 发送消息
    let result = client.send_message("user456", "Hello, World!").await?;
    println!("消息发送结果: {:?}", result);
    
    Ok(())
}
```

### 服务端使用

```rust
use flare_im::server::{FlareIMServer, FlareIMServerBuilder};
use flare_im::server::config::ServerConfigBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建服务器配置
    let config = ServerConfigBuilder::new()
        .websocket_addr("127.0.0.1:8080".parse().unwrap())
        .quic_addr("127.0.0.1:8081".parse().unwrap())
        .enable_websocket(true)
        .enable_quic(true)
        .max_connections(10000)
        .build();
    
    // 创建并启动服务器
    let server = FlareIMServerBuilder::new()
        .with_config(config)
        .build();
    
    println!("启动 Flare IM 服务器...");
    server.start().await?;
    
    Ok(())
}
```

### 自定义消息处理器

```rust
use flare_im::server::handlers::{MessageHandler, EventHandler};
use flare_im::common::conn::{ProtoMessage, Platform, ConnectionEvent};
use async_trait::async_trait;

struct MyMessageHandler;

#[async_trait]
impl MessageHandler for MyMessageHandler {
    async fn handle_message(&self, user_id: &str, message: ProtoMessage) -> Result<ProtoMessage, flare_im::Result> {
        println!("收到来自用户 {} 的消息: {:?}", user_id, message);
        
        // 返回响应
        Ok(ProtoMessage::new(
            uuid::Uuid::new_v4().to_string(),
            "response".to_string(),
            message.payload,
        ))
    }
    
    async fn handle_user_connect(&self, user_id: &str, session_id: &str, platform: Platform) -> Result<(), flare_im::Result> {
        println!("用户 {} 连接，会话: {}，平台: {:?}", user_id, session_id, platform);
        Ok(())
    }
    
    async fn handle_user_disconnect(&self, user_id: &str, session_id: &str) -> Result<(), flare_im::Result> {
        println!("用户 {} 断开，会话: {}", user_id, session_id);
        Ok(())
    }
    
    async fn handle_heartbeat(&self, user_id: &str, session_id: &str) -> Result<(), flare_im::Result> {
        println!("用户 {} 心跳，会话: {}", user_id, session_id);
        Ok(())
    }
}

// 在服务器中使用自定义处理器
let server = FlareIMServerBuilder::new()
    .with_config(config)
    .with_message_handler(Arc::new(MyMessageHandler))
    .build();
```

## 📚 文档

- [API 文档](https://docs.rs/flare-core)
- [示例代码](./examples/)

## 🔧 配置

### 客户端配置

```rust
use flare_im::client::config::ClientConfig;

let config = ClientConfig {
    user_id: "user123".to_string(),
    server_url: "flare-core://localhost".to_string(),
    connection_timeout_ms: 30000,
    heartbeat_interval_ms: 30000,
    max_reconnect_attempts: 5,
    reconnect_delay_ms: 1000,
    ..Default::default()
};
```

### 服务端配置

```rust
use flare_im::server::config::ServerConfigBuilder;

let config = ServerConfigBuilder::new()
    .websocket_addr("127.0.0.1:8080".parse().unwrap())
    .quic_addr("127.0.0.1:8081".parse().unwrap())
    .enable_websocket(true)
    .enable_quic(true)
    .max_connections(10000)
    .enable_auth(true)
    .log_level("info".to_string())
    .build();
```

## 🏗️ 架构

Flare IM 采用模块化设计，主要包含以下模块：

- **common**: 公共类型、错误处理、连接管理
- **client**: 客户端实现，支持 QUIC 和 WebSocket
- **server**: 服务端实现，支持多种处理器

### 核心特性

- **协议竞速**: 自动选择最佳传输协议
- **连接管理**: 智能连接池和重连机制
- **消息路由**: 高效的消息分发系统
- **事件系统**: 完整的事件处理机制

## 🤝 贡献

欢迎贡献代码！请查看 [贡献指南](CONTRIBUTING.md) 了解详情。

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🔗 相关链接

- [Crates.io](https://crates.io/crates/flare-core)
- [API 文档](https://docs.rs/flare-core)
- [GitHub 仓库](https://github.com/your-username/flare-core) 