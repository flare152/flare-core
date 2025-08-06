# Flare IM TLS 配置指南

## 概述

Flare IM 支持 TLS 加密通信，特别是 QUIC 协议必须使用 TLS。本文档说明如何正确配置 TLS。

## 快速开始

### 1. 生成证书

```bash
# 生成测试证书
./scripts/generate_certs.sh
```

这将生成以下证书文件：
- `certs/server.crt` - 服务器证书
- `certs/server.key` - 服务器私钥
- `certs/ca.crt` - CA 证书
- `certs/client.crt` - 客户端证书
- `certs/client.key` - 客户端私钥

### 2. 服务器配置

```rust
use flare_im::server::{FlareIMServerBuilder, DefaultAuthHandler, DefaultMessageHandler, DefaultEventHandler};

let server = FlareIMServerBuilder::new()
    .websocket_addr("127.0.0.1:8080".parse()?)
    .quic_addr("127.0.0.1:8081".parse()?)
    .enable_websocket(true)
    .enable_quic(true)
    .max_connections(1000)
    .enable_auth(true)
    .log_level("info".to_string())
    // QUIC TLS 配置
    .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
    .quic_alpn(vec![b"flare-core".to_vec()])
    .enable_quic_0rtt(true)
    .with_auth_handler(Arc::new(DefaultAuthHandler::new("secret".to_string())))
    .with_message_handler(Arc::new(DefaultMessageHandler))
    .with_event_handler(Arc::new(DefaultEventHandler))
    .build();
```

### 3. 客户端配置

```rust
use flare_im::client::{FlareIMClientBuilder, TransportProtocol};

let client = FlareIMClientBuilder::new(
    "user_001".to_string(),
    "flare-core://localhost".to_string(),
)
.preferred_protocol(TransportProtocol::QUIC)
.connection_timeout(5000)
.heartbeat_interval(15000)
.max_reconnect_attempts(3)
.auto_reconnect(true)
.auto_switch(true)
.message_retry(2, 500)
.buffer_size(4096)
.tls(true) // 启用 TLS 支持
.build()?;
```

## 端口配置

- **WebSocket**: `127.0.0.1:8080` (非 TLS)
- **QUIC**: `127.0.0.1:8081` (TLS 加密)

## TLS 配置详解

### 服务器端 TLS 配置

QUIC 服务器必须配置 TLS 证书：

```rust
// 必需：配置 QUIC TLS 证书
.quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())

// 可选：配置 ALPN 协议
.quic_alpn(vec![b"flare-core".to_vec()])

// 可选：启用 0-RTT
.enable_quic_0rtt(true)
```

### 客户端 TLS 配置

客户端 TLS 配置是可选的，但推荐启用：

```rust
// 启用 TLS（使用默认配置）
.tls(true)

// 或者使用自定义 TLS 配置
.tls_config(TlsConfig {
    verify_server_cert: true,
    client_cert_path: Some("certs/client.crt".to_string()),
    client_key_path: Some("certs/client.key".to_string()),
    ca_cert_path: Some("certs/ca.crt".to_string()),
    server_name: Some("localhost".to_string()),
})
```

## 证书管理

### 生成自签名证书

```bash
# 生成测试证书
./scripts/generate_certs.sh

# 查看证书信息
./scripts/generate_certs.sh -i

# 验证证书
./scripts/generate_certs.sh -v

# 清理临时文件
./scripts/generate_certs.sh -c
```

### 使用生产证书

对于生产环境，建议使用受信任的 CA 签发的证书：

1. 从证书颁发机构获取证书
2. 将证书文件放置在 `certs/` 目录
3. 更新服务器配置中的证书路径

## 故障排除

### 常见问题

1. **证书文件不存在**
   ```
   错误：无法读取证书文件
   解决：运行 ./scripts/generate_certs.sh 生成证书
   ```

2. **证书权限问题**
   ```
   错误：无法读取私钥文件
   解决：确保私钥文件权限为 600
   ```

3. **证书验证失败**
   ```
   错误：证书验证失败
   解决：检查证书是否过期，或使用正确的 CA 证书
   ```

4. **QUIC 连接失败**
   ```
   错误：QUIC 连接失败
   解决：确保服务器已启动且 TLS 配置正确
   ```

### 调试步骤

1. **检查证书**
   ```bash
   openssl x509 -in certs/server.crt -text -noout
   ```

2. **验证服务器配置**
   ```bash
   cargo run --example server_example
   ```

3. **测试客户端连接**
   ```bash
   cargo run --example client_example
   ```

4. **查看详细日志**
   ```bash
   RUST_LOG=debug cargo run --example server_example
   ```

## 安全建议

1. **生产环境**
   - 使用受信任的 CA 签发的证书
   - 定期更新证书
   - 使用强加密算法

2. **开发环境**
   - 使用自签名证书进行测试
   - 不要在生产环境使用测试证书

3. **证书管理**
   - 安全存储私钥文件
   - 定期备份证书
   - 监控证书过期时间

## 示例配置

### 完整服务器示例

```rust
use std::sync::Arc;
use flare_im::server::{
    FlareIMServerBuilder, 
    DefaultAuthHandler, 
    DefaultMessageHandler, 
    DefaultEventHandler
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("flare_im=info")
        .init();

    // 构建服务器
    let server = FlareIMServerBuilder::new()
        .websocket_addr("127.0.0.1:8080".parse()?)
        .quic_addr("127.0.0.1:8081".parse()?)
        .enable_websocket(true)
        .enable_quic(true)
        .max_connections(1000)
        .enable_auth(true)
        .log_level("info".to_string())
        // QUIC TLS 配置
        .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
        .quic_alpn(vec![b"flare-core".to_vec()])
        .enable_quic_0rtt(true)
        .with_auth_handler(Arc::new(DefaultAuthHandler::new("secret".to_string())))
        .with_message_handler(Arc::new(DefaultMessageHandler))
        .with_event_handler(Arc::new(DefaultEventHandler))
        .build();

    // 启动服务器
    server.start().await?;

    // 保持运行
    tokio::signal::ctrl_c().await?;
    server.stop().await?;

    Ok(())
}
```

### 完整客户端示例

```rust
use flare_im::client::{FlareIMClientBuilder, TransportProtocol};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("flare_im=info")
        .init();

    // 创建客户端
    let mut client = FlareIMClientBuilder::new(
        "user_001".to_string(),
        "flare-core://localhost".to_string(),
    )
    .preferred_protocol(TransportProtocol::QUIC)
    .connection_timeout(5000)
    .heartbeat_interval(15000)
    .max_reconnect_attempts(3)
    .auto_reconnect(true)
    .auto_switch(true)
    .message_retry(2, 500)
    .buffer_size(4096)
    .tls(true) // 启用 TLS 支持
    .build()?;

    // 连接到服务器
    let protocol = client.connect("flare-core://localhost:8081").await?;
    println!("连接成功，使用协议: {:?}", protocol);

    // 发送消息
    client.send_text_message("server", "Hello, TLS!").await?;

    // 保持连接
    tokio::signal::ctrl_c().await?;
    client.disconnect().await?;

    Ok(())
}
```

## 总结

Flare IM 的 TLS 配置相对简单：

1. **QUIC 必须使用 TLS** - 这是 QUIC 协议的要求
2. **WebSocket 可选 TLS** - 可以根据需要启用
3. **证书管理** - 提供脚本自动生成测试证书
4. **配置简洁** - 通过构建器模式简化配置

通过正确配置 TLS，可以确保 Flare IM 的通信安全性。 