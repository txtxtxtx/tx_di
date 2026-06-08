//! tx_error 核心错误类型
//!
//! 统一错误枚举 `AppError`，三种形态：
//! - `ErrCode` — 纯业务错误码，零堆分配
//! - `WithContext` — 带动态上下文
//! - `Internal` — 框架/IO/第三方库错误，带完整错误链

use std::fmt;
use crate::code::AppErrCode;
use crate::code::CodeMsg as CodeMsgTrait; // trait
use crate::CodeMsg; // derive 宏

/// 统一错误类型。
///
/// 所有错误统一走这一种类型，`Result<T, AppError>` 贯穿全栈。
/// 不实现 `Clone`（因为 `anyhow::Error` 不是 `Clone`）。
#[derive(Debug)]
pub enum AppError {
    /// 业务错误码（归一化值类型，零堆分配）
    ErrCode {
        domain: &'static str,
        code: i32,
        message: &'static str,
    },
    /// 带上下文的业务错误（错误码 + 动态信息）
    WithContext {
        domain: &'static str,
        code: i32,
        message: &'static str,
        context: String,
    },
    /// 内部错误（IO / JSON / 配置 / 第三方库 / anyhow）
    /// 插件直接用 `anyhow::Result` + `?` 即可自动走这条路径。
    Internal(anyhow::Error),
}

// ── 构造函数 ────────────────────────────────────────────────

impl AppError {
    /// 从业务错误码构造
    #[inline]
    pub fn from_code<C: CodeMsgTrait>(code: C) -> Self {
        let c = code.err_code();
        Self::ErrCode { domain: c.domain, code: c.code, message: c.message }
    }

    /// 带上下文构造
    #[inline]
    pub fn with_context<C: CodeMsgTrait>(code: C, context: impl Into<String>) -> Self {
        let c = code.err_code();
        Self::WithContext { domain: c.domain, code: c.code, message: c.message, context: context.into() }
    }

    /// 从 anyhow 构造内部错误
    #[inline]
    pub fn from_anyhow(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}

// ── 访问器 ──────────────────────────────────────────────────

impl AppError {
    #[inline]
    pub fn domain(&self) -> &'static str {
        match self {
            Self::ErrCode { domain, .. } | Self::WithContext { domain, .. } => domain,
            Self::Internal(_) => "SYS",
        }
    }

    #[inline]
    pub fn code(&self) -> i32 {
        match self {
            Self::ErrCode { code, .. } | Self::WithContext { code, .. } => *code,
            Self::Internal(_) => 90000,
        }
    }

    /// 获取错误消息。
    ///
    /// - 对 `ErrCode` / `WithContext` 返回静态消息字符串
    /// - 对 `Internal` 返回 `"Internal error"`（不泄漏内存）
    ///
    /// 如需包含 Internal 错误的完整信息，请使用 [`full_message()`](Self::full_message)。
    #[inline]
    pub fn message(&self) -> &str {
        match self {
            Self::ErrCode { message, .. } | Self::WithContext { message, .. } => message,
            Self::Internal(_) => "Internal error",
        }
    }

    /// 获取上下文（如果有）
    #[inline]
    pub fn context(&self) -> Option<&str> {
        match self {
            Self::WithContext { context, .. } => Some(context),
            _ => None,
        }
    }

    /// 获取内部错误（如果是 Internal）
    #[inline]
    pub fn internal(&self) -> Option<&anyhow::Error> {
        match self {
            Self::Internal(e) => Some(e),
            _ => None,
        }
    }

    /// 完整消息（静态消息 + 上下文，或内部错误链）
    pub fn full_message(&self) -> String {
        match self {
            Self::ErrCode { message, .. } => message.to_string(),
            Self::WithContext { message, context, .. } => format!("{message}: {context}"),
            Self::Internal(e) => format!("{e}"),
        }
    }

    /// 获取 `AppErrCode`（丢弃上下文和内部错误细节）
    pub fn err_code(&self) -> AppErrCode {
        match self {
            Self::ErrCode { domain, code, message } => AppErrCode::new(*domain, *code, *message),
            Self::WithContext { domain, code, message, .. } => AppErrCode::new(*domain, *code, *message),
            Self::Internal(_) => AppErrCode::new("SYS", 90000, "Internal error"),
        }
    }

