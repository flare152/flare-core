# Flare IM 简化 TLS 配置

## 概述

采用最简洁的证书方案：服务端证书 + 客户端验证证书，客户端使用服务端证书的副本进行验证。

## 证书结构

```
certs/
├── server.crt # 服务端证书（自签名）
├── server.key # 服务端私钥
└── client.crt # 客户端验证证书（服务端证书的副本）
```

## 证书关系

- **服务端证书**：自签名，用于服务端身份验证
- **客户端证书**：服务端证书的副本，用于验证服务端身份

## 生成证书

```bash
./scripts/generate_certs.sh
```

## 配置示例

### 服务端配置
```rust
let server = FlareIMServerBuilder::new()
    .websocket_addr("127.0.0.1:8080".parse()?)
    .quic_addr("127.0.0.1:8081".parse()?)
    .enable_websocket(true)
    .enable_quic(true)
    .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
    .build();
```

### 客户端配置
```rust
let client = FlareIMClientBuilder::new(
    "user_001".to_string(),
    "flare-core://localhost".to_string(),
)
.preferred_protocol(TransportProtocol::QUIC)
.tls(true) // 启用 TLS，使用默认配置
.build()?;
```

## 优势

1. **极简管理**：只需要服务端 key 和 crt 文件
2. **客户端简洁**：客户端只需要 crt 文件
3. **减少复杂度**：无需 CA 证书链，无需客户端私钥
4. **实用场景**：符合客户端验证服务端的常见模式

## 验证状态

✅ **证书生成**：自动生成服务端证书和客户端验证证书  
✅ **格式验证**：证书格式正确  
✅ **权限设置**：私钥权限安全  
✅ **编译通过**：代码配置正确  

简化证书方案已成功实现，QUIC 协议使用 TLS 加密通信。 