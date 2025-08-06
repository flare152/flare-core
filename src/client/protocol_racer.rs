//! 协议竞速模块
//!
//! 实现协议竞速功能，优先使用 QUIC，自动降级到 WebSocket

use crate::common::{TransportProtocol, Result, ProtoMessage};
use crate::client::{
    config::{ClientConfig, ProtocolRacingConfig, ProtocolWeights},
    types::{ProtocolMetrics, ClientEvent},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use tracing::{info, warn, error, debug};

/// 协议竞速器
pub struct ProtocolRacer {
    /// 当前协议
    current_protocol: Arc<RwLock<TransportProtocol>>,
    /// 协议性能指标
    metrics: Arc<RwLock<HashMap<TransportProtocol, ProtocolMetrics>>>,
    /// 是否启用降级
    fallback_enabled: bool,
    /// 竞速超时时间
    race_timeout_ms: u64,
    /// 测试消息数量
    test_message_count: u32,
    /// 协议权重配置
    protocol_weights: ProtocolWeights,
    /// 事件回调
    event_callback: Option<Arc<dyn Fn(ClientEvent) + Send + Sync>>,
}

impl ProtocolRacer {
    /// 创建新的协议竞速器
    pub fn new() -> Self {
        Self {
            current_protocol: Arc::new(RwLock::new(TransportProtocol::QUIC)),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            fallback_enabled: true,
            race_timeout_ms: 5000,
            test_message_count: 10,
            protocol_weights: ProtocolWeights::default(),
            event_callback: None,
        }
    }

    /// 设置事件回调
    pub fn with_event_callback(mut self, callback: Arc<dyn Fn(ClientEvent) + Send + Sync>) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// 设置竞速超时时间
    pub fn with_race_timeout(mut self, timeout_ms: u64) -> Self {
        self.race_timeout_ms = timeout_ms;
        self
    }

    /// 设置测试消息数量
    pub fn with_test_message_count(mut self, count: u32) -> Self {
        self.test_message_count = count;
        self
    }

    /// 启用/禁用降级
    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.fallback_enabled = enabled;
        self
    }

    /// 设置协议权重
    pub fn with_protocol_weights(mut self, weights: ProtocolWeights) -> Self {
        self.protocol_weights = weights;
        self
    }

    /// 执行协议竞速
    pub async fn race_protocols(&mut self, config: &ClientConfig) -> Result<TransportProtocol> {
        info!("开始协议竞速测试");

        // 获取支持的协议列表
        let supported_protocols = config.get_supported_protocols();
        if supported_protocols.is_empty() {
            return Err("没有支持的协议".into());
        }

        let mut best_protocol = supported_protocols[0];
        let mut best_score = 0.0;

        // 测试每个支持的协议
        for protocol in &supported_protocols {
            if let Ok(metrics) = self.test_protocol(config, *protocol).await {
                let score = self.calculate_protocol_score(&metrics, &self.protocol_weights);
                info!("{:?} 协议得分: {:.2}", protocol, score);
                
                if score > best_score {
                    best_score = score;
                    best_protocol = *protocol;
                }
            }
        }

        // 如果没有成功的协议且启用降级，尝试其他协议
        if best_score == 0.0 && self.fallback_enabled {
            let all_protocols = vec![TransportProtocol::QUIC, TransportProtocol::WebSocket];
            for protocol in all_protocols {
                if !supported_protocols.contains(&protocol) {
                    if let Ok(metrics) = self.test_protocol(config, protocol).await {
                        let score = self.calculate_protocol_score(&metrics, &self.protocol_weights);
                        info!("降级到 {:?} 协议得分: {:.2}", protocol, score);
                        
                        if score > best_score {
                            best_score = score;
                            best_protocol = protocol;
                        }
                    }
                }
            }
        }

        // 更新当前协议
        {
            let mut current = self.current_protocol.write().await;
            *current = best_protocol;
        }

        // 触发协议切换事件
        if let Some(callback) = &self.event_callback {
            callback(ClientEvent::ProtocolSwitched(best_protocol));
        }

        info!("协议竞速完成，选择协议: {:?}", best_protocol);
        Ok(best_protocol)
    }

    /// 测试单个协议
    async fn test_protocol(&mut self, config: &ClientConfig, protocol: TransportProtocol) -> Result<ProtocolMetrics> {
        let start_time = Instant::now();
        let mut metrics = ProtocolMetrics::new(protocol);
        let mut successful_attempts = 0u64;
        let mut total_attempts = 0u64;

        info!("测试协议: {:?}", protocol);

        // 获取协议对应的服务器地址
        let server_url = if let Some(url) = config.get_protocol_url(protocol) {
            url.clone()
        } else {
            return Err(format!("协议 {:?} 没有对应的服务器地址", protocol).into());
        };

        // 测试连接延迟
        let connection_start = Instant::now();
        if let Err(e) = self.test_connection(&server_url, protocol).await {
            warn!("协议 {:?} 连接测试失败: {}", protocol, e);
            return Err(e);
        }
        metrics.connection_latency_ms = connection_start.elapsed().as_millis() as u64;

        // 测试消息延迟和吞吐量
        let message_start = Instant::now();
        for i in 0..self.test_message_count {
            total_attempts += 1;
            
            let test_message = ProtoMessage::new(
                uuid::Uuid::new_v4().to_string(),
                "ping".to_string(),
                format!("test_message_{}", i).into_bytes(),
            );

            let msg_start = Instant::now();
            if let Ok(_) = self.test_message_send(&server_url, protocol, &test_message).await {
                successful_attempts += 1;
                let latency = msg_start.elapsed().as_millis() as u64;
                metrics.message_latency_ms = (metrics.message_latency_ms + latency) / 2; // 平均延迟
            }
        }

        let total_time_ms = message_start.elapsed().as_millis() as u64;
        metrics.calculate_throughput(successful_attempts, total_time_ms);
        metrics.calculate_success_rate(total_attempts, successful_attempts);

        // 保存指标
        {
            let mut metrics_map = self.metrics.write().await;
            metrics_map.insert(protocol, metrics.clone());
        }

        info!("协议 {:?} 测试完成 - 延迟: {}ms, 吞吐量: {:.2} msg/s, 成功率: {:.2}%", 
              protocol, metrics.message_latency_ms, metrics.throughput_msgs_per_sec, 
              metrics.success_rate * 100.0);

        Ok(metrics)
    }

    /// 测试连接
    async fn test_connection(&self, server_url: &str, protocol: TransportProtocol) -> Result<()> {
        let timeout = Duration::from_millis(self.race_timeout_ms);
        
        match protocol {
            TransportProtocol::QUIC => {
                // 这里应该实现 QUIC 连接测试
                // 为了演示，我们模拟连接过程
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(())
            }
            TransportProtocol::WebSocket => {
                // 这里应该实现 WebSocket 连接测试
                // 为了演示，我们模拟连接过程
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok(())
            }
            TransportProtocol::Auto => {
                // 自动选择：优先QUIC
                // 直接模拟QUIC连接测试
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(())
            }
        }
    }

    /// 测试消息发送
    async fn test_message_send(&self, server_url: &str, protocol: TransportProtocol, message: &ProtoMessage) -> Result<()> {
        match protocol {
            TransportProtocol::QUIC => {
                // 这里应该实现 QUIC 消息发送测试
                // 为了演示，我们模拟发送过程
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(())
            }
            TransportProtocol::WebSocket => {
                // 这里应该实现 WebSocket 消息发送测试
                // 为了演示，我们模拟发送过程
                tokio::time::sleep(Duration::from_millis(20)).await;
                Ok(())
            }
            TransportProtocol::Auto => {
                // 自动选择：优先QUIC
                // 直接模拟QUIC消息发送测试
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(())
            }
        }
    }

    /// 计算协议得分
    fn calculate_protocol_score(&self, metrics: &ProtocolMetrics, weights: &ProtocolWeights) -> f64 {
        // 基础评分算法
        let latency_score = if metrics.connection_latency_ms > 0 {
            1000.0 / metrics.connection_latency_ms as f64
        } else {
            0.0
        };

        let message_latency_score = if metrics.message_latency_ms > 0 {
            100.0 / metrics.message_latency_ms as f64
        } else {
            0.0
        };

        let throughput_score = metrics.throughput_msgs_per_sec / 100.0; // 标准化
        let success_score = metrics.success_rate;

        // 应用协议权重
        let protocol_weight = weights.get_weight(metrics.protocol);
        
        // 加权平均
        (latency_score * 0.3 + message_latency_score * 0.3 + throughput_score * 0.2 + success_score * 0.2) * protocol_weight
    }

    /// 获取当前协议
    pub async fn get_current_protocol(&self) -> TransportProtocol {
        let protocol = self.current_protocol.read().await;
        *protocol
    }

    /// 切换协议
    pub async fn switch_protocol(&mut self, protocol: TransportProtocol) -> Result<()> {
        info!("切换协议到: {:?}", protocol);
        
        {
            let mut current = self.current_protocol.write().await;
            *current = protocol;
        }

        // 触发协议切换事件
        if let Some(callback) = &self.event_callback {
            callback(ClientEvent::ProtocolSwitched(protocol));
        }

        Ok(())
    }

    /// 获取协议性能指标
    pub async fn get_protocol_metrics(&self, protocol: TransportProtocol) -> Option<ProtocolMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(&protocol).cloned()
    }

    /// 获取所有协议指标
    pub async fn get_all_metrics(&self) -> HashMap<TransportProtocol, ProtocolMetrics> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// 检查协议是否可用
    pub async fn is_protocol_available(&self, protocol: TransportProtocol) -> bool {
        if let Some(metrics) = self.get_protocol_metrics(protocol).await {
            metrics.success_rate > 0.5 // 成功率大于50%认为可用
        } else {
            false
        }
    }

    /// 获取最佳协议
    pub async fn get_best_protocol(&self) -> Option<TransportProtocol> {
        let metrics = self.metrics.read().await;
        let mut best_protocol = None;
        let mut best_score = 0.0;

        for (protocol, metric) in metrics.iter() {
            let score = self.calculate_protocol_score(metric, &self.protocol_weights);
            if score > best_score {
                best_score = score;
                best_protocol = Some(*protocol);
            }
        }

        best_protocol
    }

    /// 更新配置
    pub fn update_config(&mut self, racing_config: &ProtocolRacingConfig) {
        self.race_timeout_ms = racing_config.timeout_ms;
        self.test_message_count = racing_config.test_message_count;
        self.fallback_enabled = racing_config.auto_fallback;
        self.protocol_weights = racing_config.protocol_weights.clone();
    }
}

impl Default for ProtocolRacer {
    fn default() -> Self {
        Self::new()
    }
} 