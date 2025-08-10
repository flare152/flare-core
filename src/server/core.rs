//! Flare IM 服务端核心模块
//!
//! 提供统一的QUIC和WebSocket服务端

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::common::{
    Result, ProtocolSelection,
    MessageParser,
};

use super::{
    config::ServerConfig,
    conn_manager::{
        MemoryServerConnectionManager, 
        ServerConnectionManager, ServerConnectionManagerConfig
    },
    websocket_server::WebSocketServer,
    quic_server::QuicServer,
    message_processor::{DefaultMessageProcessor},
};

/// Flare IM 服务端
/// 提供统一的QUIC和WebSocket服务端
pub struct FlareIMServer {
    /// 配置
    config: ServerConfig,
    /// 连接管理器
    connection_manager: Arc<dyn ServerConnectionManager>,
    /// 消息解析器
    message_parser: Arc<MessageParser>,
    /// WebSocket服务器
    websocket_server: Option<WebSocketServer>,
    /// QUIC服务器
    quic_server: Option<QuicServer>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl FlareIMServer {
    /// 创建新的 Flare IM 服务端
    pub fn new(
        config: ServerConfig,
        connection_manager: Arc<dyn ServerConnectionManager>,
        message_parser: Arc<MessageParser>,
    ) -> Self {
        Self {
            config,
            connection_manager,
            message_parser,
            websocket_server: None,
            quic_server: None,
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// 使用默认配置创建新的 Flare IM 服务端
    pub fn with_default_config(config: ServerConfig) -> Self {
        // 创建连接管理器配置
        let conn_config = ServerConnectionManagerConfig {
            max_connections: config.connection_manager.max_connections,
            connection_timeout_ms: config.connection_manager.connection_timeout_ms,
            heartbeat_interval_ms: config.connection_manager.heartbeat_interval_ms,
            heartbeat_timeout_ms: config.connection_manager.heartbeat_timeout_ms,
            max_missed_heartbeats: config.connection_manager.max_missed_heartbeats as u64,
            cleanup_interval_ms: config.connection_manager.cleanup_interval_ms,
            enable_auto_reconnect: config.connection_manager.enable_auto_reconnect,
            max_reconnect_attempts: config.connection_manager.max_reconnect_attempts,
            reconnect_delay_ms: config.connection_manager.reconnect_delay_ms,
        };
        
        let connection_manager = Arc::new(MemoryServerConnectionManager::new(conn_config));
        let message_parser = Arc::new(MessageParser::with_default_callbacks());
        
        Self::new(config, connection_manager, message_parser)
    }
    
    /// 设置消息解析器
    pub fn with_message_parser(mut self, parser: Arc<MessageParser>) -> Self {
        self.message_parser = parser;
        self
    }

    /// 启动服务端
    pub async fn start(&mut self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }
        
        // 启动连接管理器
        ServerConnectionManager::start(&*self.connection_manager).await?;
        
        // 根据协议选择启动相应的服务器
        match self.config.protocol.selection {
            ProtocolSelection::WebSocketOnly => {
                self.start_websocket_server().await?;
                info!("Flare IM 服务端已启动 (仅WebSocket)");
            }
            ProtocolSelection::QuicOnly => {
                self.start_quic_server().await?;
                info!("Flare IM 服务端已启动 (仅QUIC)");
            }
            ProtocolSelection::Both => {
                if self.config.protocol.websocket.enabled {
                    self.start_websocket_server().await?;
                }
                if self.config.protocol.quic.enabled {
                    self.start_quic_server().await?;
                }
                info!("Flare IM 服务端已启动 (WebSocket + QUIC)");
            }
            ProtocolSelection::Auto => {
                // 自动选择：优先QUIC，如果QUIC不可用则使用WebSocket
                if self.config.protocol.quic.enabled {
                    match self.start_quic_server().await {
                        Ok(_) => {
                            info!("Flare IM 服务端已启动 (自动选择: QUIC)");
                        }
                        Err(e) => {
                            warn!("QUIC启动失败，回退到WebSocket: {}", e);
                            if self.config.protocol.websocket.enabled {
                                self.start_websocket_server().await?;
                                info!("Flare IM 服务端已启动 (自动选择: WebSocket)");
                            } else {
                                return Err(e);
                            }
                        }
                    }
                } else if self.config.protocol.websocket.enabled {
                    self.start_websocket_server().await?;
                    info!("Flare IM 服务端已启动 (自动选择: WebSocket)");
                } else {
                    return Err(crate::common::error::FlareError::general_error("自动模式下没有可用的协议"));
                }
            }
        }

        Ok(())
    }

    /// 停止服务端
    pub async fn stop(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        
        *running = false;
        
        // 停止WebSocket服务器
        if let Some(ref mut ws_server) = self.websocket_server {
            if let Err(e) = ws_server.stop().await {
                warn!("停止WebSocket服务器失败: {}", e);
            }
        }
        
        // 停止QUIC服务器
        if let Some(ref mut quic_server) = self.quic_server {
            if let Err(e) = quic_server.stop().await {
                warn!("停止QUIC服务器失败: {}", e);
            }
        }
        
        // 停止连接管理器
        ServerConnectionManager::stop(&*self.connection_manager).await?;
        
        info!("Flare IM 服务端已停止");
        Ok(())
    }

    /// 获取连接管理器
    pub fn get_connection_manager(&self) -> Arc<dyn ServerConnectionManager> {
        Arc::clone(&self.connection_manager)
    }

    /// 获取消息解析器
    pub fn get_message_parser(&self) -> Arc<MessageParser> {
        Arc::clone(&self.message_parser)
    }
    
    /// 启动WebSocket服务器
    async fn start_websocket_server(&mut self) -> Result<()> {
        let config = self.config.protocol.websocket.clone();
        let connection_manager = Arc::clone(&self.connection_manager);
        let message_parser = Arc::clone(&self.message_parser);
        let running = Arc::clone(&self.running);
        
        let mut server = WebSocketServer::new(
            config,
            connection_manager,
            message_parser,
            running,
        );
        
        server.start().await?;
        self.websocket_server = Some(server);
        Ok(())
    }
    
    /// 启动QUIC服务器
    async fn start_quic_server(&mut self) -> Result<()> {
        let config = self.config.protocol.quic.clone();
        let connection_manager = Arc::clone(&self.connection_manager);
        let message_parser = Arc::clone(&self.message_parser);
        let running = Arc::clone(&self.running);
        
        let mut server = QuicServer::new(
            config,
            connection_manager,
            message_parser,
            running,
        );
        
        server.start().await?;
        self.quic_server = Some(server);
        Ok(())
    }
}

/// Flare IM 服务端构建器
pub struct FlareIMServerBuilder {
    config: ServerConfig,
    connection_manager: Option<Arc<dyn ServerConnectionManager>>,
    message_parser: Option<Arc<MessageParser>>,
}

impl FlareIMServerBuilder {
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
            connection_manager: None,
            message_parser: None,
        }
    }
    
