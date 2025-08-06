# Flare IM 客户端配置指南

## 概述

Flare IM 客户端提供了灵活的配置选项，支持多种协议选择模式和服务器地址配置。本文档详细介绍了如何配置客户端以满足不同的使用需求。

## 配置结构

### 核心配置组件

1. **服务器地址配置** (`ServerAddresses`)
2. **协议选择模式** (`ProtocolSelectionMode`)
3. **协议竞速配置** (`ProtocolRacingConfig`)
4. **TLS 配置** (`TlsConfig`)

## 服务器地址配置

### ServerAddresses 结构

```rust
pub struct ServerAddresses {
    pub quic_url: Option<String>,        // QUIC 服务器地址
    pub websocket_url: Option<String>,   // WebSocket 服务器地址
}
```

### 使用示例

```rust
use flare_im::client::config::ServerAddresses;

// 配置不同的服务器地址
let addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string())
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());
```

## 协议选择模式

### ProtocolSelectionMode 枚举

```rust
pub enum ProtocolSelectionMode {
    AutoRacing,                    // 自动协议竞速模式（默认）
    Specific(TransportProtocol),   // 指定协议模式
    Manual,                        // 手动协议选择模式
}
```

### 1. 自动协议竞速模式 (AutoRacing)

**特点：**
- 自动测试所有可用协议
- 根据性能指标选择最佳协议
- 支持自动降级
- 默认模式

**使用场景：**
- 需要最佳性能
- 网络环境复杂
- 希望自动优化

**示例：**
```rust
use flare_im::client::config::ProtocolSelectionMode;

let client = FlareIMClientBuilder::new("user123".to_string())
    .server_addresses(addresses)
    .protocol_selection_mode(ProtocolSelectionMode::AutoRacing)
    .build()?;
```

### 2. 指定协议模式 (Specific)

**特点：**
- 强制使用指定协议
- 不进行协议竞速
- 连接失败时不会自动切换

**使用场景：**
- 明确知道要使用的协议
- 网络环境稳定
- 性能要求不高

**示例：**
```rust
use flare_im::common::TransportProtocol;

let client = FlareIMClientBuilder::new("user456".to_string())
    .server_addresses(addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::QUIC))
    .build()?;
```

### 3. 手动模式 (Manual)

**特点：**
- 需要手动选择协议
- 不进行自动协议竞速
- 适合高级用户

**使用场景：**
- 需要精确控制
- 调试和测试
- 特殊需求

**示例：**
```rust
let client = FlareIMClientBuilder::new("user789".to_string())
    .server_addresses(addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Manual)
    .build()?;
```

## 协议竞速配置

### ProtocolRacingConfig 结构

```rust
pub struct ProtocolRacingConfig {
    pub enabled: bool,                    // 是否启用协议竞速
    pub timeout_ms: u64,                  // 竞速超时时间（毫秒）
    pub test_message_count: u32,          // 测试消息数量
    pub protocol_weights: ProtocolWeights, // 协议权重配置
    pub auto_fallback: bool,              // 是否启用自动降级
    pub racing_interval_ms: u64,          // 竞速间隔（毫秒）
}
```

### 协议权重配置

```rust
pub struct ProtocolWeights {
    pub quic_weight: f64,        // QUIC 权重 (0.0-1.0)
    pub websocket_weight: f64,   // WebSocket 权重 (0.0-1.0)
}
```

### 使用示例

```rust
use flare_im::client::config::{ProtocolRacingConfig, ProtocolWeights};

// 创建协议权重配置
let weights = ProtocolWeights::new()
    .with_quic_weight(0.8)      // QUIC 权重更高
    .with_websocket_weight(0.2);

// 创建协议竞速配置
let racing_config = ProtocolRacingConfig {
    enabled: true,
    timeout_ms: 3000,           // 3秒超时
    test_message_count: 5,      // 测试5条消息
    protocol_weights: weights,
    auto_fallback: true,
    racing_interval_ms: 60000,  // 每分钟重新竞速
};

let client = FlareIMClientBuilder::new("user_custom".to_string())
    .server_addresses(addresses)
    .protocol_racing(racing_config)
    .build()?;
```

## TLS 配置

### TlsConfig 结构

```rust
pub struct TlsConfig {
    pub verify_server_cert: bool,         // 是否验证服务器证书
    pub server_cert_path: Option<String>, // 服务端证书路径
    pub server_name: Option<String>,      // 服务器名称指示（SNI）
    pub client_cert_path: Option<String>, // 客户端证书路径
    pub client_key_path: Option<String>,  // 客户端私钥路径
}
```

### 使用示例

```rust
use flare_im::client::config::TlsConfig;

// 创建 TLS 配置
let tls_config = TlsConfig::new(true)  // 验证服务器证书
    .with_server_cert("/path/to/server.crt".to_string())
    .with_server_name("example.com".to_string())
    .with_client_cert(
        "/path/to/client.crt".to_string(),
        "/path/to/client.key".to_string()
    );

let client = FlareIMClientBuilder::new("user_tls".to_string())
    .server_addresses(addresses)
    .tls_config(tls_config)
    .build()?;
```

## 完整配置示例

### 基础配置

```rust
use flare_im::client::FlareIMClientBuilder;
use flare_im::client::config::{ServerAddresses, ProtocolSelectionMode};

let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string())
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

let client = FlareIMClientBuilder::new("user123".to_string())
    .server_addresses(server_addresses)
    .connection_timeout(5000)
    .heartbeat_interval(30000)
    .max_reconnect_attempts(3)
    .auto_reconnect(true)
    .build()?;
```

