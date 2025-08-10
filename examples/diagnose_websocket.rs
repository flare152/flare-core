//! WebSocket 连接诊断工具
//!
//! 用于诊断 WebSocket 连接和消息发送问题

use flare_core::client::{
    Client,
    config::{ProtocolSelectionMode, ServerAddresses, ClientConfigBuilder},
    protocol_racer::ClientEvent,
    types::MessagePriority,
};
use flare_core::common::UnifiedProtocolMessage;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("flare_core=debug")
        .init();

    println!("🔍 WebSocket 连接诊断工具");

    // 创建事件回调
    let event_callback = Arc::new(Box::new(|event: ClientEvent| {
        match event {
            ClientEvent::Connect(_) => {
                info!("✅ 连接成功");
            }
            ClientEvent::Disconnect(_) => {
                info!("❌ 连接断开");
            }
            ClientEvent::Heartbeat(_) => {
                debug!("💓 心跳");
            }
            ClientEvent::Custom(_) => {
                info!("📨 自定义事件");
            }
        }
    }));

    // 创建服务器地址配置（只配置 WebSocket）
    let server_addresses = ServerAddresses::new()
        .with_websocket_url("ws://127.0.0.1:4000".to_string());

    println!("🔗 尝试连接到: ws://127.0.0.1:4000");

    // 创建客户端，指定使用 WebSocket 协议
    let config = ClientConfigBuilder::new("diagnostic_user".to_string())
        .server_addresses(server_addresses)
        .protocol_selection_mode(ProtocolSelectionMode::Specific(flare_core::common::TransportProtocol::WebSocket))
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

    let mut client = Client::new(config);

    println!("🚀 开始连接...");

    // 连接到服务器
    match client.connect().await {
        Ok(protocol) => {
            println!("✅ 连接成功，使用协议: {:?}", protocol);

            // 等待连接稳定
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // 发送测试消息
            println!("📤 发送测试消息...");
            let test_message = UnifiedProtocolMessage::text("Hello from diagnostic tool!".to_string());
            let test_result = client.send_message(
                test_message,
                MessagePriority::Normal,
                "server".to_string(),
            ).await?;
            println!("✅ 测试消息发送成功: {:?}", test_result);

            // 获取连接状态
            let is_connected = client.is_connected().await;
            println!("📊 当前连接状态: {}", if is_connected { "已连接" } else { "已断开" });

            // 连接统计功能已移除，现在在连接管理器中
            println!("📈 连接统计功能已移除");

            // 等待一段时间接收消息
            println!("⏳ 等待接收消息...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            // 断开连接
            println!("🔌 断开连接...");
            client.disconnect().await?;
            println!("✅ 连接已断开");
        }
        Err(e) => {
            println!("❌ 连接失败: {}", e);
            error!("连接失败详情: {}", e);
        }
    }

    println!("🏁 诊断完成");
    Ok(())
} 