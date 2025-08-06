# Flare IM 客户端简化配置总结

## 概述

根据您的要求，我们已经简化了客户端配置，移除了通用地址的概念，现在只支持为 QUIC 和 WebSocket 协议单独设置服务器地址。

## 简化后的配置结构

### ServerAddresses 结构

```rust
pub struct ServerAddresses {
    pub quic_url: Option<String>,        // QUIC 服务器地址
    pub websocket_url: Option<String>,   // WebSocket 服务器地址
}
```

**特点：**
- 只包含 QUIC 和 WebSocket 的地址配置
- 移除了通用地址字段
- 更简洁和直观的配置结构

## 配置方法

### 创建服务器地址配置

```rust
use flare_core::client::config::ServerAddresses;

// 方式1：链式调用
let addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string())
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

// 方式2：分别设置
let mut addresses = ServerAddresses::new();
addresses.quic_url = Some("flare-core://quic.example.com:8081".to_string());
addresses.websocket_url = Some("flare-core://ws.example.com:8080".to_string());
```

### 获取协议地址

```rust
// 获取指定协议的地址
if let Some(quic_url) = addresses.get_protocol_url(TransportProtocol::QUIC) {
    println!("QUIC 地址: {}", quic_url);
}

if let Some(ws_url) = addresses.get_protocol_url(TransportProtocol::WebSocket) {
    println!("WebSocket 地址: {}", ws_url);
}
```

## 使用示例

### 1. 基础配置

```rust
use flare_core::client::FlareIMClientBuilder;
use flare_core::client::config::ServerAddresses;

let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string())
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

let client = FlareIMClientBuilder::new("user123".to_string())
    .server_addresses(server_addresses)
    .build()?;
```

### 2. 只配置 QUIC

```rust
let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string());

let client = FlareIMClientBuilder::new("user456".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::QUIC))
    .build()?;
```

### 3. 只配置 WebSocket

```rust
let server_addresses = ServerAddresses::new()
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

let client = FlareIMClientBuilder::new("user789".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::WebSocket))
    .build()?;
```

### 4. 协议竞速模式

```rust
let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string())
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

let client = FlareIMClientBuilder::new("user_racing".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::AutoRacing)
    .build()?;
```

## 验证规则

配置验证确保：

1. **至少配置一个协议地址**：QUIC 或 WebSocket 至少有一个
2. **协议选择模式匹配**：如果指定特定协议，必须配置对应的地址
3. **地址格式正确**：URL 格式验证

## 错误处理

```rust
// 错误示例：没有配置任何地址
let addresses = ServerAddresses::new();
match addresses.validate() {
    Ok(_) => println!("配置有效"),
    Err(e) => println!("配置错误: {}", e), // "至少需要配置一个服务器地址"
}

// 错误示例：指定协议但没有对应地址
let addresses = ServerAddresses::new()
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

let config = ClientConfig::new("user".to_string())
    .with_server_addresses(addresses)
    .with_protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::QUIC));

match config.validate() {
    Ok(_) => println!("配置有效"),
    Err(e) => println!("配置错误: {}", e), // "指定的协议 QUIC 没有对应的服务器地址"
}
```

## 优势

### 1. 简化性
- 移除了不必要的通用地址概念
- 配置结构更加直观
- 减少了配置的复杂性

### 2. 明确性
- 每个协议都有明确的地址配置
- 避免了地址解析的歧义
- 更容易理解和维护

### 3. 灵活性
- 可以只配置需要的协议
- 支持协议竞速和指定协议模式
- 适应不同的部署场景

## 迁移指南

### 从通用地址迁移

```rust
// 旧方式（已移除）
let addresses = ServerAddresses::new()
    .with_generic_url("flare-core://example.com".to_string());

// 新方式
let addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string())
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());
```

### 从旧版本迁移

```rust
// 旧版本
let client = FlareIMClient::new("user123".to_string(), "flare-core://localhost".to_string())?;

// 新版本
let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://localhost:8081".to_string())
    .with_websocket_url("flare-core://localhost:8080".to_string());

let client = FlareIMClientBuilder::new("user123".to_string())
    .server_addresses(server_addresses)
    .build()?;
```

## 总结

简化后的配置结构更加清晰和直观：

1. **只支持协议特定地址**：QUIC 和 WebSocket 分别配置
2. **移除通用地址**：避免了配置的复杂性
3. **保持灵活性**：支持协议竞速和指定协议模式
4. **更好的验证**：确保配置的正确性和完整性

这种简化的设计使得配置更加直观，减少了用户的困惑，同时保持了系统的灵活性和功能性。 