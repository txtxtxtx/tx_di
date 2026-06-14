//! 基础设施层 - Toasty ORM Repository 实现
//!
//! 实现 domain 层定义的所有 Repository trait，
//! 使用 toasty ORM 连接数据库（SQLite/PostgreSQL/MySQL）。

pub mod user;
pub mod role;
pub mod permission;
pub mod menu;
pub mod department;
pub mod file;
pub mod config;
pub mod dictionary;
pub mod log;
