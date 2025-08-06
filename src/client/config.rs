//! 客户端配置模块
//!
//! 定义客户端配置结构和相关功能

use crate::common::TransportProtocol;
use serde::{Deserialize, Serialize};

/// 协议选择模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProtocolSelectionMode {
    /// 自动协议竞速模式（默认）
    AutoRacing,
    /// 指定协议模式
    Specific(TransportProtocol),
    /// 手动协议选择模式
    Manual,
}

/// 服务器地址配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAddresses {
    /// QUIC 服务器地址
    pub quic_url: Option<String>,
    /// WebSocket 服务器地址（普通TCP）
    pub websocket_url: Option<String>,
    /// WebSocket 服务器地址（TLS）
    pub websocket_tls_url: Option<String>,
}

impl ServerAddresses {
    /// 创建新的服务器地址配置
    pub fn new() -> Self {
        Self {
            quic_url: None,
            websocket_url: None,
            websocket_tls_url: None,
        }
    }

    /// 设置 QUIC 地址
    pub fn with_quic_url(mut self, url: String) -> Self {
        self.quic_url = Some(url);
        self
    }

    /// 设置 WebSocket 地址
    pub fn with_websocket_url(mut self, url: String) -> Self {
        self.websocket_url = Some(url);
        self
    }

    /// 设置 WebSocket TLS 地址
    pub fn with_websocket_tls_url(mut self, url: String) -> Self {
        self.websocket_tls_url = Some(url);
        self
    }

    /// 获取指定协议的地址
    pub fn get_protocol_url(&self, protocol: TransportProtocol) -> Option<&String> {
        match protocol {
            TransportProtocol::QUIC => self.quic_url.as_ref(),
            TransportProtocol::WebSocket => self.websocket_url.as_ref(),
            TransportProtocol::Auto => {
                // 自动选择：优先QUIC，回退WebSocket
                self.quic_url.as_ref().or_else(|| self.websocket_url.as_ref())
            }
        }
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        if self.quic_url.is_none() && self.websocket_url.is_none() && self.websocket_tls_url.is_none() {
            return Err("至少需要配置一个服务器地址".to_string());
        }
        Ok(())
    }
}

impl Default for ServerAddresses {
    fn default() -> Self {
        Self::new()
    }
}

/// 协议竞速配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolRacingConfig {
    /// 是否启用协议竞速
    pub enabled: bool,
    /// 竞速超时时间（毫秒）
    pub timeout_ms: u64,
    /// 测试消息数量
    pub test_message_count: u32,
    /// 协议权重配置
    pub protocol_weights: ProtocolWeights,
    /// 是否启用自动降级
    pub auto_fallback: bool,
    /// 竞速间隔（毫秒）
    pub racing_interval_ms: u64,
}

/// 协议权重配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolWeights {
    /// QUIC 权重
    pub quic_weight: f64,
    /// WebSocket 权重
    pub websocket_weight: f64,
}

impl ProtocolWeights {
    /// 创建默认权重配置
    pub fn new() -> Self {
        Self {
            quic_weight: 0.7,      // QUIC 默认权重更高
            websocket_weight: 0.3,
        }
    }

    /// 设置 QUIC 权重
    pub fn with_quic_weight(mut self, weight: f64) -> Self {
        self.quic_weight = weight.max(0.0).min(1.0);
        self
    }

    /// 设置 WebSocket 权重
    pub fn with_websocket_weight(mut self, weight: f64) -> Self {
        self.websocket_weight = weight.max(0.0).min(1.0);
        self
    }

    /// 获取协议权重
    pub fn get_weight(&self, protocol: TransportProtocol) -> f64 {
        match protocol {
            TransportProtocol::QUIC => self.quic_weight,
            TransportProtocol::WebSocket => self.websocket_weight,
            TransportProtocol::Auto => {
                // 自动选择：使用QUIC权重
                self.quic_weight
            }
        }
    }
}

impl Default for ProtocolWeights {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ProtocolRacingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_ms: 5000,
            test_message_count: 10,
            protocol_weights: ProtocolWeights::default(),
            auto_fallback: true,
            racing_interval_ms: 30000, // 30秒
        }
    }
}



