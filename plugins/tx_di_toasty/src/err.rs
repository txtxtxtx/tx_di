//! Toasty ORM 插件错误码

use tx_error::CodeMsg;

/// Toasty 插件业务错误码。
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("TOASTY")]
#[ie(tx_di_core::IE)]
pub enum ToastyErr {
    /// 数据库连接失败
    #[err(-1, "数据库连接失败")]
    ConnectionFailed,
    /// Schema 构建失败
    #[err(-2, "Schema 构建失败")]
    SchemaBuildFailed,
    /// Schema 推送失败
    #[err(-3, "Schema 推送失败")]
    SchemaPushFailed,
    /// 模型注册表获取失败
    #[err(-4, "模型注册表获取失败")]
    ModelRegistryError,
}
