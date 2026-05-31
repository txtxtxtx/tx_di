mod code_msg;
mod utils;

use proc_macro::TokenStream;

/// 统一错误码 derive 宏。
///
/// 为枚举实现 `CodeMsg` + `Display` + `From<AppError>`。
///
/// # 用法
///
/// ```rust,ignore
/// use tx_macros::CodeMsg;
///
/// #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
/// #[err("SYS")]
/// #[ie(tx_error::IE)]
/// pub enum SysErr {
///     #[err(0, "Success")]
///     Success,
///     #[err(1001, "Config load failed")]
///     ConfigLoadFailed,
/// }
/// ```
#[proc_macro_derive(CodeMsg, attributes(err, ie))]
pub fn derive_code_msg(input: TokenStream) -> TokenStream {
    code_msg::derive_code_msg_impl(input)
}
