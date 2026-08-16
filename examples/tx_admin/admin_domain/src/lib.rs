pub mod identity;
pub mod job;
pub mod shared;
pub mod system;

/// 重新导出 AggregrateRoot 派生宏，方便 crate 内使用 `use crate::AggregateRoot;`
pub use admin_macros::AggregateRoot;
