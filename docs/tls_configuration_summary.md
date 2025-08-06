# TLS 配置总结

## 概述

本文档总结了 Flare IM 项目中 WebSocket 和 QUIC 协议的 TLS 支持情况。

## 配置原则

- **WebSocket**: 仅支持非 TLS 连接（ws://）
- **QUIC**: 正常支持 TLS 连接（需要证书）

## WebSocket TLS 配置

### 服务端配置

```rust
// src/server/config.rs
impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from_str("0.0.0.0:8080").unwrap(),
            enabled: true,
            max_connections: 10000,
            connection_timeout_ms: 300000,
            heartbeat_interval_ms: 30000,
            enable_tls: false,  // 默认不启用 TLS
            cert_path: None,    // 无证书路径
            key_path: None,     // 无私钥路径
        }
    }
}
```

### 客户端配置

```rust
// src/client/config.rs
impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            // ... 其他配置
            tls_config: None,  // 默认无 TLS 配置
        }
    }
}
```

### WebSocket 连接器

```rust
// src/client/websocket_connector.rs
// 构建 WebSocket URL（仅支持非TLS）
let ws_url = format!("ws://{}:{}", host, port);
```

### 示例配置

```rust
// examples/websocket_client.rs
let mut client = FlareIMClientBuilder::new("websocket_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::WebSocket))
    .tls(false)  // WebSocket 不使用 TLS
    .build()?;
```

## QUIC TLS 配置

### 服务端配置

```rust
// src/server/config.rs
impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from_str("0.0.0.0:8081").unwrap(),
            enabled: true,
            max_connections: 10000,
            connection_timeout_ms: 300000,
            heartbeat_interval_ms: 30000,
            cert_path: "certs/server.crt".to_string(),  // 默认证书路径
            key_path: "certs/server.key".to_string(),   // 默认私钥路径
            alpn_protocols: vec![b"flare-core".to_vec()],
            enable_0rtt: true,
        }
    }
}
```

### 客户端配置

```rust
// examples/quic_client.rs
let mut client = FlareIMClientBuilder::new("quic_user".to_string())
    .server_addresses(server_addresses)
    .protocol_selection_mode(ProtocolSelectionMode::Specific(TransportProtocol::QUIC))
    .tls(true)  // QUIC 使用 TLS
    .build()?;
```

### QUIC 连接器

```rust
// src/client/quic_connector.rs
// 创建客户端 TLS 配置
async fn create_client_tls_config(&self) -> Result<QuinnClientConfig> {
    let mut root_store = RootCertStore::empty();

    // 如果有 TLS 配置，加载证书
    if let Some(tls_config) = &self.config.tls_config {
        if let Some(cert_path) = &tls_config.server_cert_path {
            let cert_data = fs::read(cert_path)?;
            let cert = rustls::Certificate(cert_data);
            root_store.add(&cert)?;
        }
    } else {
        // 使用默认的根证书存储
        root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
    }
    
    // 创建 Rustls 客户端配置
    let mut rustls_config = RustlsClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    
    // 配置 QUIC
    rustls_config.alpn_protocols = vec![b"flare-core".to_vec()];
    
    Ok(QuinnClientConfig::new(Arc::new(rustls_config)))
}
```

## 服务器示例配置

```rust
// examples/server_example.rs
let server = FlareIMServerBuilder::new()
    .websocket_addr("127.0.0.1:4000".parse::<SocketAddr>().unwrap())
    .quic_addr("127.0.0.1:4010".parse::<SocketAddr>().unwrap())
    .enable_websocket(true)
    .enable_quic(true)
    // 注意：没有配置 WebSocket TLS
    .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())  // 只配置 QUIC TLS
    .quic_alpn(vec![b"flare-core".to_vec()])
    .enable_quic_0rtt(true)
    .build();
```

## 证书文件

### 生成证书

项目提供了证书生成脚本：

```bash
# scripts/generate_certs.sh
#!/bin/bash
mkdir -p certs
openssl req -x509 -newkey rsa:4096 -keyout certs/server.key -out certs/server.crt -days 365 -nodes -subj "/C=CN/ST=State/L=City/O=Organization/CN=localhost"
```

### 证书文件结构

```
flare-core/
├── certs/
│   ├── server.crt    # 服务器证书（QUIC 使用）
│   └── server.key    # 服务器私钥（QUIC 使用）
└── ...
```

## 连接 URL 格式

### WebSocket
- **客户端连接**: `ws://127.0.0.1:4000`
- **服务端监听**: `127.0.0.1:4000`

### QUIC
- **客户端连接**: `quic://127.0.0.1:4010`
- **服务端监听**: `127.0.0.1:4010`

## 安全考虑

### WebSocket（非 TLS）
- 适用于内网环境
- 适用于开发测试
- 不适用于生产环境的敏感数据传输

### QUIC（TLS）
- 提供端到端加密
- 支持证书验证
- 适用于生产环境

## 总结

1. **WebSocket**: 简化设计，仅支持非 TLS 连接，减少复杂性和资源消耗
2. **QUIC**: 完整支持 TLS，提供安全连接
3. **配置分离**: 两种协议的 TLS 配置完全独立
4. **默认安全**: QUIC 默认启用 TLS，WebSocket 默认不启用

这种设计平衡了简单性和安全性，适合不同的使用场景。 