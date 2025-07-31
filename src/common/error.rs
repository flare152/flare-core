use thiserror::Error;

/// Flare IM 错误类型
#[derive(Error, Debug)]
pub enum FlareError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    #[error("认证失败: {0}")]
    AuthenticationFailed(String),

    #[error("认证错误: {0}")]
    AuthenticationError(String),

    #[error("协议错误: {0}")]
    ProtocolError(String),

    #[error("消息发送失败: {0}")]
    MessageSendFailed(String),

    #[error("协议切换失败: {0}")]
    ProtocolSwitchFailed(String),

    #[error("无效配置: {0}")]
    InvalidConfiguration(String),

    #[error("网络错误: {0}")]
    NetworkError(#[from] std::io::Error),

    #[error("超时错误: {0}")]
    TimeoutError(String),

    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("内部错误: {0}")]
    InternalError(String),

    #[error("无可用传输协议")]
    NoTransportAvailable,

    #[error("用户不存在: {user_id}")]
    UserNotFound { user_id: String },

    #[error("消息内容为空")]
    EmptyContent,

    #[error("不支持的消息类型")]
    UnsupportedMessageType,

    #[error("资源耗尽: {0}")]
    ResourceExhausted(String),
}

/// 客户端错误类型
pub type ClientError = FlareError;

/// 服务端错误类型
pub type ServerError = FlareError;

/// 结果类型别名
pub type Result<T> = std::result::Result<T, FlareError>; 