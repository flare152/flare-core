//! WebSocket 连接诊断工具
//!
//! 用于诊断 WebSocket 连接和消息发送问题

use flare_core::client::{
    FlareIMClientBuilder, 
    config::{ProtocolSelectionMode, ServerAddresses},
    types::{ClientEvent, ClientEventCallback},
};
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
    let event_callback: Arc<ClientEventCallback> = Arc::new(Box::new(|event| {
        match event {
            ClientEvent::Connected(protocol) => {
                info!("✅ 连接成功，使用协议: {:?}", protocol);
            }
            ClientEvent::Disconnected => {
                info!("❌ 连接断开");
            }
            ClientEvent::Reconnecting => {
                info!("🔄 正在重连");
            }
            ClientEvent::Reconnected(protocol) => {
                info!("✅ 重连成功，使用协议: {:?}", protocol);
            }
            ClientEvent::Error(error_msg) => {
                error!("💥 连接错误: {}", error_msg);
            }
            ClientEvent::MessageReceived(message) => {
                info!("📨 收到消息: ID={}, 类型={}, 内容={}", 
                      message.id, message.message_type, 
                      String::from_utf8_lossy(&message.payload));
            }
            ClientEvent::MessageSent(message_id) => {
                info!("📤 消息发送成功: {}", message_id);
            }
            ClientEvent::MessageFailed(message_id, error) => {
                warn!("❌ 消息发送失败: {} - {}", message_id, error);
            }
            ClientEvent::Heartbeat => {
                debug!("💓 心跳");
            }
            ClientEvent::ProtocolSwitched(protocol) => {
                info!("🔄 协议切换到: {:?}", protocol);
            }
            ClientEvent::ReconnectFailed => {
                error!("💥 重连失败");
            }
        }
    }));

    // 创建服务器地址配置（只配置 WebSocket）
    let server_addresses = ServerAddresses::new()
        .with_websocket_url("ws://127.0.0.1:4000".to_string());

    println!("🔗 尝试连接到: ws://127.0.0.1:4000");

    // 创建客户端，指定使用 WebSocket 协议
    let mut client = FlareIMClientBuilder::new("diagnostic_user".to_string())
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
        .build()?
        .with_event_callback(event_callback);

    println!("🚀 开始连接...");

    // 连接到服务器
    match client.connect().await {
        Ok(protocol) => {
            println!("✅ 连接成功，使用协议: {:?}", protocol);

            // 等待连接稳定
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // 发送测试消息
            println!("📤 发送测试消息...");
            let test_result = client.send_text_message("server", "Hello from diagnostic tool!").await?;
            if test_result.success {
                println!("✅ 测试消息发送成功: {}", test_result.message_id);
            } else {
                println!("❌ 测试消息发送失败: {:?}", test_result.error_message);
            }

            // 获取连接状态
            let status = client.get_status().await;
            println!("📊 当前连接状态: {:?}", status);

            // 获取连接统计
            let stats = client.get_connection_stats().await?;
            println!("📈 连接统计: 发送消息 {}, 接收消息 {}, 心跳次数 {}, 错误次数 {}", 
                  stats.messages_sent, stats.messages_received, stats.heartbeat_count, stats.error_count);

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