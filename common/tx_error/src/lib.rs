//! # tx_error — 统一错误设计
//!
//! 一个错误类型 `AppError` 贯穿全栈，三种形态：
//! - `ErrCode` — 业务错误码（零堆分配）
//! - `WithContext` — 带动态上下文
//! - `Internal` — 框架/IO/第三方库错误
//!
//! ## 使用
//!
//! ```rust,ignore
//! use tx_error::{AppError, AppResult, CodeMsg};
//!
//! #[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
//! #[err("USER")]
//! pub enum UserErr {
//!     #[err(2001, "User not found")] NotFound,
//! }
//!
//! // 无上下文
//! let err: AppError = UserErr::NotFound.into();
//!
//! // 带上下文
//! let err = AppError::with_context(UserErr::NotFound, format!("id={}", 42));
//!
//! // 内部错误（anyhow 自动转换）
//! let err: AppError = anyhow::anyhow!("db failed").into();
//! ```

extern crate self as tx_error;

mod code;
mod error;

pub use code::{AppErrCode, CodeMsg};
pub use error::{AppError, AppResult, log_err};

pub use tx_macros::CodeMsg;

#[cfg(feature = "axum")]
mod axum_support;
