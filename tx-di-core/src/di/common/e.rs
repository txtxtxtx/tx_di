
use thiserror::Error;
use crate::di::common::ApiRes;

/// 统一错误
#[derive(Error, Debug)]
pub enum IE {
    /// IO错误
    #[error("IO错误: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
    /// JSON序列化/反序列化错误
    #[error("JSON处理错误: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },
    /// 配置文件错误
    #[error("配置文件错误: {source}")]
    Config {
        #[from]
        source: toml::de::Error,
    },
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        source: anyhow::Error,
    },
    /// 错误
    #[error("{0}")]
    Other(String),
}

// 为 AppError 实现 Into<ApiRes>
impl From<IE> for ApiRes {
    fn from(err: IE) -> Self {
        ApiRes::fail(err.to_string())
    }
}

// 为 anyhow::Error 实现 Into<AppError>
impl From<anyhow::Error> for IE {
    fn from(err: anyhow::Error) -> Self {
        IE::WithContext {
            context: err.to_string(),
            source: err,
        }
    }
}

// 为 String 实现 Into<AppError>
impl From<String> for IE {
    fn from(s: String) -> Self {
        IE::Other(s)
    }
}

// 为 &str 实现 Into<AppError>
impl From<&str> for IE {
    fn from(s: &str) -> Self {
        IE::Other(s.to_string())
    }
}

/// 封装 Result<T, IE>
///
/// IE：统一错误
pub type RIE<T> = Result<T, IE>;