### 高级配置

```rust
use flare_im::client::FlareIMClientBuilder;
use flare_im::client::config::{
    ServerAddresses, ProtocolSelectionMode, ProtocolRacingConfig, 
    ProtocolWeights, TlsConfig
};

// 服务器地址配置
let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string())
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

// 协议权重配置
let protocol_weights = ProtocolWeights::new()
    .with_quic_weight(0.7)
    .with_websocket_weight(0.3);

// 协议竞速配置
let racing_config = ProtocolRacingConfig {
    enabled: true,
    timeout_ms: 5000,
    test_message_count: 10,
    protocol_weights,
    auto_fallback: true,
    racing_interval_ms: 30000,
};

// TLS 配置
let tls_config = TlsConfig::new(true)
    .with_server_name("example.com".to_string());

// 创建客户端
let client = FlareIMClientBuilder::new("user_advanced".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::AutoRacing)
    .protocol_racing(racing_config)
    .connection_timeout(5000)
    .heartbeat_interval(30000)
    .max_reconnect_attempts(5)
    .reconnect_delay(1000)
    .auto_reconnect(true)
    .message_retry(3, 1000)
    .buffer_size(8192)
    .compression(true)
    .encryption(true)
    .tls_config(tls_config)
    .build()?;
```

## 配置验证

客户端配置会自动进行验证，确保：

1. **用户ID** 不为空
2. **服务器地址** 至少配置一个
3. **协议选择模式** 与服务器地址配置匹配
4. **超时时间** 大于0
5. **缓冲区大小** 大于0

如果配置无效，`build()` 方法会返回错误。

## 最佳实践

### 1. 生产环境配置

```rust
// 生产环境推荐配置
let client = FlareIMClientBuilder::new("production_user".to_string())
    .server_addresses(ServerAddresses::new()
        .with_quic_url("flare-core://quic.prod.com:8081".to_string())
        .with_websocket_url("flare-core://ws.prod.com:8080".to_string()))
    .protocol_selection_mode(ProtocolSelectionMode::AutoRacing)
    .connection_timeout(10000)        // 10秒超时
    .heartbeat_interval(30000)        // 30秒心跳
    .max_reconnect_attempts(5)        // 最多重连5次
    .auto_reconnect(true)
    .message_retry(3, 2000)          // 消息重试3次，间隔2秒
    .buffer_size(16384)              // 16KB缓冲区
    .compression(true)
    .encryption(true)
    .tls(true)                       // 启用TLS
    .build()?;
```

### 2. 开发环境配置

```rust
// 开发环境推荐配置
let client = FlareIMClientBuilder::new("dev_user".to_string())
    .server_addresses(ServerAddresses::new()
        .with_quic_url("flare-core://localhost:8081".to_string())
        .with_websocket_url("flare-core://localhost:8080".to_string()))
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::WebSocket))
    .connection_timeout(3000)         // 3秒超时
    .heartbeat_interval(60000)        // 60秒心跳
    .max_reconnect_attempts(3)        // 最多重连3次
    .auto_reconnect(false)            // 开发时禁用自动重连
    .build()?;
```

### 3. 测试环境配置

```rust
// 测试环境推荐配置
let client = FlareIMClientBuilder::new("test_user".to_string())
    .server_addresses(ServerAddresses::new()
        .with_quic_url("flare-core://quic.test.com:8081".to_string())
        .with_websocket_url("flare-core://ws.test.com:8080".to_string()))
    .protocol_selection_mode(ProtocolSelectionMode::AutoRacing)
    .connection_timeout(5000)
    .heartbeat_interval(30000)
    .max_reconnect_attempts(2)        // 测试时减少重连次数
    .auto_reconnect(true)
    .message_retry(1, 1000)          // 测试时减少重试次数
    .build()?;
```

## 错误处理

配置错误会返回具体的错误信息：

```rust
match FlareIMClientBuilder::new("user".to_string()).build() {
    Ok(client) => {
        // 配置成功
    }
    Err(error) => {
        eprintln!("配置错误: {}", error);
        // 处理配置错误
    }
}
```

常见错误：
- `用户ID不能为空`
- `至少需要配置一个服务器地址`
- `指定的协议 QUIC 没有对应的服务器地址`
- `连接超时时间必须大于0`

## 总结

Flare IM 客户端提供了灵活的配置选项，可以根据不同的使用场景选择合适的配置。建议：

1. **生产环境** 使用自动协议竞速模式，配置多个服务器地址
2. **开发环境** 使用指定协议模式，简化配置
3. **测试环境** 使用自动协议竞速模式，但减少重试次数
4. **始终启用 TLS** 以确保安全性
5. **根据网络环境调整超时和重试参数** 

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

### 配置迁移

```rust
// 旧版本
let config = ClientConfig::new("user123".to_string(), "flare-core://localhost".to_string())
    .with_preferred_protocol(TransportProtocol::QUIC)
    .with_auto_switch(true);

// 新版本
let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://localhost:8081".to_string())
    .with_websocket_url("flare-core://localhost:8080".to_string());

let config = ClientConfig::new("user123".to_string())
    .with_server_addresses(server_addresses)
    .with_protocol_selection_mode(ProtocolSelectionMode::AutoRacing);
``` 