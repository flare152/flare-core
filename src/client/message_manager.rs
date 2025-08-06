//! 消息管理器模块
//!
//! 管理消息发送、接收、重试等功能

use crate::common::{Result, ProtoMessage};
use crate::client::{
    config::ClientConfig,
    types::{SendResult, MessageQueueItem, MessagePriority, ClientEvent, ClientEventCallback},
    connection_manager::ConnectionManager,
};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{Duration, interval};
use tracing::{info, warn, error, debug};

/// 消息管理器
pub struct MessageManager {
    /// 配置
    config: ClientConfig,
    /// 连接管理器
    connection_manager: Arc<Mutex<ConnectionManager>>,
    /// 消息队列
    message_queue: Arc<RwLock<VecDeque<MessageQueueItem>>>,
    /// 事件回调
    event_callback: Option<Arc<ClientEventCallback>>,
    /// 消息处理任务句柄
    message_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 是否正在运行
    is_running: Arc<RwLock<bool>>,
}

impl MessageManager {
    /// 创建新的消息管理器
    pub fn new(config: ClientConfig, connection_manager: Arc<Mutex<ConnectionManager>>) -> Self {
        Self {
            config,
            connection_manager,
            message_queue: Arc::new(RwLock::new(VecDeque::new())),
            event_callback: None,
            message_task: Arc::new(Mutex::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// 设置事件回调
    pub fn with_event_callback(mut self, callback: Arc<ClientEventCallback>) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// 启动消息管理器
    pub async fn start(&mut self) -> Result<()> {
        info!("启动消息管理器");

        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                warn!("消息管理器已在运行");
                return Ok(());
            }
            *is_running = true;
        }

        // 启动消息处理任务
        self.start_message_task().await;

        Ok(())
    }

    /// 停止消息管理器
    pub async fn stop(&mut self) -> Result<()> {
        info!("停止消息管理器");

        {
            let mut is_running = self.is_running.write().await;
            *is_running = false;
        }

        // 停止消息处理任务
        {
            let mut message_task = self.message_task.lock().await;
            if let Some(task) = message_task.take() {
                task.abort();
            }
        }

        Ok(())
    }

    /// 发送文本消息
    pub async fn send_text_message(&self, target_user_id: &str, content: &str) -> Result<SendResult> {
        let message = ProtoMessage::new(
            uuid::Uuid::new_v4().to_string(),
            "text".to_string(),
            content.as_bytes().to_vec(),
        );

        self.send_message(target_user_id, message, MessagePriority::Normal).await
    }

    /// 发送二进制消息
    pub async fn send_binary_message(
        &self,
        target_user_id: &str,
        data: Vec<u8>,
        message_type: String,
    ) -> Result<SendResult> {
        let message = ProtoMessage::new(
            uuid::Uuid::new_v4().to_string(),
            message_type,
            data,
        );

        self.send_message(target_user_id, message, MessagePriority::Normal).await
    }

    /// 发送消息
    pub async fn send_message(
        &self,
        target_user_id: &str,
        message: ProtoMessage,
        priority: MessagePriority,
    ) -> Result<SendResult> {
        let message_id = message.id.clone();

        // 检查连接状态
        let connection_manager = self.connection_manager.lock().await;
        if !connection_manager.is_connected().await {
            return Ok(SendResult::failure(
                message_id,
                "客户端未连接".to_string(),
            ));
        }
        drop(connection_manager);

        // 创建消息队列项
        let queue_item = MessageQueueItem::new(
            message.clone(),
            target_user_id.to_string(),
            message.message_type.clone(),
            self.config.message_retry_count,
            priority,
        );

        // 添加到消息队列
        {
            let mut queue = self.message_queue.write().await;
            queue.push_back(queue_item);
        }

        // 尝试立即发送
        let result = self.try_send_message(&message, target_user_id).await;

        match result {
            Ok(_) => {
                // 发送成功，从队列中移除
                self.remove_message_from_queue(&message_id).await;
                
                // 统计已移除，直接触发事件

                // 触发消息发送事件
                self.trigger_event(ClientEvent::MessageSent(message_id.clone())).await;

                Ok(SendResult::success(message_id))
            }
            Err(e) => {
                // 发送失败，保留在队列中等待重试
                warn!("消息发送失败，已加入重试队列: {}", e);
                Ok(SendResult::failure(message_id, e.to_string()))
            }
        }
    }

    /// 尝试发送消息
    async fn try_send_message(&self, message: &ProtoMessage, _target_user_id: &str) -> Result<()> {
        // 获取连接管理器
        let connection_manager = self.connection_manager.lock().await;
        
        // 检查连接状态
        if !connection_manager.is_connected().await {
            return Err("客户端未连接".into());
        }
        
        // 通过连接管理器发送消息
        connection_manager.send_message(message.clone()).await
    }

    /// 从队列中移除消息
    async fn remove_message_from_queue(&self, message_id: &str) {
        let mut queue = self.message_queue.write().await;
        queue.retain(|item| item.message.id != message_id);
    }

    /// 启动消息处理任务
    async fn start_message_task(&self) {
        let config = self.config.clone();
        let connection_manager = Arc::clone(&self.connection_manager);
        let message_queue = Arc::clone(&self.message_queue);
        let is_running = Arc::clone(&self.is_running);
        let event_callback = self.event_callback.clone();

        let task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(1000)); // 每秒处理一次

            loop {
                interval.tick().await;

                // 检查是否还在运行
                let running = is_running.read().await;
                if !*running {
                    break;
                }
                drop(running);

                // 处理消息队列
                Self::process_message_queue(
                    &config,
                    &connection_manager,
                    &message_queue,
                    &event_callback,
                ).await;
            }
        });

        {
            let mut message_task = self.message_task.lock().await;
            *message_task = Some(task);
        }
    }

    /// 处理消息队列
    async fn process_message_queue(
        config: &ClientConfig,
        connection_manager: &Arc<Mutex<ConnectionManager>>,
        message_queue: &Arc<RwLock<VecDeque<MessageQueueItem>>>,
        event_callback: &Option<Arc<ClientEventCallback>>,
    ) {
        let mut queue = message_queue.write().await;
        let mut processed_items = Vec::new();

        // 按优先级排序
        queue.make_contiguous().sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));

        for item in queue.iter_mut() {
            // 检查是否可以重试
            if !item.can_retry() {
                processed_items.push(item.message.id.clone());
                
                // 触发消息失败事件
                if let Some(callback) = event_callback {
                    callback(ClientEvent::MessageFailed(
                        item.message.id.clone(),
                        "达到最大重试次数".to_string(),
                    ));
                }
                continue;
            }

            // 尝试发送消息
            let conn_manager = connection_manager.lock().await;
            if !conn_manager.is_connected().await {
                break;
            }
            
            // 发送消息
            match conn_manager.send_message(item.message.clone()).await {
                Ok(_) => {
                    processed_items.push(item.message.id.clone());
                    
                    if let Some(callback) = event_callback {
                        callback(ClientEvent::MessageSent(item.message.id.clone()));
                    }
                }
                Err(_) => {
                    item.increment_retry();
                    tokio::time::sleep(Duration::from_millis(config.message_retry_delay_ms)).await;
                }
            }
            drop(conn_manager);
        }

        // 移除已处理的消息
        for message_id in processed_items {
            queue.retain(|item| item.message.id != message_id);
        }
    }

    /// 接收消息
    pub async fn receive_message(&self, message: ProtoMessage) -> Result<()> {
        // 触发消息接收事件
        self.trigger_event(ClientEvent::MessageReceived(message)).await;

        Ok(())
    }

    /// 获取队列长度
    pub async fn get_queue_length(&self) -> usize {
        let queue = self.message_queue.read().await;
        queue.len()
    }

    /// 清空消息队列
    pub async fn clear_queue(&self) {
        let mut queue = self.message_queue.write().await;
        queue.clear();
    }

    /// 触发事件
    async fn trigger_event(&self, event: ClientEvent) {
        if let Some(callback) = &self.event_callback {
            callback(event);
        }
    }
} 