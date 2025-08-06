//! WebSocket 客户端示例
//!
//! 展示如何使用 FlareIMClient 指定 WebSocket 协议进行连接和消息发送

use flare_im::client::{
    FlareIMClientBuilder, 
    config::{ProtocolSelectionMode, ServerAddresses},
    types::{ClientEvent, ClientEventCallback},
};
use flare_im::common::{TransportProtocol, ProtoMessage};
use std::sync::Arc;
use tracing::{info, warn, error, debug};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("flare_im=info,flare_im::client=debug")
        .init();

    info!("🚀 启动 WebSocket 客户端示例");

    // 创建事件回调
    let event_callback: Arc<ClientEventCallback> = Arc::new(Box::new(|event| {
        match event {
            ClientEvent::Connected(protocol) => {
                info!("[WebSocket] 连接成功，使用协议: {:?}", protocol);
            }
            ClientEvent::Disconnected => {
                info!("[WebSocket] 连接断开");
            }
            ClientEvent::Reconnecting => {
                info!("[WebSocket] 正在重连");
            }
            ClientEvent::Reconnected(protocol) => {
                info!("[WebSocket] 重连成功，使用协议: {:?}", protocol);
            }
            ClientEvent::Error(error_msg) => {
                error!("[WebSocket] 连接错误: {}", error_msg);
            }
            ClientEvent::MessageReceived(message) => {
                info!("[WebSocket] 收到消息: ID={}, 类型={}, 内容={}", 
                      message.id, message.message_type, 
                      String::from_utf8_lossy(&message.payload));
            }
            ClientEvent::MessageSent(message_id) => {
                info!("[WebSocket] 消息发送成功: {}", message_id);
            }
            ClientEvent::MessageFailed(message_id, error) => {
                warn!("[WebSocket] 消息发送失败: {} - {}", message_id, error);
            }
            ClientEvent::Heartbeat => {
                debug!("[WebSocket] 心跳");
            }
            ClientEvent::ProtocolSwitched(protocol) => {
                info!("[WebSocket] 协议切换到: {:?}", protocol);
            }
            ClientEvent::ReconnectFailed => {
                error!("[WebSocket] 重连失败");
            }
        }
    }));

    // 创建服务器地址配置（只配置 WebSocket）
    let server_addresses = ServerAddresses::new()
        .with_websocket_url("ws://127.0.0.1:4000".to_string());

    // 创建客户端，指定使用 WebSocket 协议
    let mut client = FlareIMClientBuilder::new("websocket_user".to_string())
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
        .build()?
        .with_event_callback(event_callback);

    info!("WebSocket 客户端创建成功，开始连接...");

    // 连接到服务器
    match client.connect().await {
        Ok(protocol) => {
            info!("WebSocket 客户端连接成功，使用协议: {:?}", protocol);

            // 等待连接稳定
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // 发送文本消息
            info!("发送文本消息...");
            let text_result = client.send_text_message("server", "Hello from WebSocket client!").await?;
            if text_result.success {
                info!("文本消息发送成功: {}", text_result.message_id);
            } else {
                warn!("文本消息发送失败: {:?}", text_result.error_message);
            }

            // 发送二进制消息
            info!("发送二进制消息...");
            let binary_data = b"Binary message from WebSocket client".to_vec();
            let binary_result = client.send_binary_message("server", binary_data, "binary".to_string()).await?;
            if binary_result.success {
                info!("二进制消息发送成功: {}", binary_result.message_id);
            } else {
                warn!("二进制消息发送失败: {:?}", binary_result.error_message);
            }

            // 发送自定义消息
            info!("发送自定义消息...");
            let custom_message = ProtoMessage::new(
                uuid::Uuid::new_v4().to_string(),
                "custom".to_string(),
                serde_json::json!({
                    "type": "custom",
                    "client": "websocket",
                    "data": "Custom message from WebSocket client",
                    "timestamp": chrono::Utc::now().timestamp()
                }).to_string().into_bytes(),
            );
            let custom_result = client.send_message("server", custom_message).await?;
            if custom_result.success {
                info!("自定义消息发送成功: {}", custom_result.message_id);
            } else {
                warn!("自定义消息发送失败: {:?}", custom_result.error_message);
            }

            // 获取连接状态
            let status = client.get_status().await;
            info!("当前连接状态: {:?}", status);

            // 获取当前协议（已移除，使用协议竞速器获取）
            info!("当前使用协议: WebSocket");

            // 获取消息队列长度
            let queue_length = client.get_message_queue_length().await;
            info!("消息队列长度: {}", queue_length);

            // 持续运行一段时间，发送心跳消息
            info!("WebSocket 客户端运行中，按 Ctrl+C 停止...");
            
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            let mut counter = 0;
            
            loop {
                interval.tick().await;
                counter += 1;
                
                // 每10秒发送一次心跳消息
                let heartbeat_msg = format!("WebSocket heartbeat message #{}", counter);
                if let Err(e) = client.send_text_message("server", &heartbeat_msg).await {
                    warn!("心跳消息发送失败: {}", e);
                }
                
                // 检查连接状态
                let status = client.get_status().await;
                if status != flare_im::client::connection_manager::ConnectionState::Connected {
                    warn!("连接状态异常: {:?}", status);
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