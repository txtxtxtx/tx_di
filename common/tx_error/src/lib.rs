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
//! - **`gen_err!`**: 宏，自动生成业务错误枚举 + `CodeMsg` + `Display` + `From<AppError>`
//!
//! ## 使用示例
//!
//! ```rust
//! use tx_error::{AppError, AppResult, CodeMsg, AppErrCode, gen_err};
//!
//! // 定义业务错误码
//! gen_err! {
//!     SysErr("SYS") {
//!         Success             = (0,   "Success"),
//!         ConfigLoadFailed    = (1001, "Config load failed"),
//!         Unknown             = (9999, "Unknown error"),
//!     }
//! }
//!
//! gen_err! {
//!     UserErr("USER") {
//!         NotFound            = (2001, "User not found"),
//!         PermissionDenied    = (2002, "Permission denied"),
//!     }
//! }
//!
//! // 在业务代码中使用
//! fn load_config() -> AppResult<()> {
//!     // 模拟错误
//!     Err(SysErr::ConfigLoadFailed.into())
//! }
//!
//! // 比较错误身份（domain + code）
//! assert!(AppErrCode::new("SYS", 1001, "Config load failed") == AppErrCode::new("SYS", 1001, "其他消息"));
//! ```

mod code;
mod error;
mod macros;

pub use code::{AppErrCode, CodeMsg};
pub use error::{AppError, AppResult};
