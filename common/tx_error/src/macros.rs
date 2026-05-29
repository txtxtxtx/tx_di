/// 定义业务错误枚举并自动实现 `CodeMsg` + `Display` + `From<AppError>`。
///
/// # 语法
///
/// ```rust,ignore
/// gen_err! {
///     SysErr("SYS") {
///         Success          = (0,    "Success"),
///         ConfigLoadFailed = (1001, "Config load failed"),
///         Unknown          = (9999, "Unknown error"),
///     }
/// }
/// ```
///
/// - `"SYS"` 是该枚举的 **domain**（错误域）
/// - 每个变体 = `(错误码, 错误消息)`
/// - 自动生成：
///   - 枚举定义（`Debug, Copy, Clone, PartialEq, Eq`）
///   - `CodeMsg` trait 实现
///   - `Display` trait 实现
///   - `From<枚举名> for AppError` 实现
#[macro_export]
macro_rules! gen_err {
    (
        $enum_name:ident($domain:expr) {
            $( $variant:ident = ($code:expr, $msg:expr) ),* $(,)?
        }
    ) => {
        /// 业务错误码枚举
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum $enum_name {
            $( $variant ),*
        }

        impl $crate::CodeMsg for $enum_name {
            fn err_code(self) -> $crate::AppErrCode {
                match self {
                    $( Self::$variant => $crate::AppErrCode::new($domain, $code, $msg), )*
                }
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let code = $crate::CodeMsg::err_code(*self);
                write!(f, "{code}")
            }
        }

        impl From<$enum_name> for $crate::AppError {
            fn from(e: $enum_name) -> Self {
                $crate::AppError::from_code(e)
            }
        }
    };
}
