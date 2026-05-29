//! Web 框架插件错误码

use tx_error::CodeMsg;

/// Web 框架插件业务错误码。
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("WEB")]
#[ie(tx_di_core::IE)]
pub enum WebErrCode {
    /// 无效的监听地址
    #[err(-1, "无效的监听地址")]
    InvalidAddress,
    /// Web 服务器启动失败
    #[err(-2, "Web 服务器启动失败")]
    ServerStartFailed,
    /// DI 组件未找到
    #[err(-3, "DI 组件未找到")]
    ComponentNotFound,
}
