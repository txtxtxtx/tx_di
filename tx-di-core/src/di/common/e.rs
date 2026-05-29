use thiserror::Error;
use tx_error::AppError;
use crate::di::common::ApiRes;

/// 统一错误
///
/// 业务错误通过 `AppError`（归一化值类型）表达，
/// 库/框架错误保留原始变体以保留错误链。
#[derive(Error, Debug)]
pub enum IE {
    /// 业务错误码（统一入口）
    #[error("{0}")]
    AppError(#[from] AppError),

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
    /// 兜底：字符串错误
    #[error("{0}")]
    Other(String),
}

// ── IE → ApiRes ──────────────────────────────────────────────────────────────

impl From<IE> for ApiRes {
    fn from(err: IE) -> Self {
        match &err {
            IE::AppError(app_err) => {
                // 业务错误码：返回 domain:code 作为 msg，code 作为 code
                ApiRes::error(app_err.code(), app_err.message().to_string())
            }
            _ => {
                // 其他错误：code = -1，msg 为错误描述
                ApiRes::fail(err.to_string())
            }
        }
    }
}

// ── 便捷 From impls ─────────────────────────────────────────────────────────

impl From<anyhow::Error> for IE {
    fn from(err: anyhow::Error) -> Self {
        IE::WithContext {
            context: err.to_string(),
            source: err,
        }
    }
}

impl From<String> for IE {
    fn from(s: String) -> Self {
        IE::Other(s)
    }
}

impl From<&str> for IE {
    fn from(s: &str) -> Self {
        IE::Other(s.to_string())
    }
}

/// 封装 Result<T, IE>
///
/// IE：统一错误
pub type RIE<T> = Result<T, IE>;

/// 统一错误码（基于 tx_error::AppErrCode）
///
/// 用于 DI 框架自身的业务错误。
/// 外部业务模块应使用 `#[derive(CodeMsg)]` 自行定义。
#[derive(Debug, Copy, Clone, PartialEq, Eq, tx_error::CodeMsg)]
#[err("DI")]
pub enum DiErr {
    /// 组件注册表错误
    #[err(-1, "组件注册表错误")]
    RegistryError,
    /// async_init_fn 错误
    #[err(-2, "async_init_fn 错误")]
    AsyncInitError,
    /// 任务 panic
    #[err(-3, "任务 panic")]
    TaskPanic,
}
