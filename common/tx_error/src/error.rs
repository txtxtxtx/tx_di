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
        code: u16,
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
    pub fn code(&self) -> u16 {
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

// NOTE: `From<C: CodeMsg> for AppError` 不能用 blanket impl，
// 因为会与宏生成的每个具体类型冲突。
// 每个业务错误枚举通过 `gen_err!` 宏自动生成 `From<Enum> for AppError`。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gen_err, impl_code_msg};

    // === 推荐方式：手写枚举 + impl_code_msg!（编辑器友好） ===

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub enum SysErr {
        Success,
        ConfigLoadFailed,
        Unknown,
    }

    impl_code_msg! {
        SysErr("SYS") {
            Success          = (0,    "Success"),
            ConfigLoadFailed = (1001, "Config load failed"),
            Unknown          = (9999, "Unknown error"),
        }
    }

    // === 简洁方式：gen_err! 一步到位 ===

    gen_err! {
        UserErr("USER") {
            NotFound         = (2001, "User not found"),
            PermissionDenied = (2002, "Permission denied"),
        }
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
}
