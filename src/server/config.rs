//! Flare IM 服务端配置模块
//!
//! 提供服务端配置定义和默认值

use std::net::SocketAddr;
use std::str::FromStr;

/// 服务端配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// WebSocket配置
    pub websocket: WebSocketConfig,
    /// QUIC配置
    pub quic: QuicConfig,
    /// 连接管理器配置
    pub connection_manager: ConnectionManagerConfig,
    /// 认证配置
    pub auth: AuthConfig,
    /// 日志配置
    pub logging: LoggingConfig,
}

/// WebSocket配置
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// 绑定地址
    pub bind_addr: SocketAddr,
    /// 是否启用
    pub enabled: bool,
    /// 最大连接数
    pub max_connections: usize,
    /// 连接超时时间（毫秒）
    pub connection_timeout_ms: u64,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 是否启用TLS
    pub enable_tls: bool,
    /// TLS证书路径
    pub cert_path: Option<String>,
    /// TLS私钥路径
    pub key_path: Option<String>,
}

/// QUIC配置
#[derive(Debug, Clone)]
pub struct QuicConfig {
    /// 绑定地址
    pub bind_addr: SocketAddr,
    /// 是否启用
    pub enabled: bool,
    /// 最大连接数
    pub max_connections: usize,
    /// 连接超时时间（毫秒）
    pub connection_timeout_ms: u64,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// TLS证书路径
    pub cert_path: String,
    /// TLS私钥路径
    pub key_path: String,
    /// ALPN协议
    pub alpn_protocols: Vec<Vec<u8>>,
    /// 是否启用0-RTT
    pub enable_0rtt: bool,
}

/// 连接管理器配置
#[derive(Debug, Clone)]
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

/// 认证配置
#[derive(Debug, Clone)]
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

/// 认证方法
#[derive(Debug, Clone, PartialEq)]
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

/// 日志配置
#[derive(Debug, Clone)]
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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            websocket: WebSocketConfig::default(),
            quic: QuicConfig::default(),
            connection_manager: ConnectionManagerConfig::default(),
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from_str("0.0.0.0:8080").unwrap(),
            enabled: true,
            max_connections: 10000,
            connection_timeout_ms: 300000, // 5分钟
            heartbeat_interval_ms: 30000, // 30秒
            enable_tls: false,
            cert_path: None,
            key_path: None,
        }
    }
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from_str("0.0.0.0:4433").unwrap(),
            enabled: true,
            max_connections: 10000,
            connection_timeout_ms: 300000, // 5分钟
            heartbeat_interval_ms: 30000, // 30秒
            cert_path: "certs/server.crt".to_string(),
            key_path: "certs/server.key".to_string(),
            alpn_protocols: vec![b"flare-im".to_vec()],
            enable_0rtt: true,
        }
    }
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
    
    pub fn websocket_addr(mut self, addr: SocketAddr) -> Self {
        self.config.websocket.bind_addr = addr;
        self
    }
    
    pub fn quic_addr(mut self, addr: SocketAddr) -> Self {
        self.config.quic.bind_addr = addr;
        self
    }
    
    pub fn enable_websocket(mut self, enabled: bool) -> Self {
        self.config.websocket.enabled = enabled;
        self
    }
    
    pub fn enable_quic(mut self, enabled: bool) -> Self {
        self.config.quic.enabled = enabled;
        self
    }
    
    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.connection_manager.max_connections = max;
        self.config.websocket.max_connections = max;
        self.config.quic.max_connections = max;
        self
    }
    
    pub fn enable_auth(mut self, enabled: bool) -> Self {
        self.config.auth.enabled = enabled;
        self
    }
    
    pub fn auth_method(mut self, method: AuthMethod) -> Self {
        self.config.auth.method = method;
        self
    }
    
    pub fn log_level(mut self, level: String) -> Self {
        self.config.logging.level = level;
        self
    }
    
    pub fn build(self) -> ServerConfig {
        self.config
    }
}

impl Default for ServerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
} 