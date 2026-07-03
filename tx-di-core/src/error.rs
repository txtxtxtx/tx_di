//! tx-di-core 错误处理
//!
//! 本 crate 不再定义独立的错误类型，全部复用 [`tx_error::AppError`]。
//! - 注入失败：`AppError` + `DiErr::InjectError`（具体上下文写入 `context()`）
//! - 注册/拓扑失败：`AppError` + `DiErr::RegistryError`（具体上下文写入 `context()`）
//!
//! 调用方可通过 `err.domain()` / `err.code()` 判定错误大类，
//! 通过 `err.context()` 取得详细描述。


pub use tx_error::{AppError, AppResult};
use crate::CodeMsg;

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
    #[err(-4, "组件注入错误")]
    InjectError,
}