//! Flare IM 客户端模块
//!
//! 提供高性能、跨平台的客户端实现，支持协议竞速和自动降级

pub mod config;
pub mod types;
pub mod protocol_racer;
pub mod connection_manager;
pub mod message_manager;
pub mod traits;
pub mod websocket_connector;
pub mod quic_connector;

use crate::common::{TransportProtocol, Result, ProtoMessage};
use crate::client::{
    config::{ClientConfig, ClientConfigBuilder, ProtocolSelectionMode, ServerAddresses},
    types::{ClientEventCallback},
    connection_manager::ConnectionManager,
    message_manager::MessageManager,
    protocol_racer::ProtocolRacer,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Flare IM 客户端
pub struct FlareIMClient {
    /// 配置
    config: ClientConfig,
    /// 连接管理器
    connection_manager: Arc<Mutex<ConnectionManager>>,
    /// 消息管理器
    message_manager: Arc<Mutex<MessageManager>>,
    /// 协议竞速器
    protocol_racer: Arc<Mutex<ProtocolRacer>>,
    /// 事件回调
    event_callback: Option<Arc<ClientEventCallback>>,
}

impl FlareIMClient {
    /// 创建新的客户端
    pub fn new(user_id: String) -> Result<Self> {
        let config = ClientConfigBuilder::new(user_id).build()?;
        Self::with_config(config)
    }

    /// 使用配置创建客户端
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(config.clone())));
        let message_manager = Arc::new(Mutex::new(MessageManager::new(
            config.clone(),
            Arc::clone(&connection_manager),
        )));

        Ok(Self {
            config,
            connection_manager,
            message_manager,
            protocol_racer: Arc::new(Mutex::new(ProtocolRacer::new())),
            event_callback: None,
        })
    }

    /// 设置事件回调
    pub fn with_event_callback(mut self, callback: Arc<ClientEventCallback>) -> Self {
        self.event_callback = Some(Arc::clone(&callback));
        self
    }

    /// 连接到服务器
    pub async fn connect(&mut self) -> Result<()> {
        info!("客户端开始连接");

        // 启动消息管理器
        {
            let mut message_manager = self.message_manager.lock().await;
            message_manager.start().await?;
        }

        // 连接到服务器
        {
            let mut connection_manager = self.connection_manager.lock().await;
            connection_manager.connect().await?;
        }

        info!("客户端连接成功");
        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        info!("客户端断开连接");

        // 停止消息管理器
        {
            let mut message_manager = self.message_manager.lock().await;
            message_manager.stop().await?;
        }

        // 断开连接
        let mut connection_manager = self.connection_manager.lock().await;
        connection_manager.disconnect().await?;

        info!("客户端已断开连接");
        Ok(())
    }

    /// 重连
    pub async fn reconnect(&mut self) -> Result<()> {
        info!("客户端开始重连");

        let mut connection_manager = self.connection_manager.lock().await;
        connection_manager.reconnect().await?;

        info!("客户端重连成功");
        Ok(())
    }

    /// 发送文本消息
    pub async fn send_text_message(&self, target_user_id: &str, content: &str) -> Result<SendResult> {
        let message_manager = self.message_manager.lock().await;
        message_manager.send_text_message(target_user_id, content).await
    }

    /// 发送二进制消息
    pub async fn send_binary_message(
        &self,
        target_user_id: &str,
        data: Vec<u8>,
        message_type: String,
    ) -> Result<SendResult> {
        let message_manager = self.message_manager.lock().await;
        message_manager.send_binary_message(target_user_id, data, message_type).await
    }

    /// 发送消息
    pub async fn send_message(&self, target_user_id: &str, message: ProtoMessage) -> Result<SendResult> {
        let message_manager = self.message_manager.lock().await;
        message_manager.send_message(target_user_id, message, types::MessagePriority::Normal).await
    }

    /// 接收消息
    pub async fn receive_message(&self, message: ProtoMessage) -> Result<()> {
        let message_manager = self.message_manager.lock().await;
        message_manager.receive_message(message).await
    }

    /// 获取连接状态
    pub async fn get_status(&self) -> connection_manager::ConnectionState {
        let connection_manager = self.connection_manager.lock().await;
        connection_manager.get_state().await
    }

    /// 检查是否已连接
    pub async fn is_connected(&self) -> bool {
        let connection_manager = self.connection_manager.lock().await;
        connection_manager.is_connected().await
    }

    /// 获取当前协议（已移除，使用协议竞速器获取）
    pub async fn get_current_protocol(&self) -> Option<TransportProtocol> {
        // 这个方法已被移除，使用协议竞速器获取当前协议
        None
    }

    /// 获取连接统计（已移除）
    pub async fn get_connection_stats(&self) -> Result<ConnectionStats> {
        // 这个方法已被移除，连接统计现在在连接管理器中
        Err("连接统计方法已移除".into())
    }

    /// 获取消息队列长度
    pub async fn get_message_queue_length(&self) -> usize {
        let message_manager = self.message_manager.lock().await;
        message_manager.get_queue_length().await
    }

    /// 清空消息队列
    pub async fn clear_message_queue(&self) {
        let message_manager = self.message_manager.lock().await;
        message_manager.clear_queue().await;
    }

    /// 切换协议
    pub async fn switch_protocol(&self, protocol: TransportProtocol) -> Result<()> {
        let mut protocol_racer = self.protocol_racer.lock().await;
        protocol_racer.switch_protocol(protocol).await
    }

    /// 获取协议性能指标
    pub async fn get_protocol_metrics(&self, protocol: TransportProtocol) -> Option<types::ProtocolMetrics> {
        let protocol_racer = self.protocol_racer.lock().await;
        protocol_racer.get_protocol_metrics(protocol).await
    }

    /// 获取所有协议指标
    pub async fn get_all_protocol_metrics(&self) -> std::collections::HashMap<TransportProtocol, types::ProtocolMetrics> {
        let protocol_racer = self.protocol_racer.lock().await;
        protocol_racer.get_all_metrics().await
    }

    /// 检查协议是否可用
    pub async fn is_protocol_available(&self, protocol: TransportProtocol) -> bool {
        let protocol_racer = self.protocol_racer.lock().await;
        protocol_racer.is_protocol_available(protocol).await
    }

    /// 获取最佳协议
    pub async fn get_best_protocol(&self) -> Option<TransportProtocol> {
        let protocol_racer = self.protocol_racer.lock().await;
        protocol_racer.get_best_protocol().await
    }

    /// 获取配置
    pub fn get_config(&self) -> &ClientConfig {
        &self.config
    }

    /// 获取连接管理器
    pub fn get_connection_manager(&self) -> Arc<Mutex<ConnectionManager>> {
        Arc::clone(&self.connection_manager)
    }

    /// 获取消息管理器
    pub fn get_message_manager(&self) -> Arc<Mutex<MessageManager>> {
        Arc::clone(&self.message_manager)
    }

    /// 获取协议竞速器
    pub fn get_protocol_racer(&self) -> Arc<Mutex<ProtocolRacer>> {
        Arc::clone(&self.protocol_racer)
    }
}

