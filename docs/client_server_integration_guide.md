# Flare IM 客户端服务器集成指南

## 概述

本指南详细说明了如何将 Flare IM 客户端与服务器进行集成，确保它们能够正常连接和使用。

## 架构概览

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   WebSocket     │    │      QUIC       │    │   协议竞速      │
│    客户端       │    │     客户端       │    │     客户端       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │   Flare IM      │
                    │     服务器       │
                    └─────────────────┘
```

## 服务器配置

### 1. 服务器地址配置

服务器示例配置了两个监听地址：

```rust
let server = FlareIMServerBuilder::new()
    .websocket_addr("127.0.0.1:4000".parse::<SocketAddr>().unwrap())
    .quic_addr("127.0.0.1:4010".parse::<SocketAddr>().unwrap())
    .enable_websocket(true)
    .enable_quic(true)
    .build();
```

### 2. TLS 配置

QUIC 协议需要 TLS 证书：

```rust
.quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
.quic_alpn(vec![b"flare-core".to_vec()])
.enable_quic_0rtt(true)
```

### 3. 消息处理器

服务器使用自定义消息处理器：

```rust
#[async_trait::async_trait]
impl flare_core::server::MessageHandler for CustomMessageHandler {
    async fn handle_message(&self, user_id: &str, message: ProtoMessage) -> Result<ProtoMessage> {
        match message.message_type.as_str() {
            "chat" => {
                // 处理聊天消息
                let response = ProtoMessage::new(
                    uuid::Uuid::new_v4().to_string(),
                    "chat_response".to_string(),
                    format!("收到聊天消息: {}", String::from_utf8_lossy(&message.payload)).into_bytes(),
                );
                Ok(response)
            }
            _ => {
                // 默认 echo 响应
                let response = ProtoMessage::new(
                    uuid::Uuid::new_v4().to_string(),
                    "echo".to_string(),
                    message.payload,
                );
                Ok(response)
            }
        }
    }
}
```

## 客户端配置

### 1. WebSocket 客户端

```rust
// 服务器地址配置
let server_addresses = ServerAddresses::new()
    .with_websocket_url("ws://127.0.0.1:4000".to_string());

