//! Flare IM 服务端示例
//!
//! 演示如何使用FlareIMServer构建器和不同的协议选择

use std::sync::Arc;
use flare_im::{
    server::{
        FlareIMServer, FlareIMServerBuilder,
        DefaultAuthHandler, DefaultMessageHandler, DefaultEventHandler,
        MessageProcessingCenter,
    },
    common::{ProtocolSelection, Result},
};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🚀 Flare IM 服务端示例");
    println!("======================");
    println!("选择运行模式:");
    println!("1. 仅WebSocket");
    println!("2. 仅QUIC");
    println!("3. 同时使用WebSocket和QUIC");
    println!("4. 自动选择协议");
    println!("5. 默认模式 (同时使用WebSocket和QUIC)");
    
    // 这里可以根据用户输入选择不同的模式
    // 为了演示，我们展示几种不同的配置方式
    
    // 示例1: 仅使用WebSocket
    println!("\n📡 示例1: 仅使用WebSocket");
    let server1 = FlareIMServerBuilder::new()
        .websocket_only()
        .websocket_addr("127.0.0.1:4001".parse().unwrap())
        .with_auth_handler(Arc::new(DefaultAuthHandler::new()))
        .with_message_handler(Arc::new(DefaultMessageHandler::new()))
        .with_event_handler(Arc::new(DefaultEventHandler::new()))
        .build();
    
    // 示例2: 仅使用QUIC
    println!("\n📡 示例2: 仅使用QUIC");
    let server2 = FlareIMServerBuilder::new()
        .quic_only()
        .quic_addr("127.0.0.1:4011".parse().unwrap())
        .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
        .with_auth_handler(Arc::new(DefaultAuthHandler::new()))
        .with_message_handler(Arc::new(DefaultMessageHandler::new()))
        .with_event_handler(Arc::new(DefaultEventHandler::new()))
        .build();
    
    // 示例3: 同时使用WebSocket和QUIC
    println!("\n📡 示例3: 同时使用WebSocket和QUIC");
    let server3 = FlareIMServerBuilder::new()
        .both_protocols()
        .websocket_addr("127.0.0.1:4002".parse().unwrap())
        .quic_addr("127.0.0.1:4012".parse().unwrap())
        .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
        .with_auth_handler(Arc::new(DefaultAuthHandler::new()))
        .with_message_handler(Arc::new(DefaultMessageHandler::new()))
        .with_event_handler(Arc::new(DefaultEventHandler::new()))
        .build();
    
    // 示例4: 自动选择协议
    println!("\n📡 示例4: 自动选择协议");
    let server4 = FlareIMServerBuilder::new()
        .auto_protocol()
        .websocket_addr("127.0.0.1:4003".parse().unwrap())
        .quic_addr("127.0.0.1:4013".parse().unwrap())
        .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
        .with_auth_handler(Arc::new(DefaultAuthHandler::new()))
        .with_message_handler(Arc::new(DefaultMessageHandler::new()))
        .with_event_handler(Arc::new(DefaultEventHandler::new()))
        .build();
    
    // 示例5: 使用协议选择枚举
    println!("\n📡 示例5: 使用协议选择枚举");
    let server5 = FlareIMServerBuilder::new()
        .protocol_selection(ProtocolSelection::Both)
        .websocket_addr("127.0.0.1:4004".parse().unwrap())
        .quic_addr("127.0.0.1:4014".parse().unwrap())
        .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
        .with_auth_handler(Arc::new(DefaultAuthHandler::new()))
        .with_message_handler(Arc::new(DefaultMessageHandler::new()))
        .with_event_handler(Arc::new(DefaultEventHandler::new()))
        .build();
    
    // 运行默认服务器 (同时使用WebSocket和QUIC)
    println!("\n🚀 启动默认服务器 (同时使用WebSocket和QUIC)");
    let server = FlareIMServerBuilder::new()
        .both_protocols()
        .websocket_addr("127.0.0.1:4000".parse().unwrap())
        .quic_addr("127.0.0.1:4010".parse().unwrap())
        .quic_tls("certs/server.crt".to_string(), "certs/server.key".to_string())
        .with_auth_handler(Arc::new(DefaultAuthHandler::new()))
        .with_message_handler(Arc::new(DefaultMessageHandler::new()))
        .with_event_handler(Arc::new(DefaultEventHandler::new()))
        .build();
    
    // 启动服务器
    if let Err(e) = server.start().await {
        eprintln!("\n⚠️  服务器启动遇到问题: {}", e);
        eprintln!("服务器将继续运行，但可能无法正常工作");
        eprintln!("请检查:");
        eprintln!("1. 端口是否被占用");
        eprintln!("2. 证书文件是否存在");
        eprintln!("3. 网络配置是否正确");
    } else {
        println!("\n✅ 服务器已启动!");
        println!("WebSocket: ws://127.0.0.1:4000");
        println!("QUIC: quic://127.0.0.1:4010");
    }
    
    println!("\n按 Ctrl+C 停止服务器");
    
    // 等待中断信号
    tokio::signal::ctrl_c().await?;
    
    // 停止服务器
    server.stop().await?;
    
    println!("\n👋 服务器已停止");
    Ok(())
}

 