//! tx_common — 通用工具类型
//!
//! - `ApiR<T>` / `ApiRes` — API 响应结构体
//! - `RCode` — 响应状态码枚举
//! - `FormattedDateTime` — 格式化日期时间包装器

pub mod api_r;
pub mod date;
pub mod id;
pub mod page;

pub use api_r::{ApiR, ApiRes, RCode};
pub use date::FormattedDateTime;