/// 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// 用户ID
    pub user_id: String,
    /// 服务器地址配置
    pub server_addresses: ServerAddresses,
    /// 协议选择模式
    pub protocol_selection_mode: ProtocolSelectionMode,
    /// 协议竞速配置
    pub protocol_racing: ProtocolRacingConfig,
    /// 连接超时时间（毫秒）
    pub connection_timeout_ms: u64,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 最大重连次数
    pub max_reconnect_attempts: u32,
    /// 重连延迟（毫秒）
    pub reconnect_delay_ms: u64,
    /// 是否启用自动重连
    pub auto_reconnect_enabled: bool,
    /// 消息重试次数
    pub message_retry_count: u32,
    /// 消息重试延迟（毫秒）
    pub message_retry_delay_ms: u64,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 是否启用压缩
    pub compression_enabled: bool,
    /// 是否启用加密
    pub encryption_enabled: bool,
    /// 客户端证书路径（可选）
    pub client_cert: Option<String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            server_addresses: ServerAddresses::default(),
            protocol_selection_mode: ProtocolSelectionMode::AutoRacing,
            protocol_racing: ProtocolRacingConfig::default(),
            connection_timeout_ms: 5000,
            heartbeat_interval_ms: 30000,
            max_reconnect_attempts: 5,
            reconnect_delay_ms: 1000,
            auto_reconnect_enabled: true,
            message_retry_count: 3,
            message_retry_delay_ms: 1000,
            buffer_size: 8192,
            compression_enabled: false,
            encryption_enabled: false,
            client_cert: None,
        }
    }
}

impl ClientConfig {
    /// 创建新的配置
    pub fn new(user_id: String) -> Self {
        let mut config = Self::default();
        config.user_id = user_id;
        config
    }

    /// 设置服务器地址
    pub fn with_server_addresses(mut self, addresses: ServerAddresses) -> Self {
        self.server_addresses = addresses;
        self
    }

    /// 设置协议选择模式
    pub fn with_protocol_selection_mode(mut self, mode: ProtocolSelectionMode) -> Self {
        self.protocol_selection_mode = mode;
        self
    }

    /// 设置协议竞速配置
    pub fn with_protocol_racing(mut self, racing: ProtocolRacingConfig) -> Self {
        self.protocol_racing = racing;
        self
    }

    /// 设置连接超时
    pub fn with_connection_timeout(mut self, timeout_ms: u64) -> Self {
        self.connection_timeout_ms = timeout_ms;
        self
    }

    /// 设置心跳间隔
    pub fn with_heartbeat_interval(mut self, interval_ms: u64) -> Self {
        self.heartbeat_interval_ms = interval_ms;
        self
    }

    /// 设置最大重连次数
    pub fn with_max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnect_attempts = attempts;
        self
    }

    /// 设置重连延迟
    pub fn with_reconnect_delay(mut self, delay_ms: u64) -> Self {
        self.reconnect_delay_ms = delay_ms;
        self
    }

    /// 启用/禁用自动重连
    pub fn with_auto_reconnect(mut self, enabled: bool) -> Self {
        self.auto_reconnect_enabled = enabled;
        self
    }

    /// 设置消息重试配置
    pub fn with_message_retry(mut self, retry_count: u32, retry_delay_ms: u64) -> Self {
        self.message_retry_count = retry_count;
        self.message_retry_delay_ms = retry_delay_ms;
        self
    }

    /// 设置缓冲区大小
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// 启用/禁用压缩
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }

    /// 启用/禁用加密
    pub fn with_encryption(mut self, enabled: bool) -> Self {
        self.encryption_enabled = enabled;
        self
    }

    /// 设置客户端证书路径
    pub fn with_client_cert(mut self, cert_path: String) -> Self {
        self.client_cert = Some(cert_path);
        self
    }

    /// 启用 TLS（使用默认配置）
    pub fn with_tls(mut self, enabled: bool) -> Self {
        if enabled {
            // 如果启用TLS但没有指定证书，使用默认证书
            self.client_cert = Some("certs/server.crt".to_string());
        }
        self
    }

    /// 获取指定协议的服务器地址
    pub fn get_protocol_url(&self, protocol: TransportProtocol) -> Option<&String> {
        self.server_addresses.get_protocol_url(protocol)
    }

    /// 检查是否支持指定协议
    pub fn supports_protocol(&self, protocol: TransportProtocol) -> bool {
        self.server_addresses.get_protocol_url(protocol).is_some()
    }

    /// 获取支持的协议列表
    pub fn get_supported_protocols(&self) -> Vec<TransportProtocol> {
        let mut protocols = Vec::new();
        if self.server_addresses.quic_url.is_some() {
            protocols.push(TransportProtocol::QUIC);
        }
        if self.server_addresses.websocket_url.is_some() {
            protocols.push(TransportProtocol::WebSocket);
        }
        protocols
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        if self.user_id.is_empty() {
            return Err("用户ID不能为空".to_string());
        }

        // 验证服务器地址配置
        self.server_addresses.validate()?;

        // 验证协议选择模式
        match &self.protocol_selection_mode {
            ProtocolSelectionMode::Specific(protocol) => {
                if !self.supports_protocol(*protocol) {
                    return Err(format!("指定的协议 {:?} 没有对应的服务器地址", protocol));
                }
            }
            ProtocolSelectionMode::AutoRacing => {
                if self.get_supported_protocols().is_empty() {
                    return Err("协议竞速模式需要至少配置一个协议地址".to_string());
                }
            }
            ProtocolSelectionMode::Manual => {
                // 手动模式不需要特殊验证
            }
        }

        if self.connection_timeout_ms == 0 {
            return Err("连接超时时间必须大于0".to_string());
        }

        if self.heartbeat_interval_ms == 0 {
            return Err("心跳间隔必须大于0".to_string());
        }

        if self.buffer_size == 0 {
            return Err("缓冲区大小必须大于0".to_string());
        }

        Ok(())
    }
}

