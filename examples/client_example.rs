//! Flare IM 客户端示例
//!
//! 演示如何使用FlareIMClient和不同的协议选择

use std::sync::Arc;
use flare_core::{
    client::{
        FlareIMClient, FlareIMClientBuilder,
    },
    common::{TransportProtocol, Result},
};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🚀 Flare IM 客户端示例");
    println!("======================");
    println!("选择连接模式:");
    println!("1. 仅WebSocket");
    println!("2. 仅QUIC");
    println!("3. 自动选择协议");
    println!("4. 默认模式 (自动选择)");
    
    // 示例1: 仅使用WebSocket连接
    println!("\n📡 示例1: 仅使用WebSocket连接");
    let client1 = FlareIMClientBuilder::new("user1".to_string())
        .server_addresses(flare_core::client::config::ServerAddresses::new()
            .with_websocket_url("ws://127.0.0.1:4000".to_string()))
        .protocol_selection_mode(flare_core::client::config::ProtocolSelectionMode::Specific(TransportProtocol::WebSocket))
        .build()?;
    
    // 示例2: 仅使用QUIC连接
    println!("\n📡 示例2: 仅使用QUIC连接");
    let client2 = FlareIMClientBuilder::new("user2".to_string())
        .server_addresses(flare_core::client::config::ServerAddresses::new()
            .with_quic_url("quic://127.0.0.1:4010".to_string()))
        .protocol_selection_mode(flare_core::client::config::ProtocolSelectionMode::Specific(TransportProtocol::QUIC))
        .build()?;
    
    // 示例3: 自动选择协议
    println!("\n📡 示例3: 自动选择协议");
    let client3 = FlareIMClientBuilder::new("user3".to_string())
        .server_addresses(flare_core::client::config::ServerAddresses::new()
            .with_websocket_url("ws://127.0.0.1:4000".to_string())
            .with_quic_url("quic://127.0.0.1:4010".to_string()))
        .protocol_selection_mode(flare_core::client::config::ProtocolSelectionMode::AutoRacing)
        .build()?;
    
    // 示例4: 使用协议选择枚举
    println!("\n📡 示例4: 使用协议选择枚举");
    let client4 = FlareIMClientBuilder::new("user4".to_string())
        .server_addresses(flare_core::client::config::ServerAddresses::new()
            .with_websocket_url("ws://127.0.0.1:4000".to_string())
            .with_quic_url("quic://127.0.0.1:4010".to_string()))
        .protocol_selection_mode(flare_core::client::config::ProtocolSelectionMode::AutoRacing)
        .build()?;
    
    // 运行默认客户端 (自动选择协议)
    println!("\n🚀 启动默认客户端 (自动选择协议)");
    let mut client = FlareIMClientBuilder::new("test_user".to_string())
        .server_addresses(flare_core::client::config::ServerAddresses::new()
            .with_websocket_url("ws://127.0.0.1:4000".to_string())
            .with_quic_url("quic://127.0.0.1:4010".to_string()))
        .protocol_selection_mode(flare_core::client::config::ProtocolSelectionMode::AutoRacing)
        .build()?;
    
    // 连接到服务器
    println!("正在连接到服务器...");
    client.connect().await?;
    
    println!("✅ 客户端已连接!");
    println!("用户ID: test_user");
    println!("连接状态: 已连接");
    println!("传输协议: 自动选择");
    println!("\n按 Ctrl+C 断开连接");
    
    // 等待中断信号
    tokio::signal::ctrl_c().await.unwrap_or_else(|_| {
        println!("无法监听中断信号");
    });
    
    // 断开连接
    client.disconnect().await?;
    
    println!("\n👋 客户端已断开连接");
    Ok(())
} 