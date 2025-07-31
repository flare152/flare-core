//! WebSocket服务器示例
//!
//! 展示如何使用Flare IM WebSocket服务器

use std::sync::Arc;
use tokio::sync::RwLock;
use flare_im::{
    server::{
        config::WebSocketConfig,
        websocket_server::WebSocketServer,
        conn_manager::MemoryServerConnectionManager,
        handlers::{AuthHandler, MessageHandler, EventHandler},
    },
    common::{
        conn::{ConnectionEvent, ProtoMessage, Platform},
        Result,
    },
};
use std::net::SocketAddr;

/// 简单的认证处理器
struct SimpleAuthHandler;

#[async_trait::async_trait]
impl AuthHandler for SimpleAuthHandler {
    async fn validate_token(&self, token: &str) -> Result<Option<String>> {
        // 简单的token验证逻辑
        if token == "test_token" {
            Ok(Some("test_user".to_string()))
        } else {
            Ok(None)
        }
    }
}

/// 简单的消息处理器
struct SimpleMessageHandler;

#[async_trait::async_trait]
impl MessageHandler for SimpleMessageHandler {
    async fn handle_user_connect(&self, user_id: &str, session_id: &str, platform: Platform) -> Result<()> {
        println!("用户连接: {} 会话 {} 平台 {:?}", user_id, session_id, platform);
        Ok(())
    }

    async fn handle_user_disconnect(&self, user_id: &str, session_id: &str) -> Result<()> {
        println!("用户断开: {} 会话 {}", user_id, session_id);
        Ok(())
    }

    async fn handle_message(&self, user_id: &str, message: ProtoMessage) -> Result<ProtoMessage> {
        println!("收到消息: 用户 {} 类型 {} 内容 {:?}", user_id, message.message_type, message.payload);
        
        // 简单的echo响应
        Ok(ProtoMessage::new(
            uuid::Uuid::new_v4().to_string(),
            "echo".to_string(),
            message.payload,
        ))
    }

    async fn handle_heartbeat(&self, user_id: &str, session_id: &str) -> Result<()> {
        println!("心跳: 用户 {} 会话 {}", user_id, session_id);
        Ok(())
    }
}

/// 简单的事件处理器
struct SimpleEventHandler;

#[async_trait::async_trait]
impl EventHandler for SimpleEventHandler {
    async fn handle_connection_event(&self, user_id: &str, event: ConnectionEvent) -> Result<()> {
        match event {
            ConnectionEvent::Connected => {
                println!("连接事件: 用户 {} 已连接", user_id);
            }
            ConnectionEvent::Disconnected => {
                println!("连接事件: 用户 {} 已断开", user_id);
            }
            ConnectionEvent::MessageReceived(msg) => {
                println!("消息事件: 用户 {} 收到消息 {:?}", user_id, msg.message_type);
            }
            ConnectionEvent::MessageSent(msg) => {
                println!("消息事件: 用户 {} 发送消息 {:?}", user_id, msg.message_type);
            }
            ConnectionEvent::Heartbeat => {
                println!("心跳事件: 用户 {}", user_id);
            }
            _ => {
                println!("其他事件: 用户 {} {:?}", user_id, event);
            }
        }
        Ok(())
    }

    async fn handle_error(&self, error: &str) -> Result<()> {
        println!("错误事件: {}", error);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 创建WebSocket配置
    let config = WebSocketConfig {
        bind_addr: "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
        enabled: true,
        max_connections: 1000,
        connection_timeout_ms: 300_000, // 5分钟
        heartbeat_interval_ms: 30_000, // 30秒
        enable_tls: false,
        cert_path: None,
        key_path: None,
    };

    let bind_addr = config.bind_addr;

    // 创建连接管理器
    let connection_manager = Arc::new(MemoryServerConnectionManager::new_default());

    // 创建处理器
    let auth_handler = Arc::new(SimpleAuthHandler);
    let message_handler = Arc::new(SimpleMessageHandler);
    let event_handler = Arc::new(SimpleEventHandler);

    // 创建运行状态标志
    let running = Arc::new(RwLock::new(true));

    // 创建WebSocket服务器
    let server = WebSocketServer::new(
        config,
        connection_manager,
        Some(auth_handler),
        Some(message_handler),
        Some(event_handler),
        running.clone(),
    );

    println!("启动WebSocket服务器在: {}", bind_addr);
    println!("使用token 'test_token' 进行认证");

    // 启动服务器
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("服务器错误: {}", e);
        }
    });

    // 等待用户输入来停止服务器
    println!("按回车键停止服务器...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    // 停止服务器
    {
        let mut running_guard = running.write().await;
        *running_guard = false;
    }

    // 等待服务器停止
    if let Err(e) = server_handle.await {
        eprintln!("服务器任务错误: {}", e);
    }

    println!("服务器已停止");
    Ok(())
} 