    /// 设置协议选择
    pub fn protocol_selection(mut self, selection: ProtocolSelection) -> Self {
        self.config.protocol.selection = selection;
        self
    }
    
    /// 仅使用WebSocket
    pub fn websocket_only(mut self) -> Self {
        self.config.protocol.selection = ProtocolSelection::WebSocketOnly;
        self.config.protocol.websocket.enabled = true;
        self.config.protocol.quic.enabled = false;
        self
    }
    
    /// 仅使用QUIC
    pub fn quic_only(mut self) -> Self {
        self.config.protocol.selection = ProtocolSelection::QuicOnly;
        self.config.protocol.websocket.enabled = false;
        self.config.protocol.quic.enabled = true;
        self
    }
    
    /// 同时使用WebSocket和QUIC
    pub fn both_protocols(mut self) -> Self {
        self.config.protocol.selection = ProtocolSelection::Both;
        self.config.protocol.websocket.enabled = true;
        self.config.protocol.quic.enabled = true;
        self
    }
    
    /// 自动选择协议
    pub fn auto_protocol(mut self) -> Self {
        self.config.protocol.selection = ProtocolSelection::Auto;
        self.config.protocol.websocket.enabled = true;
        self.config.protocol.quic.enabled = true;
        self
    }
    
    /// 设置连接管理器 (必须实现 ServerConnectionManager trait)
    pub fn with_connection_manager(mut self, manager: Arc<dyn ServerConnectionManager>) -> Self {
        // 从提供的连接管理器获取配置
        let config = manager.get_config().clone();
        
        // 创建新的 MemoryServerConnectionManager 实例，使用相同的配置
        // 这样可以确保类型一致性，同时允许用户提供自定义的连接管理器
        self.connection_manager = Some(manager);
        
        self
    }
    
    /// 设置消息解析器
    pub fn with_message_parser(mut self, parser: Arc<MessageParser>) -> Self {
        self.message_parser = Some(parser);
        self
    }

    pub fn websocket_addr(mut self, addr: std::net::SocketAddr) -> Self {
        self.config.protocol.websocket.bind_addr = addr.to_string();
        self
    }

    pub fn quic_addr(mut self, addr: std::net::SocketAddr) -> Self {
        self.config.protocol.quic.bind_addr = addr.to_string();
        self
    }

    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.connection_manager.max_connections = max;
        // 如果已经设置了连接管理器，则更新其配置
        if let Some(ref manager) = self.connection_manager {
            let current_config = manager.get_config();
            let new_config = ServerConnectionManagerConfig {
                max_connections: max,
                connection_timeout_ms: current_config.connection_timeout_ms,
                heartbeat_interval_ms: current_config.heartbeat_interval_ms,
                heartbeat_timeout_ms: current_config.heartbeat_timeout_ms,
                max_missed_heartbeats: current_config.max_missed_heartbeats,
                cleanup_interval_ms: current_config.cleanup_interval_ms,
                enable_auto_reconnect: current_config.enable_auto_reconnect,
                max_reconnect_attempts: current_config.max_reconnect_attempts,
                reconnect_delay_ms: current_config.reconnect_delay_ms,
            };
            // 创建一个新的 MemoryServerConnectionManager 实例
            self.connection_manager = Some(Arc::new(MemoryServerConnectionManager::new(new_config)));
        }
        self
    }
    
    /// 配置 QUIC TLS 证书
    pub fn quic_tls(mut self, cert_path: String, key_path: String) -> Self {
        self.config.protocol.quic.cert_path = cert_path;
        self.config.protocol.quic.key_path = key_path;
        self
    }
    
    pub fn build(self) -> Result<FlareIMServer> {
        // 验证必需字段并提供默认值
        let connection_manager = self.connection_manager
            .unwrap_or_else(|| Arc::new(MemoryServerConnectionManager::new(ServerConnectionManagerConfig::default())));

        let message_parser = self.message_parser
            .unwrap_or_else(|| Arc::new(MessageParser::with_default_callbacks()));

        // 验证连接管理器配置
        let config = connection_manager.get_config();
        if config.max_connections == 0 {
            return Err(crate::common::error::FlareError::InvalidConfiguration(
                "连接管理器最大连接数不能为0".to_string()
            ));
        }

        // 创建服务器实例
        let server = FlareIMServer::new(self.config, connection_manager, message_parser);

        Ok(server)
    }
}

impl Default for FlareIMServerBuilder {
    fn default() -> Self {
        Self::new()
    }
} 

 