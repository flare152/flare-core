# Flare IM 客户端示例

本目录包含了三个独立的客户端示例，展示了 Flare IM 客户端的三种不同使用方式。

## 示例文件

### 1. WebSocket 客户端 (`websocket_client.rs`)

**特点：**
- 指定使用 WebSocket 协议
- 只配置 WebSocket 服务器地址
- 不进行协议竞速，直接使用 WebSocket

**运行方式：**
```bash
cargo run --example websocket_client
```

**配置：**
```rust
let server_addresses = ServerAddresses::new()
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

let client = FlareIMClientBuilder::new("websocket_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::WebSocket))
    .build()?;
```

### 2. QUIC 客户端 (`quic_client.rs`)

**特点：**
- 指定使用 QUIC 协议
- 只配置 QUIC 服务器地址
- 不进行协议竞速，直接使用 QUIC

**运行方式：**
```bash
cargo run --example quic_client
```

**配置：**
```rust
let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string());

let client = FlareIMClientBuilder::new("quic_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::QUIC))
    .build()?;
```

### 3. 协议竞速客户端 (`auto_racing_client.rs`)

**特点：**
- 自动选择最佳协议
- 配置 QUIC 和 WebSocket 两个服务器地址
- 进行协议竞速测试，选择性能最佳的协议

**运行方式：**
```bash
cargo run --example auto_racing_client
```

**配置：**
```rust
let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string())
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

let client = FlareIMClientBuilder::new("racing_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::AutoRacing)
    .build()?;
```

## 功能特性

所有示例都包含以下完整功能：

### 1. 事件处理
- 连接成功/失败事件
- 重连事件
- 消息发送/接收事件
- 心跳事件
- 协议切换事件
- 错误处理

### 2. 消息发送
- 文本消息发送
- 二进制消息发送
- 自定义消息发送
- 消息发送结果处理

### 3. 状态监控
- 连接状态检查
- 连接统计信息
- 协议性能指标
- 消息队列长度

### 4. 心跳机制
- 定期发送心跳消息
- 连接状态监控
- 自动重连机制

## 快速开始

### 1. 环境准备

确保您的系统已安装：
- Rust (1.70+)
- OpenSSL
- Cargo

### 2. 生成证书

```bash
# 生成 TLS 证书（用于 QUIC 连接）
./scripts/generate_certs.sh
```

### 3. 运行演示

#### 完整演示
```bash
# 运行完整演示（服务器 + 所有客户端）
./examples/demo.sh
```

#### 单独运行
```bash
# 仅启动服务器
./examples/demo.sh -s

# 运行 WebSocket 客户端
./examples/demo.sh -w

# 运行 QUIC 客户端
./examples/demo.sh -q

# 运行协议竞速客户端
./examples/demo.sh -a
```

#### 连接测试
```bash
# 快速连接测试
./examples/test_connection.sh
```

### 4. 手动运行

#### 启动服务器
```bash
# 在一个终端中启动服务器
cargo run --example server_example
```

#### 运行客户端
```bash
# 在另一个终端中运行客户端
cargo run --example websocket_client
cargo run --example quic_client
cargo run --example auto_racing_client
```

## 运行环境要求

### 依赖项
确保 `Cargo.toml` 中包含以下依赖：

```toml
[dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "net", "time", "macros"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### 服务器配置

示例中的服务器配置：
- **WebSocket 服务器**: `ws://127.0.0.1:4000`
- **QUIC 服务器**: `quic://127.0.0.1:4010`
- **TLS 证书**: `certs/server.crt` 和 `certs/server.key`

## 自定义配置

### 修改服务器地址

```rust
// 修改为您的服务器地址
let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://your-quic-server:8081".to_string())
    .with_websocket_url("flare-core://your-websocket-server:8080".to_string());
```

### 调整连接参数

```rust
let client = FlareIMClientBuilder::new("user".to_string())
    .server_addresses(server_addresses)
    .connection_timeout(10000)        // 10秒超时
    .heartbeat_interval(60000)        // 60秒心跳
    .max_reconnect_attempts(5)        // 最多重连5次
    .auto_reconnect(true)
    .build()?;
```

### 自定义协议竞速配置

```rust
let protocol_weights = ProtocolWeights::new()
    .with_quic_weight(0.8)      // QUIC 权重更高
    .with_websocket_weight(0.2);

let racing_config = ProtocolRacingConfig {
    enabled: true,
    timeout_ms: 5000,           // 5秒超时
    test_message_count: 10,     // 测试10条消息
    protocol_weights,
    auto_fallback: true,
    racing_interval_ms: 30000,  // 每30秒重新竞速
};
```

## 日志输出

所有示例都会输出详细的日志信息：

```
🚀 启动 WebSocket 客户端示例
[WebSocket] 连接成功，使用协议: WebSocket
发送文本消息...
[WebSocket] 消息发送成功: 12345678-1234-1234-1234-123456789abc
文本消息发送成功: 12345678-1234-1234-1234-123456789abc
连接统计: 发送消息 1, 接收消息 0, 心跳次数 0, 错误次数 0
WebSocket 客户端运行中，按 Ctrl+C 停止...
✅ WebSocket 客户端示例运行完成
```

## 故障排除

### 常见问题

1. **连接失败**
   - 检查服务器地址是否正确
   - 确认服务器是否正在运行
   - 检查网络连接

2. **协议竞速失败**
   - 确保至少配置了一个协议地址
   - 检查协议竞速配置参数
   - 查看详细错误日志

3. **消息发送失败**
   - 检查连接状态
   - 确认目标用户ID是否正确
   - 查看消息队列长度

### 调试模式

启用详细日志：

```bash
RUST_LOG=debug cargo run --example websocket_client
```

## 扩展示例

基于这些基础示例，您可以：

1. **添加消息接收处理**
2. **实现自定义消息类型**
3. **添加用户认证**
4. **实现群组消息功能**
5. **添加文件传输功能**

## 贡献

如果您有改进建议或发现问题，请：

1. 提交 Issue
2. 创建 Pull Request
3. 更新文档

## 许可证

本示例代码遵循 MIT 许可证。 