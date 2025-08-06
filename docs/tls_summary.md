# Flare IM TLS 配置总结

## 概述

Flare IM 已成功配置 TLS 支持，QUIC 协议使用 TLS 加密通信。

## 配置要点

### 1. 证书生成
```bash
./scripts/generate_certs.sh
```

### 2. 服务器配置
```rust
let server = FlareIMServerBuilder::new()
    .websocket_addr("127.0.0.1:8080".parse()?)
    .quic_addr("127.0.0.1:8081".parse()?)
    .enable_websocket(true)
    .enable_quic(true)
    .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
    .build();
```

### 3. 客户端配置
```rust
let client = FlareIMClientBuilder::new(
    "user_001".to_string(),
    "flare-core://localhost".to_string(),
)
.preferred_protocol(TransportProtocol::QUIC)
.tls(true)
.build()?;
```

## 端口配置

- **WebSocket**: `127.0.0.1:8080` (非 TLS)
- **QUIC**: `127.0.0.1:8081` (TLS 加密)

## 验证状态

✅ **证书生成**: 自动生成 CA、服务器和客户端证书  
✅ **服务器 TLS**: QUIC 服务器正确加载 TLS 证书  
✅ **客户端 TLS**: 客户端支持 TLS 连接  
✅ **连接测试**: 客户端成功连接到 QUIC 服务器  

## 使用方法

1. **生成证书**: `./scripts/generate_certs.sh`
2. **启动服务器**: `cargo run --example server_example`
3. **启动客户端**: `cargo run --example client_example`

## 技术实现

- **QUIC 服务器**: 使用 `rustls` 和 `quinn` 实现 TLS
- **证书格式**: 支持 PKCS8 和 RSA 私钥格式
- **ALPN 协议**: 配置为 `flare-core`
- **0-RTT 支持**: 启用快速重连

TLS 配置已完成，QUIC 协议现在使用加密通信。 