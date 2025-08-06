# Flare IM 客户端架构

## 概述

Flare IM 客户端采用模块化设计，支持协议竞速和自动降级，提供高性能、可靠的即时通讯功能。

## 架构设计

### 核心模块

```
FlareIMClient
├── config/          # 配置管理
├── types/           # 类型定义
├── protocol_racer/  # 协议竞速
├── connection_manager/  # 连接管理
├── message_manager/     # 消息管理
└── traits/         # 接口定义
```

### 模块职责

#### 1. 配置模块 (`config.rs`)
- **ClientConfig**: 客户端配置结构
- **ClientConfigBuilder**: 配置构建器
- **功能**: 
  - 连接参数配置
  - 协议偏好设置
  - 重连策略配置
  - 消息重试配置

#### 2. 类型定义 (`types.rs`)
- **ClientStatus**: 客户端状态枚举
- **SendResult**: 发送结果结构
- **ConnectionStats**: 连接统计信息
- **ProtocolMetrics**: 协议性能指标
- **ClientEvent**: 客户端事件枚举
- **MessagePriority**: 消息优先级

#### 3. 协议竞速 (`protocol_racer.rs`)
- **ProtocolRacer**: 协议竞速器
- **功能**:
  - 协议性能测试
  - 自动协议选择
  - 性能指标收集
  - 协议切换管理

#### 4. 连接管理 (`connection_manager.rs`)
- **ConnectionManager**: 连接管理器
- **功能**:
  - 连接建立和断开
  - 自动重连机制
  - 心跳管理
  - 连接状态监控
  - 事件触发

#### 5. 消息管理 (`message_manager.rs`)
- **MessageManager**: 消息管理器
- **功能**:
  - 消息发送和接收
  - 消息队列管理
  - 消息重试机制
  - 优先级处理
  - 事件触发

## 核心特性

### 1. 协议竞速

#### 竞速策略
```rust
// 优先使用 QUIC，失败时降级到 WebSocket
let protocol = protocol_racer.race_protocols(server_url).await?;
```

#### 性能指标
- **连接延迟**: 建立连接所需时间
- **消息延迟**: 消息往返时间
- **吞吐量**: 每秒处理消息数
- **成功率**: 消息发送成功率

#### 评分算法
```rust
fn calculate_protocol_score(&self, metrics: &ProtocolMetrics) -> f64 {
    let latency_score = 1000.0 / metrics.connection_latency_ms as f64;
    let message_latency_score = 100.0 / metrics.message_latency_ms as f64;
    let throughput_score = metrics.throughput_msgs_per_sec / 100.0;
    let success_score = metrics.success_rate;
    
    // 加权平均
    latency_score * 0.3 + message_latency_score * 0.3 + 
    throughput_score * 0.2 + success_score * 0.2
}
```

### 2. 连接管理

#### 状态机
```
Disconnected -> Connecting -> Connected
     ^              |            |
     |              v            v
     +-------- Reconnecting <- Failed
```

#### 重连策略
- **指数退避**: 重连延迟逐渐增加
- **最大重试**: 限制重连次数
- **状态检查**: 避免重复重连

#### 心跳机制
```rust
async fn start_heartbeat_task(&self) {
    let mut interval = interval(Duration::from_millis(config.heartbeat_interval_ms));
    loop {
        interval.tick().await;
        if let Err(e) = Self::send_heartbeat(&config).await {
            warn!("心跳发送失败: {}", e);
        }
    }
}
```

### 3. 消息管理

#### 消息队列
- **优先级队列**: 按优先级处理消息
- **重试机制**: 失败消息自动重试
- **队列清理**: 定期清理过期消息

#### 消息类型
```rust
// 文本消息
client.send_text_message("user_id", "Hello").await?;

// 二进制消息
client.send_binary_message("user_id", data, "binary".to_string()).await?;

// 自定义消息
let message = ProtoMessage::new(id, "custom", payload);
client.send_message("user_id", message).await?;
```

### 4. 事件驱动

#### 事件类型
```rust
pub enum ClientEvent {
    Connected(TransportProtocol),
    Disconnected,
    Reconnecting,
    Reconnected(TransportProtocol),
    MessageReceived(ProtoMessage),
    MessageSent(String),
    MessageFailed(String, String),
    Heartbeat,
    Error(String),
    ProtocolSwitched(TransportProtocol),
}
```

#### 事件处理
```rust
let event_callback = Arc::new(Box::new(move |event: ClientEvent| {
    match event {
        ClientEvent::Connected(protocol) => {
            info!("连接成功: {:?}", protocol);
        }
        ClientEvent::MessageReceived(message) => {
            info!("收到消息: {}", message.message_type);
        }
        // ... 其他事件处理
    }
}));
```

