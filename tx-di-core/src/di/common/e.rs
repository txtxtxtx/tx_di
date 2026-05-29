use std::fmt;
use tx_error::{AppErrCode, CodeMsg};
use crate::di::common::ApiRes;

/// 统一错误枚举。
///
/// 两条路径，职责清晰：
/// - **`Business`** — 类型化业务错误码，有 domain/code，可匹配、可序列化、前端可识别。
/// - **`Internal`** — 框架/IO/第三方库错误，带完整错误链，对外统一 500。
///
/// 插件直接用 `anyhow::Result` + `?` 即可自动走 `Internal`；
/// 如果需要类型化业务码，`#[derive(CodeMsg)]` 定义后 `?` 自动走 `Business`。
#[derive(Debug)]
pub enum IE {
    /// 业务错误码（类型化，有 domain/code，前端可匹配）
    Business(AppErrCode),

    /// 内部错误（IO / JSON / 配置 / 第三方库 / anyhow），带完整错误链
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

// ── From impls ────────────────────────────────────────────────────────────────

/// 业务错误码 → IE::Business
impl From<AppErrCode> for IE {
    fn from(code: AppErrCode) -> Self {
        IE::Business(code)
    }
}

/// anyhow::Error → IE::Internal
impl From<anyhow::Error> for IE {
    fn from(err: anyhow::Error) -> Self {
        IE::Internal(err)
    }
}

/// String → IE::Internal（兼容现有 `Err("msg".into())` 写法）
impl From<String> for IE {
    fn from(s: String) -> Self {
        IE::Internal(anyhow::anyhow!(s))
    }
}

/// &str → IE::Internal（兼容现有 `Err("msg")?` 写法）
impl From<&str> for IE {
    fn from(s: &str) -> Self {
        IE::Internal(anyhow::anyhow!(s.to_string()))
    }
}

/// std::io::Error → IE::Internal
impl From<std::io::Error> for IE {
    fn from(err: std::io::Error) -> Self {
        IE::Internal(err.into())
    }
}

/// serde_json::Error → IE::Internal
impl From<serde_json::Error> for IE {
    fn from(err: serde_json::Error) -> Self {
        IE::Internal(err.into())
    }
}

/// toml::de::Error → IE::Internal
impl From<toml::de::Error> for IE {
    fn from(err: toml::de::Error) -> Self {
        IE::Internal(err.into())
    }
}

// ── IE → ApiRes ──────────────────────────────────────────────────────────────

impl From<IE> for ApiRes {
    fn from(err: IE) -> Self {
        match &err {
            IE::Business(code) => {
                ApiRes::error(code.code, code.message.to_string())
            }
            IE::Internal(e) => {
                ApiRes::fail(e.to_string())
            }
        }
    }
}

/// 封装 `Result<T, IE>`
pub type RIE<T> = Result<T, IE>;

// ── 业务错误码定义 ──────────────────────────────────────────────────────────

/// DI 框架自身的业务错误码。
/// 外部业务模块应使用 `#[derive(CodeMsg)]` 自行定义。
#[derive(Debug, Copy, Clone, PartialEq, Eq, tx_error::CodeMsg)]
#[err("DI")]
#[ie(crate::IE)]
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

