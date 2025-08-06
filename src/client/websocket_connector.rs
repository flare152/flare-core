//! WebSocket 连接器
//!
//! 负责创建和管理 WebSocket 连接

use crate::common::{
    TransportProtocol, Result, ProtoMessage,
    conn::{Connection, Platform, ConnectionBuilder},
};
use crate::client::config::ClientConfig;
use std::time::Duration;
use tracing::{info, warn};
use tokio_tungstenite::connect_async;
use futures_util::SinkExt;
use url::Url;

/// WebSocket 连接器
pub struct WebSocketConnector {
    config: ClientConfig,
}

impl WebSocketConnector {
    /// 创建新的 WebSocket 连接器
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    /// 创建 WebSocket 连接
    pub async fn create_connection(&self, server_url: &str, timeout: Duration) -> Result<Box<dyn Connection + Send + Sync>> {
        info!("创建 WebSocket 连接到: {}", server_url);

        // 解析服务器URL
        let url = Url::parse(server_url).map_err(|e| format!("无效的URL: {}", e))?;
        let host = url.host_str().ok_or("缺少主机名")?;
        let port = url.port().unwrap_or(4000); // WebSocket 默认端口

        info!("解析 WebSocket 地址: {}:{}", host, port);

        // 创建连接配置
        let config = ConnectionBuilder::new()
            .id(uuid::Uuid::new_v4().to_string())
            .remote_addr(format!("{}:{}", host, port))
            .platform(Platform::Desktop)
            .protocol(TransportProtocol::WebSocket)
            .timeout_ms(self.config.connection_timeout_ms)
            .heartbeat_interval_ms(self.config.heartbeat_interval_ms)
            .max_reconnect_attempts(self.config.max_reconnect_attempts)
            .reconnect_delay_ms(self.config.reconnect_delay_ms)
            .build();

        // 根据原始URL构建 WebSocket URL（支持TLS和非TLS）
        let ws_url = if url.scheme() == "wss" || url.scheme() == "https" {
            format!("wss://{}:{}", host, port)
        } else {
            format!("ws://{}:{}", host, port)
        };

        info!("连接到 WebSocket URL: {}", ws_url);

        // 创建 WebSocket 连接
        let ws_stream = self.connect_websocket(&ws_url, timeout).await?;

        // 创建 WebSocket 连接实例
        let ws_connection = crate::common::conn::websocket::WebSocketConnectionFactory::from_tungstenite_stream(
            config,
            ws_stream,
        );

        info!("WebSocket 连接创建成功");
        Ok(Box::new(ws_connection))
    }

    /// 建立 WebSocket 连接
    async fn connect_websocket(&self, url: &str, timeout: Duration) -> Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> {
        let url = Url::parse(url).map_err(|e| format!("无效的 WebSocket URL: {}", e))?;

        info!("尝试连接到 WebSocket URL: {}", url);

        // 设置连接超时
        let connect_future = connect_async(url);
        let ws_stream = tokio::time::timeout(timeout, connect_future)
            .await
            .map_err(|_| "WebSocket 连接超时")?
            .map_err(|e| format!("WebSocket 连接失败: {}", e))?;

        info!("WebSocket 连接建立成功");
        Ok(ws_stream.0)
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
                info!("WebSocket 连接验证成功");
                Ok(true)
            }
            Err(e) => {
                warn!("WebSocket 连接验证失败: {}", e);
                Ok(false)
            }
        }
    }
} 