//! Flare IM 服务端配置模块
//!
//! 提供服务端配置管理功能

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::common::ProtocolSelection;

/// 服务器协议配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProtocolConfig {
    /// 协议选择
    pub selection: ProtocolSelection,
    /// WebSocket配置
    pub websocket: WebSocketServerConfig,
    /// QUIC配置
    pub quic: QuicServerConfig,
}

impl Default for ServerProtocolConfig {
    fn default() -> Self {
        Self {
            selection: ProtocolSelection::Both,
            websocket: WebSocketServerConfig::default(),
            quic: QuicServerConfig::default(),
        }
    }
}

/// WebSocket服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketServerConfig {
    /// 绑定地址
    pub bind_addr: String,
    /// 是否启用
    pub enabled: bool,
    /// 最大连接数
    pub max_connections: usize,
    /// 是否启用TLS
    pub enable_tls: bool,
    /// TLS证书路径
    pub cert_path: Option<String>,
    /// TLS私钥路径
    pub key_path: Option<String>,
}

impl Default for WebSocketServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:4000".to_string(),
            enabled: true,
            max_connections: 10000,
            enable_tls: false,
            cert_path: None,
            key_path: None,
        }
    }
}

/// QUIC服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicServerConfig {
    /// 绑定地址
    pub bind_addr: String,
    /// 是否启用
    pub enabled: bool,
    /// 最大连接数
    pub max_connections: usize,
    /// TLS证书路径
    pub cert_path: String,
    /// TLS私钥路径
    pub key_path: String,
    /// ALPN协议
    pub alpn_protocols: Vec<Vec<u8>>,
    /// 是否启用0-RTT
    pub enable_0rtt: bool,
}

impl Default for QuicServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:4010".to_string(),
            enabled: true,
            max_connections: 10000,
            cert_path: "certs/server.crt".to_string(),
            key_path: "certs/server.key".to_string(),
            alpn_protocols: vec![b"flare-core".to_vec()],
            enable_0rtt: true,
        }
    }
}

/// 服务端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 协议配置
    pub protocol: ServerProtocolConfig,
    /// 连接管理器配置
    pub connection_manager: ConnectionManagerConfig,
    /// 认证配置
    pub auth: AuthConfig,
    /// 日志配置
    pub logging: LoggingConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            protocol: ServerProtocolConfig::default(),
            connection_manager: ConnectionManagerConfig::default(),
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// 连接管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionManagerConfig {
    /// 最大连接数
    pub max_connections: usize,
    /// 连接超时时间（毫秒）
    pub connection_timeout_ms: u64,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 心跳超时时间（毫秒）
    pub heartbeat_timeout_ms: u64,
    /// 最大心跳丢失次数
    pub max_missed_heartbeats: u32,
    /// 清理间隔（毫秒）
    pub cleanup_interval_ms: u64,
    /// 是否启用自动重连
    pub enable_auto_reconnect: bool,
    /// 最大重连次数
    pub max_reconnect_attempts: u32,
    /// 重连延迟（毫秒）
    pub reconnect_delay_ms: u64,
}

impl Default for ConnectionManagerConfig {
    fn default() -> Self {
        Self {
            max_connections: 100000, // 10万连接
            connection_timeout_ms: 300000, // 5分钟
            heartbeat_interval_ms: 30000, // 30秒
            heartbeat_timeout_ms: 60000, // 60秒
            max_missed_heartbeats: 3,
            cleanup_interval_ms: 60000, // 1分钟
            enable_auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_delay_ms: 1000, // 1秒
        }
    }
}

/// 认证方法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// 基于令牌的认证
    Token,
    /// 基于密码的认证
    Password,
    /// 匿名认证
    Anonymous,
    /// OAuth2认证
    OAuth2,
    /// JWT认证
    JWT,
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// 是否启用认证
    pub enabled: bool,
    /// 认证方法
    pub method: AuthMethod,
    /// 认证超时时间（秒）
    pub timeout_secs: u64,
    /// JWT密钥
    pub jwt_secret: Option<String>,
    /// JWT过期时间（秒）
    pub jwt_expiry_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            method: AuthMethod::Anonymous,
            timeout_secs: 30,
            jwt_secret: None,
            jwt_expiry_secs: 3600, // 1小时
        }
    }
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: String,
    /// 日志文件路径
    pub file_path: Option<String>,
    /// 是否启用控制台输出
    pub enable_console: bool,
    /// 是否启用文件输出
    pub enable_file: bool,
    /// 日志轮转大小（MB）
    pub rotation_size_mb: u64,
    /// 保留日志文件数量
    pub max_files: usize,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: None,
            enable_console: true,
            enable_file: false,
            rotation_size_mb: 100,
            max_files: 10,
        }
    }
}

/// 配置构建器
pub struct ServerConfigBuilder {
    config: ServerConfig,
}

impl ServerConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
        }
    }
    
    /// 设置协议选择
    pub fn protocol_selection(mut self, selection: ProtocolSelection) -> Self {
        self.config.protocol.selection = selection;
        self
    }
    
    /// 设置WebSocket地址
    pub fn websocket_addr(mut self, addr: SocketAddr) -> Self {
        self.config.protocol.websocket.bind_addr = addr.to_string();
        self
    }
    
    /// 设置QUIC地址
    pub fn quic_addr(mut self, addr: SocketAddr) -> Self {
        self.config.protocol.quic.bind_addr = addr.to_string();
        self
    }
    
    /// 启用WebSocket
    pub fn enable_websocket(mut self, enabled: bool) -> Self {
        self.config.protocol.websocket.enabled = enabled;
        self
    }
    
    /// 启用QUIC
    pub fn enable_quic(mut self, enabled: bool) -> Self {
        self.config.protocol.quic.enabled = enabled;
        self
    }
    
    /// 设置最大连接数
    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.connection_manager.max_connections = max;
        self.config.protocol.websocket.max_connections = max;
        self.config.protocol.quic.max_connections = max;
        self
    }
    
    /// 启用认证
    pub fn enable_auth(mut self, enabled: bool) -> Self {
        self.config.auth.enabled = enabled;
        self
    }
    
    /// 设置认证方法
    pub fn auth_method(mut self, method: AuthMethod) -> Self {
        self.config.auth.method = method;
        self
    }
    
    /// 设置日志级别
    pub fn log_level(mut self, level: String) -> Self {
        self.config.logging.level = level;
        self
    }
    
    /// 配置WebSocket TLS
    pub fn websocket_tls(mut self, cert_path: String, key_path: String) -> Self {
        self.config.protocol.websocket.enable_tls = true;
        self.config.protocol.websocket.cert_path = Some(cert_path);
        self.config.protocol.websocket.key_path = Some(key_path);
        self
    }
    
    /// 配置QUIC TLS证书
    pub fn quic_tls(mut self, cert_path: String, key_path: String) -> Self {
        self.config.protocol.quic.cert_path = cert_path;
        self.config.protocol.quic.key_path = key_path;
        self
    }
    
    /// 配置QUIC ALPN协议
    pub fn quic_alpn(mut self, alpn_protocols: Vec<Vec<u8>>) -> Self {
        self.config.protocol.quic.alpn_protocols = alpn_protocols;
        self
    }
    
    /// 启用QUIC 0-RTT
    pub fn enable_quic_0rtt(mut self, enabled: bool) -> Self {
        self.config.protocol.quic.enable_0rtt = enabled;
        self
    }
    
    /// 构建配置
    pub fn build(self) -> ServerConfig {
        self.config
    }
}

impl Default for ServerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
} 