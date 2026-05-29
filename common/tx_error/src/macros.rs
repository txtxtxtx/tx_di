/// 为已定义的业务错误枚举实现 `CodeMsg` + `Display` + `From<AppError>`。
///
/// # 用法
///
/// 先手写枚举定义（编辑器可识别），再用宏补全 trait 实现：
///
/// ```rust,ignore
/// #[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// pub enum SysErr {
///     Success,
///     ConfigLoadFailed,
///     Unknown,
/// }
///
/// impl_code_msg! {
///     SysErr("SYS") {
///         Success          = (0,    "Success"),
///         ConfigLoadFailed = (1001, "Config load failed"),
///         Unknown          = (9999, "Unknown error"),
///     }
/// }
/// ```
///
/// 宏会生成：
/// - `CodeMsg` trait 实现（err_code / domain / code / message）
/// - `Display` trait 实现
/// - `From<枚举名> for AppError` 实现
#[macro_export]
macro_rules! impl_code_msg {
    (
        $enum_name:ident($domain:expr) {
            $( $variant:ident = ($code:expr, $msg:expr) ),* $(,)?
        }
    ) => {
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

/// 一步到位：定义枚举 + 实现所有 trait（`gen_err!` = 手写 enum + `impl_code_msg!`）。
///
/// 如果你不在意编辑器对类型的跳转提示，可以用这个宏省去手写枚举的步骤。
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

        $crate::impl_code_msg! {
            $enum_name($domain) {
                $( $variant = ($code, $msg) ),*
            }
        }
    };
}
