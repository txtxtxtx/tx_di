use thiserror::Error;

/// 应用错误类型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("协议解析错误: {0}")]
    Protocol(String),

    #[error("CRC校验错误: expected {expected}, got {actual}")]
    CrcMismatch { expected: String, actual: String },

    #[error("长度校验错误: expected {expected}, got {actual}")]
    LengthMismatch { expected: usize, actual: usize },

    #[error("未知设备类型: {0}")]
    UnknownDeviceType(String),

    #[error("MQTT错误: {0}")]
    Mqtt(#[from] rumqttc::ClientError),

    #[error("JSON序列化错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("配置错误: {0}")]
    Config(#[from] anyhow::Error),

    #[error("其他错误: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// 应用结果类型
pub type AppResult<T> = Result<T, AppError>;