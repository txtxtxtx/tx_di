use std::fmt;

use crate::code::{AppErrCode, CodeMsg};

/// 统一错误类型。
///
/// 两种形态：
/// - `ErrCode`: 纯错误码，零堆分配，`Copy`
/// - `WithContext`: 带动态上下文（如 "用户 123 不存在"），`Clone` 但非 `Copy`
#[derive(Debug, Clone)]
pub enum AppError {
    /// 业务错误码（归一化值类型）
    ErrCode {
        domain: &'static str,
        code: i32,
        message: &'static str,
    },
    /// 带上下文的错误（错误码 + 动态信息）
    WithContext {
        domain: &'static str,
        code: i32,
        message: &'static str,
        context: String,
    },
}

impl AppError {
    /// 泛型构造函数 — 单态化入口点。
    #[inline]
    pub fn from_code<C: CodeMsg>(code: C) -> Self {
        let c = code.err_code();
        Self::ErrCode {
            domain: c.domain,
            code: c.code,
            message: c.message,
        }
    }

    /// 带上下文的构造函数。
    ///
    /// ```rust,ignore
    /// let err = AppError::with_context(UserErr::NotFound, format!("id={}", user_id));
    /// // Display: [USER:2001] User not found: id=123
    /// ```
    #[inline]
    pub fn with_context<C: CodeMsg>(code: C, context: impl Into<String>) -> Self {
        let c = code.err_code();
        Self::WithContext {
            domain: c.domain,
            code: c.code,
            message: c.message,
            context: context.into(),
        }
    }

    #[inline]
    pub fn domain(&self) -> &'static str {
        match self {
            Self::ErrCode { domain, .. } => domain,
            Self::WithContext { domain, .. } => domain,
        }
    }

    #[inline]
    pub fn code(&self) -> i32 {
        match self {
            Self::ErrCode { code, .. } => *code,
            Self::WithContext { code, .. } => *code,
        }
    }

    /// 获取静态消息（不含上下文）
    #[inline]
    pub fn message(&self) -> &'static str {
        match self {
            Self::ErrCode { message, .. } => message,
            Self::WithContext { message, .. } => message,
        }
    }

    /// 获取上下文信息（如果有）
    #[inline]
    pub fn context(&self) -> Option<&str> {
        match self {
            Self::ErrCode { .. } => None,
            Self::WithContext { context, .. } => Some(context),
        }
    }

    /// 获取完整消息（静态消息 + 上下文）
    pub fn full_message(&self) -> String {
        match self {
            Self::ErrCode { message, .. } => message.to_string(),
            Self::WithContext { message, context, .. } => format!("{message}: {context}"),
        }
    }

    /// 获取内部的 `AppErrCode`（丢弃上下文）
    #[inline]
    pub fn err_code(&self) -> AppErrCode {
        match self {
            Self::ErrCode { domain, code, message } => {
                AppErrCode::new(*domain, *code, *message)
            }
            Self::WithContext { domain, code, message, .. } => {
                AppErrCode::new(*domain, *code, *message)
            }
        }
    }

    /// 是否为同一类错误（domain + code 相同即为同类）
    pub fn is_same_kind(&self, other: &Self) -> bool {
        self.domain() == other.domain() && self.code() == other.code()
    }
}

// ── PartialEq: 只比较 domain + code，不比较 context ──────
impl PartialEq for AppError {
    fn eq(&self, other: &Self) -> bool {
        self.domain() == other.domain() && self.code() == other.code()
    }
}
impl Eq for AppError {}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ErrCode { domain, code, message } => {
                write!(f, "[{domain}:{code}] {message}")
            }
            Self::WithContext { domain, code, message, context } => {
                write!(f, "[{domain}:{code}] {message}: {context}")
            }
        }
    }
}

impl std::error::Error for AppError {}

/// `Result<T, AppError>` 类型别名
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodeMsg;

    #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
    #[err("SYS")]
    pub enum SysErr {
        #[err(0, "Success")]
        Success,
        #[err(1001, "Config load failed")]
        ConfigLoadFailed,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
    #[err("USER")]
    pub enum UserErr {
        #[err(2001, "User not found")]
        NotFound,
        #[err(2002, "Permission denied")]
        PermissionDenied,
    }

    #[test]
    fn test_err_code_display() {
        let code = AppErrCode::new("SYS", 1001, "Config load failed");
        assert_eq!(code.to_string(), "[SYS:1001] Config load failed");
    }

    #[test]
    fn test_err_code_identity() {
        let a = AppErrCode::new("SYS", 1001, "Config load failed");
        let b = AppErrCode::new("SYS", 1001, "Different message");
        let c = AppErrCode::new("USER", 1001, "Config load failed");

        assert_eq!(a, b); // 同 domain + code → 相等
        assert_ne!(a, c); // 不同 domain → 不等
    }

    #[test]
    fn test_from_enum_into_app_error() {
        let err: AppError = SysErr::ConfigLoadFailed.into();
        assert_eq!(err.domain(), "SYS");
        assert_eq!(err.code(), 1001);
        assert_eq!(err.message(), "Config load failed");
        assert_eq!(err.context(), None);
    }

    #[test]
    fn test_with_context() {
        let err = AppError::with_context(UserErr::NotFound, "id=42");
        assert_eq!(err.domain(), "USER");
        assert_eq!(err.code(), 2001);
        assert_eq!(err.message(), "User not found");
        assert_eq!(err.context(), Some("id=42"));
        assert_eq!(err.full_message(), "User not found: id=42");
        assert_eq!(err.to_string(), "[USER:2001] User not found: id=42");
    }

    #[test]
    fn test_error_equality_ignores_context() {
        let a = AppError::with_context(UserErr::NotFound, "id=1");
        let b = AppError::with_context(UserErr::NotFound, "id=2");
        let c: AppError = UserErr::NotFound.into();

        // 同类错误相等（context 不参与比较）
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn test_is_same_kind() {
        let a: AppError = SysErr::ConfigLoadFailed.into();
        let b: AppError = UserErr::NotFound.into();
        assert!(a.is_same_kind(&a));
        assert!(!a.is_same_kind(&b));
    }

    #[test]
    fn test_display_without_context() {
        let err: AppError = SysErr::ConfigLoadFailed.into();
        assert_eq!(err.to_string(), "[SYS:1001] Config load failed");
    }

    #[test]
    fn test_display_with_context() {
        let err = AppError::with_context(UserErr::NotFound, "username=admin");
        assert_eq!(err.to_string(), "[USER:2001] User not found: username=admin");
    }

    #[test]
    fn test_app_result_type() {
        fn ok_fn() -> AppResult<i32> { Ok(42) }
        fn err_fn() -> AppResult<i32> { Err(SysErr::Success.into()) }
        assert_eq!(ok_fn().unwrap(), 42);
        assert!(err_fn().is_err());
    }
}
