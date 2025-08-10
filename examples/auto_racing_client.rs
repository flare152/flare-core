//! 协议竞速客户端示例
//!
//! 展示如何使用 FlareIMClient 进行协议竞速自动选择最佳协议

use flare_core::client::{
    Client, config::{ClientConfigBuilder, ProtocolSelectionMode, ServerAddresses, ProtocolRacingConfig, ProtocolWeights},
    protocol_racer::ClientEvent,
    types::MessagePriority,
};
use flare_core::common::{TransportProtocol, UnifiedProtocolMessage};
use std::sync::Arc;
use tracing::{info, warn, error, debug};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("flare_core=info,flare_core::client=debug")
        .init();

    info!("🚀 启动协议竞速客户端示例");

    // 创建事件回调
    let event_callback = Arc::new(Box::new(|event: ClientEvent| {
        match event {
            ClientEvent::Connect(_) => {
                info!("[AutoRacing] 连接成功");
            }
            ClientEvent::Disconnect(_) => {
                info!("[AutoRacing] 连接断开");
            }
            ClientEvent::Heartbeat(_) => {
                debug!("[AutoRacing] 心跳");
            }
            ClientEvent::Custom(_) => {
                info!("[AutoRacing] 自定义事件");
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

    // 创建客户端配置，使用协议竞速模式
    let config = ClientConfigBuilder::new("racing_user".to_string())
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
        .build()?;

    // 创建客户端
    let mut client = Client::new(config);

    info!("协议竞速客户端创建成功，开始连接...");

    // 连接到服务器
    match client.connect().await {
        Ok(()) => {
            info!("协议竞速客户端连接成功");

            // 等待连接稳定
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // 发送测试消息
            info!("发送测试消息...");
            let test_message = UnifiedProtocolMessage::text(
                "Hello from auto-racing client!".to_string(),
            );
            let test_result = client.send_message(
                test_message,
                MessagePriority::Normal,
                "server".to_string(),
            ).await?;
            info!("测试消息发送成功: {:?}", test_result);

            // 发送性能测试消息
            info!("发送性能测试消息...");
            for i in 0..5 {
                let perf_message = UnifiedProtocolMessage::text(
                    format!("QUIC 性能测试消息 #{}", i + 1),
                );
                let perf_result = client.send_message(
                    perf_message,
                    MessagePriority::Low,
                    "server".to_string(),
                ).await?;
                info!("性能测试消息 #{} 发送成功: {:?}", i + 1, perf_result);
                
                // 短暂延迟
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // 发送自定义消息
            info!("发送自定义消息...");
            let custom_message = UnifiedProtocolMessage::custom_message(
                "performance_test".to_string(),
                serde_json::json!({
                    "type": "performance_test",
                    "client": "auto_racing",
                    "data": "Performance test from auto-racing client",
                    "timestamp": chrono::Utc::now().timestamp(),
                    "protocol_racing": true
                }).to_string().into_bytes(),
            );
            let custom_result = client.send_message(
                custom_message,
                MessagePriority::Normal,
                "server".to_string(),
            ).await?;
            info!("自定义消息发送成功: {:?}", custom_result);

            // 检查连接状态
            let is_connected = client.is_connected().await;
            info!("当前连接状态: {}", is_connected);

            // 持续运行一段时间，发送心跳消息
            info!("协议竞速客户端运行中，按 Ctrl+C 停止...");
            
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            let mut counter = 0;
            
            loop {
                interval.tick().await;
                counter += 1;
                
                // 每10秒发送一次心跳消息
                let heartbeat_msg = format!("Auto-racing heartbeat message #{}", counter);
                let heartbeat_message = UnifiedProtocolMessage::text(heartbeat_msg);
                if let Err(e) = client.send_message(
                    heartbeat_message,
                    MessagePriority::Low,
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