## 使用模式

### 1. 基础使用

```rust
// 创建客户端
let client = FlareIMClientBuilder::new(
    "user_id".to_string(),
    "flare-core://localhost:8080".to_string(),
)
.build()?;

// 连接服务器
client.connect("flare-core://localhost:8080").await?;

// 发送消息
client.send_text_message("target_user", "Hello").await?;

// 断开连接
client.disconnect().await?;
```

### 2. 高级配置

```rust
let client = FlareIMClientBuilder::new(
    "user_id".to_string(),
    "flare-core://localhost:8080".to_string(),
)
.preferred_protocol(TransportProtocol::QUIC)
.connection_timeout(5000)
.heartbeat_interval(30000)
.max_reconnect_attempts(5)
.auto_reconnect(true)
.auto_switch(true)
.message_retry(3, 1000)
.buffer_size(8192)
.build()?;
```

### 3. 事件处理

```rust
let event_handler = Arc::new(MyEventHandler::new());
let event_callback = Arc::new(Box::new(move |event: ClientEvent| {
    let handler = Arc::clone(&event_handler);
    tokio::spawn(async move {
        handler.handle_event(event).await;
    });
}));

let mut client = client.with_event_callback(event_callback);
```

## 性能优化

### 1. 连接优化
- **连接池**: 复用连接减少开销
- **连接复用**: 多个消息共享连接
- **连接预热**: 预先建立连接

### 2. 消息优化
- **批量发送**: 合并多个消息
- **压缩**: 启用消息压缩
- **缓存**: 缓存常用消息

### 3. 协议优化
- **协议选择**: 根据网络环境选择最佳协议
- **动态切换**: 根据性能指标动态切换协议
- **预测试**: 预先测试协议性能

## 错误处理

### 1. 连接错误
```rust
match client.connect(server_url).await {
    Ok(protocol) => {
        info!("连接成功: {:?}", protocol);
    }
    Err(e) => {
        error!("连接失败: {}", e);
        // 尝试重连或切换协议
    }
}
```

### 2. 消息错误
```rust
let result = client.send_message("user_id", message).await?;
if !result.success {
    warn!("消息发送失败: {:?}", result.error_message);
    // 处理失败逻辑
}
```

### 3. 重连错误
```rust
if let Err(e) = client.reconnect(server_url).await {
    error!("重连失败: {}", e);
    // 切换到备用服务器或协议
}
```

## 监控和调试

### 1. 状态监控
```rust
let status = client.get_status().await;
let stats = client.get_connection_stats().await?;
let protocol = client.get_current_protocol().await;
```

### 2. 性能监控
```rust
let metrics = client.get_protocol_metrics(TransportProtocol::QUIC).await;
if let Some(metrics) = metrics {
    info!("QUIC 延迟: {}ms, 吞吐量: {:.2} msg/s", 
          metrics.message_latency_ms, metrics.throughput_msgs_per_sec);
}
```

### 3. 日志记录
```rust
// 启用详细日志
tracing_subscriber::fmt()
    .with_env_filter("flare_core=debug")
    .init();
```

## 扩展开发

### 1. 自定义协议
```rust
impl TransportProtocol {
    pub fn custom_protocol() -> Self {
        // 实现自定义协议
    }
}
```

### 2. 自定义消息处理器
```rust
struct CustomMessageHandler;

impl MessageHandler for CustomMessageHandler {
    async fn handle_message(&self, message: ProtoMessage) -> Result<ProtoMessage> {
        // 自定义消息处理逻辑
        Ok(message)
    }
}
```

### 3. 自定义事件处理器
```rust
struct CustomEventHandler;

impl EventHandler for CustomEventHandler {
    async fn handle_event(&self, event: ClientEvent) -> Result<()> {
        // 自定义事件处理逻辑
        Ok(())
    }
}
```

## 最佳实践

### 1. 配置管理
- 使用环境变量管理敏感配置
- 根据部署环境调整参数
- 定期更新配置

### 2. 错误处理
- 实现优雅的错误处理
- 提供用户友好的错误信息
- 记录详细的错误日志

### 3. 性能优化
- 监控关键性能指标
- 根据使用场景调整参数
- 定期进行性能测试

### 4. 安全性
- 使用 TLS 加密传输
- 实现消息签名验证
- 保护用户隐私数据

## 总结

Flare IM 客户端采用模块化、事件驱动的架构设计，提供了：

1. **高性能**: 协议竞速和自动优化
2. **高可靠**: 自动重连和错误恢复
3. **易扩展**: 模块化设计和插件机制
4. **易使用**: 简洁的 API 和丰富的示例

这种设计使得客户端能够适应不同的网络环境和应用场景，为用户提供最佳的即时通讯体验。 