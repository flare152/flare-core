//! WebSocket 客户端示例
//!
//! 展示如何使用 FlareIMClient 指定 WebSocket 协议进行连接和消息发送

use flare_core::client::{
    config::{ProtocolSelectionMode, ServerAddresses, ClientConfigBuilder},
    protocol_racer::ClientEvent,
    callbacks::ClientCallbackManager,
};
use flare_core::common::{TransportProtocol, UnifiedProtocolMessage};
use flare_core::client::Client;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("flare_core=info,flare_core::client=debug")
        .init();

    info!("🚀 启动 WebSocket 客户端示例");

    // 创建事件回调
    let event_callback = Arc::new(Box::new(|event: ClientEvent| {
        match event {
            ClientEvent::Connect(_) => {
                info!("[WebSocket] 连接成功");
            }
            ClientEvent::Disconnect(_) => {
                info!("[WebSocket] 连接断开");
            }
            ClientEvent::Heartbeat(_) => {
                debug!("[WebSocket] 心跳");
            }
            ClientEvent::Custom(_) => {
                info!("[WebSocket] 自定义事件");
            }
        }
    }));

    // 创建服务器地址配置（只配置 WebSocket）
    let server_addresses = ServerAddresses::new()
        .with_websocket_url("ws://127.0.0.1:4000".to_string());

    // 创建客户端配置
    let config = ClientConfigBuilder::new("websocket_user".to_string())
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
        .build()?;

    // 创建客户端
    let mut client = Client::new(config);

    info!("WebSocket 客户端创建成功，开始连接...");

    // 连接到服务器
    match client.connect().await {
        Ok(()) => {
            info!("WebSocket 客户端连接成功");

            // 等待连接稳定
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // 发送文本消息
            info!("发送文本消息...");
            let text_message = UnifiedProtocolMessage::text(
                "Hello from WebSocket client!".to_string(),
            );
            let text_result = client.send_message(
                text_message,
                flare_core::client::types::MessagePriority::Normal,
                "server".to_string(),
            ).await?;
            info!("文本消息发送成功: {:?}", text_result);

            // 发送二进制消息
            info!("发送二进制消息...");
            let binary_data = b"Binary message from WebSocket client".to_vec();
            let binary_message = UnifiedProtocolMessage::binary(
                binary_data,
            );
            let binary_result = client.send_message(
                binary_message,
                flare_core::client::types::MessagePriority::Normal,
                "server".to_string(),
            ).await?;
            info!("二进制消息发送成功: {:?}", binary_result);

            // 发送自定义消息
            info!("发送自定义消息...");
            let custom_message = UnifiedProtocolMessage::custom_message(
                "custom".to_string(),
                serde_json::json!({
                    "type": "custom",
                    "client": "websocket",
                    "data": "Custom message from WebSocket client",
                    "timestamp": chrono::Utc::now().timestamp()
                }).to_string().into_bytes(),
            );
            let custom_result = client.send_message(
                custom_message,
                flare_core::client::types::MessagePriority::Normal,
                "server".to_string(),
            ).await?;
            info!("自定义消息发送成功: {:?}", custom_result);

            // 检查连接状态
            let is_connected = client.is_connected().await;
            info!("当前连接状态: {}", is_connected);

            // 持续运行一段时间，发送心跳消息
            info!("WebSocket 客户端运行中，按 Ctrl+C 停止...");
            
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            let mut counter = 0;
            
            loop {
                interval.tick().await;
                counter += 1;
                
                // 每10秒发送一次心跳消息
                let heartbeat_msg = format!("WebSocket heartbeat message #{}", counter);
                let heartbeat_message = UnifiedProtocolMessage::text(heartbeat_msg);
                if let Err(e) = client.send_message(
                    heartbeat_message,
                    flare_core::client::types::MessagePriority::Low,
                    "server".to_string(),
                ).await {
                    warn!("心跳消息发送失败: {}", e);
                }
                
                // 检查连接状态
                let is_connected = client.is_connected().await;
                if !is_connected {
                    warn!("连接状态异常: 已断开");
                }
                
                // 运行60秒后停止
                if counter >= 6 {
                    break;
                }
            }

            // 断开连接
            info!("断开 WebSocket 连接...");
            client.disconnect().await?;
            info!("WebSocket 客户端已断开连接");
        }
        Err(e) => {
            error!("WebSocket 客户端连接失败: {}", e);
        }
    }

    info!("✅ WebSocket 客户端示例运行完成");
    Ok(())
} 