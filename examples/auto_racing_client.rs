//! 协议竞速客户端示例
//!
//! 展示如何使用 FlareIMClient 进行协议竞速自动选择最佳协议

use flare_im::client::{
    FlareIMClientBuilder, 
    config::{ProtocolSelectionMode, ServerAddresses, ProtocolRacingConfig, ProtocolWeights},
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

    info!("🚀 启动协议竞速客户端示例");

    // 创建事件回调
    let event_callback: Arc<ClientEventCallback> = Arc::new(Box::new(|event| {
        match event {
            ClientEvent::Connected(protocol) => {
                info!("[AutoRacing] 连接成功，使用协议: {:?}", protocol);
            }
            ClientEvent::Disconnected => {
                info!("[AutoRacing] 连接断开");
            }
            ClientEvent::Reconnecting => {
                info!("[AutoRacing] 正在重连");
            }
            ClientEvent::Reconnected(protocol) => {
                info!("[AutoRacing] 重连成功，使用协议: {:?}", protocol);
            }
            ClientEvent::Error(error_msg) => {
                error!("[AutoRacing] 连接错误: {}", error_msg);
            }
            ClientEvent::MessageReceived(message) => {
                info!("[AutoRacing] 收到消息: ID={}, 类型={}, 内容={}", 
                      message.id, message.message_type, 
                      String::from_utf8_lossy(&message.payload));
            }
            ClientEvent::MessageSent(message_id) => {
                info!("[AutoRacing] 消息发送成功: {}", message_id);
            }
            ClientEvent::MessageFailed(message_id, error) => {
                warn!("[AutoRacing] 消息发送失败: {} - {}", message_id, error);
            }
            ClientEvent::Heartbeat => {
                debug!("[AutoRacing] 心跳");
            }
            ClientEvent::ProtocolSwitched(protocol) => {
                info!("[AutoRacing] 协议切换到: {:?}", protocol);
            }
            ClientEvent::ReconnectFailed => {
                error!("[AutoRacing] 重连失败");
            }
        }
    }));

    // 创建协议权重配置
    let protocol_weights = ProtocolWeights::new()
        .with_quic_weight(0.7)      // QUIC 权重更高
        .with_websocket_weight(0.3);

    // 创建协议竞速配置
    let racing_config = ProtocolRacingConfig {
        enabled: true,
        timeout_ms: 3000,           // 3秒超时
        test_message_count: 5,      // 测试5条消息
        protocol_weights,
        auto_fallback: true,
        racing_interval_ms: 60000,  // 每分钟重新竞速
    };

    // 创建服务器地址配置（配置 QUIC 和 WebSocket）
    let server_addresses = ServerAddresses::new()
        .with_quic_url("quic://127.0.0.1:4010".to_string())
        .with_websocket_url("ws://127.0.0.1:4000".to_string());

    // 创建客户端，使用协议竞速模式
    let mut client = FlareIMClientBuilder::new("racing_user".to_string())
        .server_addresses(server_addresses)
        .protocol_selection_mode(ProtocolSelectionMode::AutoRacing)
        .protocol_racing(racing_config)
        .connection_timeout(5000)
        .heartbeat_interval(30000)
        .max_reconnect_attempts(3)
        .reconnect_delay(1000)
        .auto_reconnect(true)
        .message_retry(3, 1000)
        .buffer_size(8192)
        .compression(true)
        .encryption(true)
        .tls(true)  // 协议竞速使用 TLS
        .build()?
        .with_event_callback(event_callback);

    info!("协议竞速客户端创建成功，开始连接...");

    // 连接到服务器
    match client.connect().await {
        Ok(protocol) => {
            info!("协议竞速客户端连接成功，使用协议: {:?}", protocol);

            // 等待连接稳定
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // 发送文本消息
            info!("发送文本消息...");
            let text_result = client.send_text_message("server", "Hello from AutoRacing client!").await?;
            if text_result.success {
                info!("文本消息发送成功: {}", text_result.message_id);
            } else {
                warn!("文本消息发送失败: {:?}", text_result.error_message);
            }

            // 发送二进制消息
            info!("发送二进制消息...");
            let binary_data = b"Binary message from AutoRacing client".to_vec();
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
                    "client": "autoracing",
                    "data": "Custom message from AutoRacing client",
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
            info!("当前使用协议: 自动选择");

            // 获取协议性能指标
            let metrics = client.get_all_protocol_metrics().await;
            info!("协议性能指标: {:?}", metrics);

            // 获取最佳协议
            if let Some(best_protocol) = client.get_best_protocol().await {
                info!("最佳协议: {:?}", best_protocol);
            }

            // 检查协议可用性
            let quic_available = client.is_protocol_available(TransportProtocol::QUIC).await;
            let ws_available = client.is_protocol_available(TransportProtocol::WebSocket).await;
            info!("协议可用性 - QUIC: {}, WebSocket: {}", quic_available, ws_available);

            // 获取消息队列长度
            let queue_length = client.get_message_queue_length().await;
            info!("消息队列长度: {}", queue_length);

            // 持续运行一段时间，发送心跳消息
            info!("协议竞速客户端运行中，按 Ctrl+C 停止...");
            
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            let mut counter = 0;
            
            loop {
                interval.tick().await;
                counter += 1;
                
                // 每10秒发送一次心跳消息
                let heartbeat_msg = format!("AutoRacing heartbeat message #{}", counter);
                if let Err(e) = client.send_text_message("server", &heartbeat_msg).await {
                    warn!("心跳消息发送失败: {}", e);
                }
                
                // 检查连接状态
                let status = client.get_status().await;
                if status != flare_im::client::connection_manager::ConnectionState::Connected {
                    warn!("连接状态异常: {:?}", status);
                }
                
                // 每30秒检查一次协议性能
                if counter % 3 == 0 {
                    let metrics = client.get_all_protocol_metrics().await;
                    info!("当前协议性能指标: {:?}", metrics);
                }
                
                // 运行60秒后停止
                if counter >= 6 {
                    break;
                }
            }

            // 断开连接
            info!("断开协议竞速连接...");
            client.disconnect().await?;
            info!("协议竞速客户端已断开连接");
        }
        Err(e) => {
            error!("协议竞速客户端连接失败: {}", e);
        }
    }

    info!("✅ 协议竞速客户端示例运行完成");
    Ok(())
} 