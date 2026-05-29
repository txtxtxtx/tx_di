use std::fmt;

use crate::code::{AppErrCode, CodeMsg};

/// 统一错误类型。
///
/// 只存储归一化后的值字段（domain, code, message），
/// 无堆分配，无虚表，可 `Copy`/`Clone`，可直接比较错误身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// 业务错误码（归一化值类型）
    ErrCode {
        domain: &'static str,
        code: i32,
        message: &'static str,
    },
}

impl AppError {
    /// 泛型构造函数 — 单态化入口点。
    ///
    /// 编译器会为每个具体的 `CodeMsg` 实现生成特化代码，
    /// 运行时无虚表调用，无堆分配。
    #[inline]
    pub fn from_code<C: CodeMsg>(code: C) -> Self {
        let c = code.err_code();
        Self::ErrCode {
            domain: c.domain,
            code: c.code,
            message: c.message,
        }
    }

    #[inline]
    pub fn domain(&self) -> &'static str {
        match self {
            Self::ErrCode { domain, .. } => domain,
        }
    }

    #[inline]
    pub fn code(&self) -> i32 {
        match self {
            Self::ErrCode { code, .. } => *code,
        }
    }

    #[inline]
    pub fn message(&self) -> &'static str {
        match self {
            Self::ErrCode { message, .. } => message,
        }
    }

    /// 获取内部的 `AppErrCode`
    #[inline]
    pub fn err_code(&self) -> AppErrCode {
        match self {
            Self::ErrCode { domain, code, message } => {
                AppErrCode::new(*domain, *code, *message)
            }
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ErrCode { domain, code, message } => {
                write!(f, "[{domain}:{code}] {message}")
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

    // ── 语法1: 简洁形式（推荐） ──────────────────────────────
    #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
    #[err("SYS")]
    pub enum SysErr {
        #[err(0, "Success")]
        Success,
        #[err(1001, "Config load failed")]
        ConfigLoadFailed,
        #[err(9999, "Unknown error")]
        Unknown,
    }

    // ── 语法2: 命名参数形式（兼容） ──────────────────────────
    #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
    #[err(domain = "USER")]
    pub enum UserErr {
        #[err(code = 2001, msg = "User not found")]
        NotFound,
        #[err(code = 2002, msg = "Permission denied")]
        PermissionDenied,
    }

    // ── 语法3: 只传消息，code 默认 -1（通用错误） ──────────
    #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
    #[err("BIZ")]
    pub enum BizErr {
        #[err("Something went wrong")]
        GenericFailure,
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

        // 同 domain + code → 相等（message 不参与比较）
        assert_eq!(a, b);
        // 不同 domain → 不等
        assert_ne!(a, c);
    }

    #[test]
    fn test_from_enum_into_app_error() {
        let err: AppError = SysErr::ConfigLoadFailed.into();
        assert_eq!(err.domain(), "SYS");
        assert_eq!(err.code(), 1001);
        assert_eq!(err.message(), "Config load failed");
    }

    #[test]
    fn test_app_error_display() {
        let err: AppError = UserErr::NotFound.into();
        assert_eq!(err.to_string(), "[USER:2001] User not found");
    }

    #[test]
    fn test_code_msg_trait() {
        let code = SysErr::err_code(SysErr::Success);
        assert_eq!(code.domain, "SYS");
        assert_eq!(code.code, 0);
        assert_eq!(code.message, "Success");

        // 便捷方法
        assert_eq!(SysErr::domain(SysErr::Success), "SYS");
        assert_eq!(SysErr::code(SysErr::Success), 0);
        assert_eq!(SysErr::message(SysErr::Success), "Success");
    }

    #[test]
    fn test_app_error_equality() {
        let a: AppError = SysErr::ConfigLoadFailed.into();
        let b: AppError = SysErr::ConfigLoadFailed.into();
        assert_eq!(a, b);
    }

    #[test]
    fn test_app_result_type() {
        fn ok_fn() -> AppResult<i32> {
            Ok(42)
        }
        fn err_fn() -> AppResult<i32> {
            Err(SysErr::Unknown.into())
        }
        assert_eq!(ok_fn().unwrap(), 42);
        assert!(err_fn().is_err());
    }

    #[test]
    fn test_default_error_code() {
        // 只传消息时 code 默认 -1
        let err: AppError = BizErr::GenericFailure.into();
        assert_eq!(err.domain(), "BIZ");
        assert_eq!(err.code(), -1);
        assert_eq!(err.message(), "Something went wrong");
        assert_eq!(err.to_string(), "[BIZ:-1] Something went wrong");
    }

    #[test]
    fn test_short_syntax_works() {
        // 简洁语法 #[err("SYS")] + #[err(0, "Success")]
        let code = SysErr::err_code(SysErr::Success);
        assert_eq!(code.code, 0);

        // 命名语法 #[err(domain = "USER")] + #[err(code = 2001, msg = "...")]
        let code = UserErr::err_code(UserErr::NotFound);
        assert_eq!(code.code, 2001);
    }
}
