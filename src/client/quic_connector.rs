//! QUIC 连接器
//!
//! 负责创建和管理 QUIC 连接

use crate::common::{
    TransportProtocol, Result, ProtoMessage,
    conn::{Connection, Platform, ConnectionBuilder},
};
use crate::client::config::ClientConfig;
use std::time::Duration;
use tracing::{info, warn};
use quinn::{Endpoint, ClientConfig as QuinnClientConfig};
use std::net::SocketAddr;

/// QUIC 连接器
pub struct QuicConnector {
    config: ClientConfig,
}

impl QuicConnector {
    /// 创建新的 QUIC 连接器
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    /// 创建 QUIC 连接
    pub async fn create_connection(&self, server_url: &str, timeout: Duration) -> Result<Box<dyn Connection + Send + Sync>> {
        info!("创建 QUIC 连接到: {}", server_url);

        // 解析服务器URL
        let url = url::Url::parse(server_url).map_err(|e| format!("无效的URL: {}", e))?;
        let host = url.host_str().ok_or("缺少主机名")?;
        let port = url.port().unwrap_or(4010); // QUIC 默认端口

        info!("解析 QUIC 地址: {}:{}", host, port);

        // 创建连接配置
        let config = ConnectionBuilder::new()
            .id(uuid::Uuid::new_v4().to_string())
            .remote_addr(format!("{}:{}", host, port))
            .platform(Platform::Desktop)
            .protocol(TransportProtocol::QUIC)
            .timeout_ms(self.config.connection_timeout_ms)
            .heartbeat_interval_ms(self.config.heartbeat_interval_ms)
            .max_reconnect_attempts(self.config.max_reconnect_attempts)
            .reconnect_delay_ms(self.config.reconnect_delay_ms)
            .build();

        // 创建 QUIC 端点
        let endpoint = self.create_quic_endpoint().await?;

        // 建立 QUIC 连接
        let (connection, send_stream, recv_stream) = self.connect_quic(&endpoint, host, port, timeout).await?;

        // 创建 QUIC 连接实例
        let quic_connection = crate::common::conn::quic::QuicConnectionFactory::from_quinn_connection(
            config,
            connection,
            send_stream,
            recv_stream,
        ).await;

        info!("QUIC 连接创建成功");
        Ok(Box::new(quic_connection))
    }

    /// 创建 QUIC 端点
    async fn create_quic_endpoint(&self) -> Result<Endpoint> {
        // 创建客户端 TLS 配置
        let client_config = self.create_client_tls_config().await?;

        // 创建 QUIC 端点
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse::<SocketAddr>().unwrap())
            .map_err(|e| format!("创建 QUIC 端点失败: {}", e))?;

        // 设置默认客户端配置
        endpoint.set_default_client_config(client_config);

        Ok(endpoint)
    }

    /// 创建客户端 TLS 配置
    async fn create_client_tls_config(&self) -> Result<QuinnClientConfig> {
        // 使用统一的TLS处理，支持自定义证书路径
        let cert_path = self.config.client_cert.as_deref().unwrap_or("certs/server.crt");
        
        let client_config = crate::common::tls::create_client_config(cert_path)
            .map_err(|e| format!("创建客户端TLS配置失败: {}", e))?;
        
        // 注意：quinn 0.11的ClientConfig没有直接的transport设置方法
        // 传输配置通常在创建时设置，这里我们使用默认配置
        Ok(client_config)
    }

    /// 建立 QUIC 连接
    async fn connect_quic(
        &self,
        endpoint: &Endpoint,
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<(quinn::Connection, quinn::SendStream, quinn::RecvStream)> {
        let server_addr = format!("{}:{}", host, port)
            .parse::<SocketAddr>()
            .map_err(|e| format!("解析服务器地址失败: {}", e))?;

        info!("连接到 QUIC 服务器: {}", server_addr);

        // 建立连接
        info!("开始建立 QUIC 连接...");
        let connecting = endpoint.connect(server_addr, host)
            .map_err(|e| format!("QUIC 连接失败: {}", e))?;
        
        info!("等待 QUIC 连接完成...");
        let connection = tokio::time::timeout(timeout, connecting)
            .await
            .map_err(|_| "QUIC 连接超时")?
            .map_err(|e| format!("QUIC 连接失败: {}", e))?;

        info!("QUIC 连接建立成功");

        // 打开双向流，添加超时处理
        info!("打开 QUIC 双向流...");
        let (send_stream, recv_stream) = tokio::time::timeout(
            Duration::from_secs(10),
            connection.open_bi()
        ).await
            .map_err(|_| "打开 QUIC 双向流超时")?
            .map_err(|e| format!("打开 QUIC 流失败: {}", e))?;

        info!("QUIC 双向流打开成功");
        Ok((connection, send_stream, recv_stream))
    }

    /// 验证连接
    pub async fn validate_connection(&self, connection: &Box<dyn Connection + Send + Sync>) -> Result<bool> {
        // 发送 ping 消息测试连接
        let ping_message = ProtoMessage::new(
            uuid::Uuid::new_v4().to_string(),
            "ping".to_string(),
            b"ping".to_vec(),
        );

        match connection.send(ping_message).await {
            Ok(_) => {
                info!("QUIC 连接验证成功");
                Ok(true)
            }
            Err(e) => {
                warn!("QUIC 连接验证失败: {}", e);
                Ok(false)
            }
        }
    }
} 