    /// 是否为同一类错误
    pub fn is_same_kind(&self, other: &Self) -> bool {
        self.domain() == other.domain() && self.code() == other.code()
    }

    /// 是否为内部错误
    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Internal(_))
    }
}

// ── PartialEq: 只比较 domain + code ────────────────────────
impl PartialEq for AppError {
    fn eq(&self, other: &Self) -> bool {
        self.domain() == other.domain() && self.code() == other.code()
    }
}
impl Eq for AppError {}

// ── Display ─────────────────────────────────────────────────
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ErrCode { domain, code, message } => write!(f, "[{domain}:{code}] {message}"),
            Self::WithContext { domain, code, message, context } => write!(f, "[{domain}:{code}] {message}: {context}"),
            Self::Internal(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self { Self::Internal(e) => Some(e.as_ref()), _ => None }
    }
}

// ── From 实现 ───────────────────────────────────────────────

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}


impl From<String> for AppError {
    fn from(s: String) -> Self { Self::Internal(anyhow::anyhow!(s)) }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self { Self::Internal(anyhow::anyhow!(s.to_string())) }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self { Self::Internal(err.into()) }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self { Self::Internal(err.into()) }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self { Self::Internal(err.into()) }
}

// ── 类型别名 ────────────────────────────────────────────────

/// 统一 Result 类型
pub type AppResult<T> = Result<T, AppError>;

// ── DI 框架业务错误码 ──────────────────────────────────────

/// DI 框架自身的业务错误码。
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("DI")]
pub enum DiErr {
    #[err(-1, "组件注册表错误")]
    RegistryError,
    #[err(-2, "async_init_fn 错误")]
    AsyncInitError,
    #[err(-3, "任务 panic")]
    TaskPanic,
}

// ═══════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodeMsg;

    #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
    #[err("SYS")]
    pub enum SysErr {
        #[err(0, "Success")] Success,
        #[err(1001, "Config load failed")] ConfigLoadFailed,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
    #[err("USER")]
    pub enum UserErr {
        #[err(2001, "User not found")] NotFound,
        #[err(2002, "Permission denied")] PermissionDenied,
    }

    #[test]
    fn test_err_code() {
        let err: AppError = SysErr::ConfigLoadFailed.into();
        assert_eq!(err.domain(), "SYS");
        assert_eq!(err.code(), 1001);
        assert_eq!(err.message(), "Config load failed");
        assert_eq!(err.context(), None);
        assert!(!err.is_internal());
    }

    #[test]
    fn test_with_context() {
        let err = AppError::with_context(UserErr::NotFound, "id=42");
        assert_eq!(err.domain(), "USER");
        assert_eq!(err.code(), 2001);
        assert_eq!(err.context(), Some("id=42"));
        assert_eq!(err.full_message(), "User not found: id=42");
        assert_eq!(err.to_string(), "[USER:2001] User not found: id=42");
    }

    #[test]
    fn test_internal() {
        let err: AppError = anyhow::anyhow!("db connection failed").into();
        assert!(err.is_internal());
        assert_eq!(err.code(), 90000);
        assert!(err.to_string().contains("db connection failed"));
    }

    #[test]
    fn test_equality_ignores_context() {
        let a = AppError::with_context(UserErr::NotFound, "id=1");
        let b: AppError = UserErr::NotFound.into();
        assert_eq!(a, b); // 同类错误相等
    }

    #[test]
    fn test_is_same_kind() {
        let a: AppError = SysErr::ConfigLoadFailed.into();
        let b: AppError = UserErr::NotFound.into();
        let c = AppError::internal(anyhow::anyhow!("test"));
        assert!(a.is_same_kind(&a));
        assert!(!a.is_same_kind(&b));
        assert!(!a.is_same_kind(&c));
    }

    #[test]
    fn test_di_err() {
        let err: AppError = DiErr::RegistryError.into();
        assert_eq!(err.domain(), "DI");
        assert_eq!(err.code(), -1);
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: AppError = io_err.into();
        assert!(err.is_internal());
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_string_conversion() {
        let err: AppError = "something went wrong".into();
        assert!(err.is_internal());
    }
}
