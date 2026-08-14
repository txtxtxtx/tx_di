pub mod auth;
pub mod config;
pub mod department;
pub mod dictionary;
pub mod file;
pub mod log;
pub mod menu;
pub mod password;
pub mod role;
pub mod shared;
pub mod user;

/// 重新导出 AggregrateRoot 派生宏，方便 crate 内使用 `use crate::AggregateRoot;`
pub use admin_macros::AggregateRoot;
