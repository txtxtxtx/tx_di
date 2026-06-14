//! 基础设施层 - Toasty ORM Repository 实现
//!
//! 实现 domain 层定义的所有 Repository trait，
//! 使用 toasty ORM 连接数据库（SQLite/PostgreSQL/MySQL）。
//!
//! ## Feature Flags
//! - `mock` — 启用内存 Mock 仓库（默认关闭，用于测试）

pub mod user;
pub mod role;
pub mod permission;
pub mod menu;
pub mod department;
pub mod file;
pub mod config;
pub mod dictionary;
pub mod log;

/// Mock 仓库实现（内存存储，用于测试）
///
/// 启用方式：`admin_infra = { features = ["mock"] }`
#[cfg(feature = "mock")]
pub mod mock;
