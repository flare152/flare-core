# Flare IM 客户端示例总结

## 概述

我们创建了三个独立的客户端示例，分别展示了 Flare IM 客户端的三种不同使用方式。每个示例都是完整的、可运行的客户端，包含完整的事件处理和消息发送接收功能。

## 示例文件结构

```
examples/
├── websocket_client.rs      # WebSocket 专用客户端
├── quic_client.rs          # QUIC 专用客户端
├── auto_racing_client.rs   # 协议竞速客户端
└── README.md              # 示例说明文档
```

## 1. WebSocket 客户端 (`websocket_client.rs`)

### 特点
- **指定协议**：强制使用 WebSocket 协议
- **单一地址**：只配置 WebSocket 服务器地址
- **无竞速**：不进行协议竞速测试
- **直接连接**：直接连接到 WebSocket 服务器

### 配置示例
```rust
let server_addresses = ServerAddresses::new()
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

let client = FlareIMClientBuilder::new("websocket_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::WebSocket))
    .build()?;
```

### 适用场景
- 明确知道要使用 WebSocket 协议
- 网络环境稳定，不需要协议切换
- 对性能要求不高，优先考虑兼容性
- 开发和测试环境

### 运行方式
```bash
cargo run --example websocket_client
```

## 2. QUIC 客户端 (`quic_client.rs`)

### 特点
- **指定协议**：强制使用 QUIC 协议
- **单一地址**：只配置 QUIC 服务器地址
- **无竞速**：不进行协议竞速测试
- **高性能**：直接使用 QUIC 的高性能特性

### 配置示例
```rust
let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string());

let client = FlareIMClientBuilder::new("quic_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::QUIC))
    .build()?;
```

### 适用场景
- 明确知道要使用 QUIC 协议
- 对性能有较高要求
- 网络环境支持 QUIC
- 生产环境的高性能应用

### 运行方式
```bash
cargo run --example quic_client
```

## 3. 协议竞速客户端 (`auto_racing_client.rs`)

### 特点
- **自动选择**：自动测试并选择最佳协议
- **双地址配置**：同时配置 QUIC 和 WebSocket 地址
- **智能竞速**：进行协议性能测试
- **自动降级**：支持协议自动降级

### 配置示例
```rust
// 协议权重配置
let protocol_weights = ProtocolWeights::new()
    .with_quic_weight(0.7)      // QUIC 权重更高
    .with_websocket_weight(0.3);

// 协议竞速配置
let racing_config = ProtocolRacingConfig {
    enabled: true,
    timeout_ms: 3000,           // 3秒超时
    test_message_count: 5,      // 测试5条消息
    protocol_weights,
    auto_fallback: true,
    racing_interval_ms: 60000,  // 每分钟重新竞速
};

let server_addresses = ServerAddresses::new()
    .with_quic_url("flare-core://quic.example.com:8081".to_string())
    .with_websocket_url("flare-core://ws.example.com:8080".to_string());

let client = FlareIMClientBuilder::new("racing_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::AutoRacing)
    .protocol_racing(racing_config)
    .build()?;
```

### 适用场景
- 需要最佳性能
- 网络环境复杂
- 希望自动优化
- 生产环境的高可靠性应用

### 运行方式
```bash
cargo run --example auto_racing_client
```

## 共同功能特性

所有三个示例都包含以下完整功能：

### 1. 完整的事件处理
```rust
let event_callback: Arc<ClientEventCallback> = Arc::new(Box::new(|event| {
    match event {
        ClientEvent::Connected(protocol) => {
            info!("连接成功，使用协议: {:?}", protocol);
        }
        ClientEvent::MessageReceived(message) => {
            info!("收到消息: ID={}, 类型={}, 内容={}", 
                  message.id, message.message_type, 
                  String::from_utf8_lossy(&message.payload));
        }
        // ... 其他事件处理
    }
}));
```

### 2. 多种消息发送
- **文本消息**：`client.send_text_message("target", "Hello")`
- **二进制消息**：`client.send_binary_message("target", data, "binary")`
- **自定义消息**：`client.send_message("target", custom_message)`

### 3. 状态监控
- 连接状态检查
- 连接统计信息
- 协议性能指标（协议竞速客户端）
- 消息队列长度

### 4. 心跳机制
- 定期发送心跳消息
- 连接状态监控
- 自动重连机制

## 性能对比

| 特性 | WebSocket 客户端 | QUIC 客户端 | 协议竞速客户端 |
|------|------------------|-------------|----------------|
| 连接速度 | 中等 | 快速 | 自动选择最佳 |
| 兼容性 | 高 | 中等 | 高 |
| 性能 | 中等 | 高 | 最优 |
| 复杂度 | 低 | 低 | 中等 |
| 可靠性 | 高 | 高 | 最高 |

## 使用建议

### 选择 WebSocket 客户端的情况
- 需要广泛的浏览器兼容性
- 网络环境对 QUIC 支持有限
- 开发和测试阶段
- 对性能要求不高

### 选择 QUIC 客户端的情况
- 现代网络环境
- 对性能有较高要求
- 服务器支持 QUIC
- 生产环境应用

### 选择协议竞速客户端的情况
- 需要最佳用户体验
- 网络环境复杂多变
- 高可靠性要求
- 生产环境的关键应用

## 自定义和扩展

### 修改服务器地址
```rust
// 修改为您的实际服务器地址
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

### 添加自定义事件处理
```rust
// 在事件回调中添加自定义逻辑
ClientEvent::MessageReceived(message) => {
    // 自定义消息处理逻辑
    if message.message_type == "custom" {
        // 处理自定义消息
        handle_custom_message(&message);
    }
    info!("收到消息: {:?}", message);
}
```

## 故障排除

### 常见问题

1. **连接失败**
   - 检查服务器地址和端口
   - 确认服务器正在运行
   - 检查网络连接和防火墙

2. **协议竞速失败**
   - 确保至少配置了一个协议地址
   - 检查协议竞速配置参数
   - 查看详细错误日志

3. **消息发送失败**
   - 检查连接状态
   - 确认目标用户ID
   - 查看消息队列长度

### 调试技巧

1. **启用详细日志**
   ```bash
   RUST_LOG=debug cargo run --example websocket_client
   ```

2. **监控连接状态**
   ```rust
   let status = client.get_status().await;
   info!("当前连接状态: {:?}", status);
   ```

3. **查看协议性能**
   ```rust
   let metrics = client.get_all_protocol_metrics().await;
   info!("协议性能指标: {:?}", metrics);
   ```

## 总结

这三个客户端示例提供了 Flare IM 的完整使用方案：

1. **WebSocket 客户端**：适合需要广泛兼容性的场景
2. **QUIC 客户端**：适合对性能有高要求的场景
3. **协议竞速客户端**：适合需要最佳用户体验的场景

每个示例都是独立的、完整的，可以直接运行和扩展。用户可以根据具体需求选择合适的示例作为起点，进行自定义开发。 