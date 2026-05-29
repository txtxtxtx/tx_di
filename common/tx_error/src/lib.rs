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
//! - **`#[derive(CodeMsg)]`**: proc-macro，为枚举自动实现 `CodeMsg` + `Display` + `From<AppError>`
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use tx_error::{AppErrCode, AppError, AppResult, CodeMsg};
//! use tx_di_macros::CodeMsg;
//!
//! #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
//! #[err(domain = "SYS")]
//! pub enum SysErr {
//!     #[err(code = 0, msg = "Success")]
//!     Success,
//!     #[err(code = 1001, msg = "Config load failed")]
//!     ConfigLoadFailed,
//!     #[err(code = 9999, msg = "Unknown error")]
//!     Unknown,
//! }
//!
//! fn load_config() -> AppResult<()> {
//!     Err(SysErr::ConfigLoadFailed.into())
//! }
//! ```

// 允许 derive 宏生成的 `tx_error::AppErrCode` 等路径在 crate 内部也能解析
extern crate self as tx_error;

mod code;
mod error;
mod macros;

pub use code::{AppErrCode, CodeMsg};
pub use error::{AppError, AppResult};

// re-export derive 宏，用户可以直接 use tx_error::CodeMsg 来作为 derive
pub use tx_di_macros::CodeMsg;
