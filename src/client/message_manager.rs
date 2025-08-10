//! 消息管理器模块
//!
//! 管理消息发送、接收、重试等功能

use crate::common::{
    protocol::UnifiedProtocolMessage,
    FlareError,
    callback::EventCallback,
};
use crate::client::{
    config::ClientConfig,
    connection_manager::ConnectionManager,
    types::{MessageQueueItem, MessagePriority, SendResult},
};
use tracing::{info, warn, error};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

/// 消息管理器
/// 
/// 负责管理消息的发送、接收、重试等逻辑
pub struct MessageManager {
    /// 消息队列
    message_queue: Arc<Mutex<VecDeque<MessageQueueItem>>>,
    /// 配置
    config: ClientConfig,
    /// 事件回调
    event_callback: Option<Arc<dyn EventCallback + Send + Sync>>,
    /// 连接管理器引用
    connection_manager: Arc<Mutex<ConnectionManager>>,
}

impl MessageManager {
    /// 创建新的消息管理器
    pub fn new(
        config: ClientConfig,
        connection_manager: Arc<Mutex<ConnectionManager>>,
    ) -> Self {
        Self {
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            config,
            event_callback: None,
            connection_manager,
        }
    }

    /// 设置事件回调
    pub fn with_event_callback(mut self, callback: Arc<dyn EventCallback + Send + Sync>) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// 发送消息
    pub async fn send_message(
        &self,
        message: UnifiedProtocolMessage,
        priority: MessagePriority,
        session_id: String,
    ) -> Result<SendResult, FlareError> {
        let message_id = uuid::Uuid::new_v4().to_string();
        
        info!("发送消息: ID={}, 优先级={:?}", message_id, priority);
        
        // 创建消息队列项
        let queue_item = MessageQueueItem::new(
            message_id.clone(),
            message,
            session_id,
            "message".to_string(),
            self.config.message_retry_count,
            priority.clone(),
        );
        
        // 根据优先级插入队列
        let mut queue = self.message_queue.lock().await;
        match priority {
            MessagePriority::High => {
                queue.push_front(queue_item);
            }
            MessagePriority::Normal => {
                queue.push_back(queue_item);
            }
            MessagePriority::Low => {
                queue.push_back(queue_item);
            }
            MessagePriority::Urgent => {
                queue.push_front(queue_item);
            }
        }
        
        // 启动处理任务
        let queue_clone = self.message_queue.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            Self::process_queue_worker(queue_clone, config).await;
        });
        
        Ok(SendResult::success(message_id))
    }

    /// 处理消息队列
    async fn process_queue(&self) -> Result<(), FlareError> {
        let mut queue = self.message_queue.lock().await;
        
        while let Some(mut item) = queue.pop_front() {
            // 检查重试次数
            if item.retry_count >= item.max_retries {
                error!("消息发送失败，已达到最大重试次数: {:?}", item);
                continue;
            }
            
            // 尝试发送消息
            match self.send_single_message(&item).await {
                Ok(_) => {
                    info!("消息发送成功: {:?}", item);
                }
                Err(e) => {
                    warn!("消息发送失败，将重试: {:?}, 错误: {:?}", item, e);
                    item.increment_retry();
                    queue.push_back(item);
                }
            }
        }
        
        Ok(())
    }

    /// 处理消息队列的静态工作方法
    async fn process_queue_worker(
        queue: Arc<Mutex<VecDeque<MessageQueueItem>>>,
        config: ClientConfig,
    ) {
        loop {
            // 等待一段时间再处理
            sleep(Duration::from_millis(100)).await;
            
            let mut queue_guard = queue.lock().await;
            if queue_guard.is_empty() {
                break;
            }
            
            // 处理队列中的消息
            while let Some(item) = queue_guard.pop_front() {
                if item.retry_count >= item.max_retries {
                    continue;
                }
                
                // 这里应该实现实际的发送逻辑
                // 暂时跳过，避免循环依赖
                break;
            }
        }
    }

    /// 发送单个消息
    async fn send_single_message(&self, item: &MessageQueueItem) -> Result<(), FlareError> {
        let conn_manager = self.connection_manager.lock().await;
        
        // 检查连接状态
        if !conn_manager.is_connected().await {
            return Err(FlareError::ConnectionFailed("连接未建立".to_string()));
        }
        
        // 发送消息
        conn_manager.send_message(item.message.clone()).await?;
        
        Ok(())
    }

    /// 接收消息
    pub async fn receive_message(&self, message: UnifiedProtocolMessage) -> Result<(), FlareError> {
        info!("接收消息: {:?}", message);
        
        if let Some(callback) = &self.event_callback {
            // 处理消息回调
            // TODO: 实现具体的消息处理逻辑
        }
        
        Ok(())
    }

    /// 获取队列状态
    pub async fn get_queue_status(&self) -> (usize, usize) {
        let queue = self.message_queue.lock().await;
        let total = queue.len();
        let pending = queue.iter().filter(|item| item.retry_count > 0).count();
        (total, pending)
    }

    /// 清空消息队列
    pub async fn clear_queue(&self) {
        let mut queue = self.message_queue.lock().await;
        queue.clear();
        info!("消息队列已清空");
    }
} 