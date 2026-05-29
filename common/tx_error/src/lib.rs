//! # tx_error — 统一错误设计
//!
//! 零堆分配、无虚表、单态化的高性能错误处理框架。
//!
//! ## 核心设计
//!
//! - **`AppErrCode`**: 归一化值类型错误码（domain + code + message），纯静态引用，可 `Copy`
//! - **`CodeMsg`**: 错误码转换 trait，连接业务错误枚举与统一 `AppError`
//! - **`AppError`**: 统一错误枚举，只存储归一化后的值字段
//! - **`AppResult<T>`**: `Result<T, AppError>` 类型别名
//! - **`impl_code_msg!`**: 宏，为已定义的枚举补全 `CodeMsg` + `Display` + `From` 实现
//! - **`gen_err!`**: 宏，一步到位定义枚举 + 实现所有 trait
//!
//! ## 推荐用法（编辑器友好）
//!
//! 手写枚举定义（编辑器可识别类型、支持跳转），再用 `impl_code_msg!` 补全 trait：
//!
//! ```rust
//! use tx_error::{AppError, AppResult, CodeMsg, AppErrCode, impl_code_msg};
//!
//! // 1. 手写枚举 — 编辑器可识别
//! #[derive(Debug, Copy, Clone, PartialEq, Eq)]
//! pub enum SysErr {
//!     Success,
//!     ConfigLoadFailed,
//!     Unknown,
//! }
//!
//! // 2. 宏补全 trait 实现
//! impl_code_msg! {
//!     SysErr("SYS") {
//!         Success          = (0,    "Success"),
//!         ConfigLoadFailed = (1001, "Config load failed"),
//!         Unknown          = (9999, "Unknown error"),
//!     }
//! }
//!
//! // 使用
//! fn load_config() -> AppResult<()> {
//!     Err(SysErr::ConfigLoadFailed.into())
//! }
//!
//! // 比较错误身份（domain + code）
//! assert!(AppErrCode::new("SYS", 1001, "Config load failed") == AppErrCode::new("SYS", 1001, "其他消息"));
//! ```
//!
//! ## 简洁用法（`gen_err!`）
//!
//! 如果不在意编辑器跳转，`gen_err!` 一步到位：
//!
//! ```rust
//! use tx_error::gen_err;
//!
//! gen_err! {
//!     SysErr("SYS") {
//!         Success          = (0,    "Success"),
//!         ConfigLoadFailed = (1001, "Config load failed"),
//!         Unknown          = (9999, "Unknown error"),
//!     }
//! }
//! ```

mod code;
mod error;
mod macros;

pub use code::{AppErrCode, CodeMsg};
pub use error::{AppError, AppResult};