/// 客户端构建器
pub struct FlareIMClientBuilder {
    config_builder: ClientConfigBuilder,
}

impl FlareIMClientBuilder {
    /// 创建新的构建器
    pub fn new(user_id: String) -> Self {
        Self {
            config_builder: ClientConfigBuilder::new(user_id),
        }
    }

    /// 设置服务器地址
    pub fn server_addresses(mut self, addresses: ServerAddresses) -> Self {
        self.config_builder = self.config_builder.server_addresses(addresses);
        self
    }

    /// 设置协议选择模式
    pub fn protocol_selection_mode(mut self, mode: ProtocolSelectionMode) -> Self {
        self.config_builder = self.config_builder.protocol_selection_mode(mode);
        self
    }

    /// 设置协议竞速配置
    pub fn protocol_racing(mut self, racing: config::ProtocolRacingConfig) -> Self {
        self.config_builder = self.config_builder.protocol_racing(racing);
        self
    }

    /// 设置连接超时
    pub fn connection_timeout(mut self, timeout_ms: u64) -> Self {
        self.config_builder = self.config_builder.connection_timeout(timeout_ms);
        self
    }

    /// 设置心跳间隔
    pub fn heartbeat_interval(mut self, interval_ms: u64) -> Self {
        self.config_builder = self.config_builder.heartbeat_interval(interval_ms);
        self
    }

    /// 设置最大重连次数
    pub fn max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.config_builder = self.config_builder.max_reconnect_attempts(attempts);
        self
    }

    /// 设置重连延迟
    pub fn reconnect_delay(mut self, delay_ms: u64) -> Self {
        self.config_builder = self.config_builder.reconnect_delay(delay_ms);
        self
    }

    /// 启用/禁用自动重连
    pub fn auto_reconnect(mut self, enabled: bool) -> Self {
        self.config_builder = self.config_builder.auto_reconnect(enabled);
        self
    }

    /// 设置消息重试配置
    pub fn message_retry(mut self, retry_count: u32, retry_delay_ms: u64) -> Self {
        self.config_builder = self.config_builder.message_retry(retry_count, retry_delay_ms);
        self
    }

    /// 设置缓冲区大小
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config_builder = self.config_builder.buffer_size(size);
        self
    }

    /// 启用/禁用压缩
    pub fn compression(mut self, enabled: bool) -> Self {
        self.config_builder = self.config_builder.compression(enabled);
        self
    }

    /// 启用/禁用加密
    pub fn encryption(mut self, enabled: bool) -> Self {
        self.config_builder = self.config_builder.encryption(enabled);
        self
    }

    /// 启用 TLS（使用默认配置）
    pub fn tls(mut self, verify_server_cert: bool) -> Self {
        self.config_builder = self.config_builder.tls(verify_server_cert);
        self
    }

    /// 设置客户端证书路径
    pub fn client_cert(mut self, cert_path: String) -> Self {
        self.config_builder = self.config_builder.client_cert(cert_path);
        self
    }

    /// 构建客户端
    pub fn build(self) -> Result<FlareIMClient> {
        let config = self.config_builder.build()?;
        FlareIMClient::with_config(config)
    }
}

impl Default for FlareIMClientBuilder {
    fn default() -> Self {
        Self {
            config_builder: ClientConfigBuilder::default(),
        }
    }
}

// 重新导出常用类型
pub use types::{ClientStatus, SendResult, ConnectionStats, ClientEvent, MessagePriority, ProtocolMetrics};

 
 