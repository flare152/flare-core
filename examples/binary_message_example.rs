use flare_im::server::{
    FlareIMServer, FlareIMServerBuilder, ServerConfigBuilder,
    MessageHandler, DefaultMessageHandler, DefaultAuthHandler, DefaultEventHandler,
};
use flare_im::common::conn::ProtoMessage;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// 自定义二进制消息处理器示例
struct CustomBinaryMessageHandler;

#[async_trait]
impl MessageHandler for CustomBinaryMessageHandler {
    async fn handle_message(&self, user_id: &str, message: ProtoMessage) -> flare_im::Result<ProtoMessage> {
        println!("收到用户 {} 的消息: 类型={}, 载荷大小={} 字节", 
                 user_id, message.message_type, message.payload.len());
        
        // 根据消息类型处理不同的二进制数据
        match message.message_type.as_str() {
            "text" => {
                // 处理文本消息
                let text = String::from_utf8_lossy(&message.payload);
                println!("文本消息: {}", text);
                
                // 返回确认消息
                let response = format!("收到文本消息: {}", text);
                Ok(ProtoMessage::new(
                    Uuid::new_v4().to_string(),
                    "text_ack".to_string(),
                    response.into_bytes(),
                ))
            }
            "image" => {
                // 处理图片消息
                println!("图片消息: {} 字节", message.payload.len());
                
                // 返回图片处理结果
                let response = format!("图片已处理，大小: {} 字节", message.payload.len());
                Ok(ProtoMessage::new(
                    Uuid::new_v4().to_string(),
                    "image_ack".to_string(),
                    response.into_bytes(),
                ))
            }
            "file" => {
                // 处理文件消息
                println!("文件消息: {} 字节", message.payload.len());
                
                // 返回文件处理结果
                let response = format!("文件已接收，大小: {} 字节", message.payload.len());
                Ok(ProtoMessage::new(
                    Uuid::new_v4().to_string(),
                    "file_ack".to_string(),
                    response.into_bytes(),
                ))
            }
            "custom_binary" => {
                // 处理自定义二进制数据
                println!("自定义二进制数据: {} 字节", message.payload.len());
                
                // 直接返回原始二进制数据（echo）
                Ok(ProtoMessage::new(
                    Uuid::new_v4().to_string(),
                    "custom_binary_echo".to_string(),
                    message.payload, // 保持原始二进制格式
                ))
            }
            _ => {
                // 默认处理：返回原始数据
                println!("未知消息类型: {}", message.message_type);
                Ok(ProtoMessage::new(
                    Uuid::new_v4().to_string(),
                    "echo".to_string(),
                    message.payload, // 保持原始二进制格式
                ))
            }
        }
    }
    
    async fn handle_user_connect(&self, user_id: &str, session_id: &str, platform: flare_im::common::conn::Platform) -> flare_im::Result<()> {
        println!("用户 {} 连接，会话: {}，平台: {:?}", user_id, session_id, platform);
        Ok(())
    }
    
    async fn handle_user_disconnect(&self, user_id: &str, session_id: &str) -> flare_im::Result<()> {
        println!("用户 {} 断开，会话: {}", user_id, session_id);
        Ok(())
    }
    
    async fn handle_heartbeat(&self, user_id: &str, session_id: &str) -> flare_im::Result<()> {
        println!("用户 {} 心跳，会话: {}", user_id, session_id);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("=== FlareIM 二进制消息示例 ===");
    println!("此示例展示了如何使用统一的二进制消息格式处理不同类型的消息");
    println!("");
    
    // 创建服务器配置
    let config = ServerConfigBuilder::new()
        .quic_addr("127.0.0.1:4433".parse().unwrap())
        .websocket_addr("127.0.0.1:8080".parse().unwrap())
        .enable_quic(true)
        .enable_websocket(true)
        .max_connections(100)
        .build();
    
    // 创建服务器
    let server = FlareIMServerBuilder::new()
        .with_config(config)
        .with_auth_handler(Arc::new(DefaultAuthHandler::new()))
        .with_message_handler(Arc::new(CustomBinaryMessageHandler))
        .with_event_handler(Arc::new(DefaultEventHandler::new()))
        .build();
    
    println!("服务器配置完成");
    println!("- QUIC 监听: 127.0.0.1:4433");
    println!("- WebSocket 监听: 127.0.0.1:8080");
    println!("- 消息处理器: CustomBinaryMessageHandler");
    println!("");
    println!("支持的消息类型:");
    println!("- text: 文本消息");
    println!("- image: 图片消息");
    println!("- file: 文件消息");
    println!("- custom_binary: 自定义二进制数据");
    println!("- 其他: 默认echo响应");
    println!("");
    println!("服务器已启动，等待连接...");
    
    // 启动服务器
    server.start().await?;
    
    // 保持服务器运行一段时间
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    println!("30秒后自动停止服务器...");
    
    server.stop().await?;
    println!("服务器已停止");
    
    Ok(())
} 