/// 客户端构建器
pub struct ClientConfigBuilder {
    config: ClientConfig,
}

impl ClientConfigBuilder {
    /// 创建新的构建器
    pub fn new(user_id: String) -> Self {
        Self {
            config: ClientConfig::new(user_id),
        }
    }

    /// 设置服务器地址
    pub fn server_addresses(mut self, addresses: ServerAddresses) -> Self {
        self.config.server_addresses = addresses;
        self
    }

    /// 设置协议选择模式
    pub fn protocol_selection_mode(mut self, mode: ProtocolSelectionMode) -> Self {
        self.config.protocol_selection_mode = mode;
        self
    }

    /// 设置协议竞速配置
    pub fn protocol_racing(mut self, racing: ProtocolRacingConfig) -> Self {
        self.config.protocol_racing = racing;
        self
    }

    /// 设置连接超时
    pub fn connection_timeout(mut self, timeout_ms: u64) -> Self {
        self.config.connection_timeout_ms = timeout_ms;
        self
    }

    /// 设置心跳间隔
    pub fn heartbeat_interval(mut self, interval_ms: u64) -> Self {
        self.config.heartbeat_interval_ms = interval_ms;
        self
    }

    /// 设置最大重连次数
    pub fn max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.config.max_reconnect_attempts = attempts;
        self
    }

    /// 设置重连延迟
    pub fn reconnect_delay(mut self, delay_ms: u64) -> Self {
        self.config.reconnect_delay_ms = delay_ms;
        self
    }

    /// 启用/禁用自动重连
    pub fn auto_reconnect(mut self, enabled: bool) -> Self {
        self.config.auto_reconnect_enabled = enabled;
        self
    }

    /// 设置消息重试配置
    pub fn message_retry(mut self, retry_count: u32, retry_delay_ms: u64) -> Self {
        self.config.message_retry_count = retry_count;
        self.config.message_retry_delay_ms = retry_delay_ms;
        self
    }

    /// 设置缓冲区大小
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    /// 启用/禁用压缩
    pub fn compression(mut self, enabled: bool) -> Self {
        self.config.compression_enabled = enabled;
        self
    }

    /// 启用/禁用加密
    pub fn encryption(mut self, enabled: bool) -> Self {
        self.config.encryption_enabled = enabled;
        self
    }

    /// 设置客户端证书路径
    pub fn client_cert(mut self, cert_path: String) -> Self {
        self.config.client_cert = Some(cert_path);
        self
    }

    /// 启用 TLS（使用默认配置）
    pub fn tls(mut self, enabled: bool) -> Self {
        if enabled {
            // 如果启用TLS但没有指定证书，使用默认证书
            self.config.client_cert = Some("certs/server.crt".to_string());
        }
        self
    }

    /// 构建配置
    pub fn build(self) -> Result<ClientConfig, String> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for ClientConfigBuilder {
    fn default() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }
}