// 客户端配置
let mut client = FlareIMClientBuilder::new("websocket_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::WebSocket))
    .connection_timeout(5000)
    .heartbeat_interval(30000)
    .max_reconnect_attempts(3)
    .reconnect_delay(1000)
    .auto_reconnect(true)
    .message_retry(3, 1000)
    .buffer_size(8192)
    .compression(true)
    .encryption(true)
    .tls(false)  // WebSocket 不使用 TLS
    .build()?
    .with_event_callback(event_callback);
```

### 2. QUIC 客户端

```rust
// 服务器地址配置
let server_addresses = ServerAddresses::new()
    .with_quic_url("quic://127.0.0.1:4010".to_string());

// 客户端配置
let mut client = FlareIMClientBuilder::new("quic_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::QUIC))
    .connection_timeout(5000)
    .heartbeat_interval(30000)
    .max_reconnect_attempts(3)
    .reconnect_delay(1000)
    .auto_reconnect(true)
    .message_retry(3, 1000)
    .buffer_size(8192)
    .compression(true)
    .encryption(true)
    .tls(true)  // QUIC 使用 TLS
    .build()?
    .with_event_callback(event_callback);
```

### 3. 协议竞速客户端

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

// 服务器地址配置
let server_addresses = ServerAddresses::new()
    .with_quic_url("quic://127.0.0.1:4010".to_string())
    .with_websocket_url("ws://127.0.0.1:4000".to_string());

// 客户端配置
let mut client = FlareIMClientBuilder::new("racing_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::AutoRacing)
    .protocol_racing(racing_config)
    .connection_timeout(5000)
    .heartbeat_interval(30000)
    .max_reconnect_attempts(3)
    .reconnect_delay(1000)
    .auto_reconnect(true)
    .message_retry(3, 1000)
    .buffer_size(8192)
    .compression(true)
    .encryption(true)
    .tls(true)  // 协议竞速使用 TLS
    .build()?
    .with_event_callback(event_callback);
```

## 事件处理

### 1. 事件回调设置

```rust
use flare_core::client::callbacks::ClientEventCallback;
let event_callback: Arc<ClientEventCallback> = Arc::new(Box::new(|event| {
    match event {
        ClientEvent::Connected(protocol) => {
            info!("连接成功，使用协议: {:?}", protocol);
        }
        ClientEvent::Disconnected => {
            info!("连接断开");
        }
        ClientEvent::Reconnecting => {
            info!("正在重连");
        }
        ClientEvent::Reconnected(protocol) => {
            info!("重连成功，使用协议: {:?}", protocol);
        }
        ClientEvent::Error(error_msg) => {
            error!("连接错误: {}", error_msg);
        }
        ClientEvent::MessageReceived(message) => {
            info!("收到消息: ID={}, 类型={}, 内容={}", 
                  message.id, message.message_type, 
                  String::from_utf8_lossy(&message.payload));
        }
        ClientEvent::MessageSent(message_id) => {
            info!("消息发送成功: {}", message_id);
        }
        ClientEvent::MessageFailed(message_id, error) => {
            warn!("消息发送失败: {} - {}", message_id, error);
        }
        ClientEvent::Heartbeat => {
            debug!("心跳");
        }
        ClientEvent::ProtocolSwitched(protocol) => {
            info!("协议切换到: {:?}", protocol);
        }
        ClientEvent::ReconnectFailed => {
            error!("重连失败");
        }
    }
}));
```

### 2. 事件类型说明

| 事件类型 | 说明 | 触发时机 |
|---------|------|----------|
| `Connected` | 连接成功 | 客户端成功连接到服务器 |
| `Disconnected` | 连接断开 | 客户端与服务器断开连接 |
| `Reconnecting` | 正在重连 | 客户端开始重连 |
| `Reconnected` | 重连成功 | 客户端重连成功 |
| `Error` | 连接错误 | 发生连接错误 |
| `MessageReceived` | 收到消息 | 从服务器收到消息 |
| `MessageSent` | 消息发送成功 | 消息成功发送到服务器 |
| `MessageFailed` | 消息发送失败 | 消息发送失败 |
| `Heartbeat` | 心跳 | 心跳消息 |
| `ProtocolSwitched` | 协议切换 | 协议竞速时切换协议 |
| `ReconnectFailed` | 重连失败 | 重连失败 |

## 消息发送

### 1. 文本消息

```rust
let text_result = client.send_text_message("server", "Hello from client!").await?;
if text_result.success {
    info!("文本消息发送成功: {}", text_result.message_id);
} else {
    warn!("文本消息发送失败: {:?}", text_result.error_message);
}
```

### 2. 二进制消息

```rust
let binary_data = b"Binary message from client".to_vec();
let binary_result = client.send_binary_message("server", binary_data, "binary".to_string()).await?;
if binary_result.success {
    info!("二进制消息发送成功: {}", binary_result.message_id);
} else {
    warn!("二进制消息发送失败: {:?}", binary_result.error_message);
}
```

### 3. 自定义消息

```rust
let custom_message = ProtoMessage::new(
    uuid::Uuid::new_v4().to_string(),
    "custom".to_string(),
    serde_json::json!({
        "type": "custom",
        "client": "client_name",
        "data": "Custom message from client",
        "timestamp": chrono::Utc::now().timestamp()
    }).to_string().into_bytes(),
);
let custom_result = client.send_message("server", custom_message).await?;
if custom_result.success {
    info!("自定义消息发送成功: {}", custom_result.message_id);
} else {
    warn!("自定义消息发送失败: {:?}", custom_result.error_message);
}
```

## 状态监控

### 1. 连接状态

```rust
let status = client.get_status().await;
info!("当前连接状态: {:?}", status);
```

### 2. 连接统计

```rust
let stats = client.get_connection_stats().await?;
info!("连接统计: 发送消息 {}, 接收消息 {}, 心跳次数 {}, 错误次数 {}", 
      stats.messages_sent, stats.messages_received, stats.heartbeat_count, stats.error_count);
```

### 3. 协议性能指标（协议竞速客户端）

```rust
let metrics = client.get_all_protocol_metrics().await;
info!("协议性能指标: {:?}", metrics);

if let Some(best_protocol) = client.get_best_protocol().await {
    info!("最佳协议: {:?}", best_protocol);
}
```

### 4. 协议可用性

```rust
let quic_available = client.is_protocol_available(TransportProtocol::QUIC).await;
let ws_available = client.is_protocol_available(TransportProtocol::WebSocket).await;
info!("协议可用性 - QUIC: {}, WebSocket: {}", quic_available, ws_available);
```

## 运行步骤

### 1. 环境准备

```bash
# 检查 Rust 版本
rustc --version

# 检查 OpenSSL
openssl version

# 生成证书
./scripts/generate_certs.sh
```

### 2. 编译项目

```bash
# 编译所有示例
cargo build --examples
```

### 3. 启动服务器

```bash
# 启动服务器
cargo run --example server_example
```

### 4. 运行客户端

```bash
# 运行 WebSocket 客户端
cargo run --example websocket_client

# 运行 QUIC 客户端
cargo run --example quic_client

# 运行协议竞速客户端
cargo run --example auto_racing_client
```

### 5. 使用演示脚本

```bash
# 完整演示
./examples/demo.sh

# 连接测试
./examples/test_connection.sh
```

## 故障排除

### 1. 连接失败

**问题**: 客户端无法连接到服务器

**解决方案**:
- 检查服务器是否正在运行
- 确认服务器地址和端口正确
- 检查防火墙设置
- 查看服务器日志

### 2. QUIC 连接失败

**问题**: QUIC 客户端连接失败

**解决方案**:
- 确认证书文件存在且有效
- 检查证书路径配置
- 确认 OpenSSL 版本兼容
- 查看详细错误日志

### 3. WebSocket 连接失败

**问题**: WebSocket 客户端连接失败

**解决方案**:
- 确认服务器 WebSocket 端口开放
- 检查 URL 格式是否正确
- 确认网络连接正常
- 查看浏览器开发者工具

### 4. 协议竞速失败

**问题**: 协议竞速客户端无法选择协议

**解决方案**:
- 确认至少配置了一个协议地址
- 检查协议竞速配置参数
- 确认网络环境支持所有协议
- 查看协议性能指标

### 5. 消息发送失败

**问题**: 消息无法发送到服务器

**解决方案**:
- 检查连接状态
- 确认目标用户ID正确
- 查看消息队列长度
- 检查消息格式

## 性能优化

### 1. 连接优化

- 使用连接池复用连接
- 启用连接预热
- 优化重连策略
- 调整连接超时时间

### 2. 消息优化

- 启用消息压缩
- 使用批量发送
- 优化消息序列化
- 实现消息缓存

### 3. 协议优化

- 根据网络环境选择最佳协议
- 动态调整协议权重
- 优化协议竞速参数
- 实现智能降级

## 总结

通过本指南，您可以：

1. **正确配置服务器和客户端**
2. **实现完整的事件处理**
3. **发送和接收各种类型的消息**
4. **监控连接状态和性能**
5. **处理各种错误情况**
6. **优化性能和可靠性**

Flare IM 提供了灵活的配置选项和强大的功能，可以满足各种即时通讯应用的需求。 