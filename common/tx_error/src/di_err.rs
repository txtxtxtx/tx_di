//! DI 框架内部错误类型
//!
//! - `IE` — 内部错误枚举（业务错误码 / 内部异常）
//! - `RIE<T>` — `Result<T, IE>` 别名
//! - `DiErr` — DI 框架自身的业务错误码

use std::fmt;
use crate::{AppErrCode, CodeMsg};

/// 统一错误枚举（DI 框架内部使用）。
///
/// 两条路径：
/// - **`Business`** — 类型化业务错误码，有 domain/code，可匹配。
/// - **`Internal`** — 框架/IO/第三方库错误，带完整错误链。
#[derive(Debug)]
pub enum IE {
    /// 业务错误码（类型化）
    Business(AppErrCode),
    /// 内部错误（IO / JSON / 配置 / 第三方库 / anyhow）
    Internal(anyhow::Error),
}

impl fmt::Display for IE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IE::Business(code) => write!(f, "[{}:{}] {}", code.domain, code.code, code.message),
            IE::Internal(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for IE {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IE::Internal(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<AppErrCode> for IE {
    fn from(code: AppErrCode) -> Self { IE::Business(code) }
}

impl From<anyhow::Error> for IE {
    fn from(err: anyhow::Error) -> Self { IE::Internal(err) }
}

impl From<String> for IE {
    fn from(s: String) -> Self { IE::Internal(anyhow::anyhow!(s)) }
}

impl From<&str> for IE {
    fn from(s: &str) -> Self { IE::Internal(anyhow::anyhow!(s.to_string())) }
}

impl From<std::io::Error> for IE {
    fn from(err: std::io::Error) -> Self { IE::Internal(err.into()) }
}

impl From<serde_json::Error> for IE {
    fn from(err: serde_json::Error) -> Self { IE::Internal(err.into()) }
}

impl From<toml::de::Error> for IE {
    fn from(err: toml::de::Error) -> Self { IE::Internal(err.into()) }
}

/// `Result<T, IE>` 类型别名
pub type RIE<T> = Result<T, IE>;

/// DI 框架自身的业务错误码。
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("DI")]
#[ie(crate::IE)]
pub enum DiErr {
    #[err(-1, "组件注册表错误")]
    RegistryError,
    #[err(-2, "async_init_fn 错误")]
    AsyncInitError,
    #[err(-3, "任务 panic")]
    TaskPanic,
}
