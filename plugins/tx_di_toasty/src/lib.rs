//! tx_di_toasty — 基于 Toasty ORM 的 tx-di 数据库插件
//!
//! 封装 [Toasty](https://github.com/tokio-rs/toasty) 0.6+ 异步 ORM，集成到 tx-di 依赖注入框架，
//! 支持 SQLite / PostgreSQL / MySQL / DynamoDB 多数据库切换。
//!
//! # 快速开始
//!
//! ```toml
//! # Cargo.toml
//! tx_di_toasty = { path = "plugins/tx_di_toasty", features = ["sqlite"] }
//! ```
//!
//! ```toml
//! # config/config.toml
//! [toasty_config]
//! database_url = "sqlite://gb28181.db"
//! # database_url = "postgresql://user:pass@localhost/gb28181"
//! # database_url = "mysql://user:pass@localhost/gb28181"
//! auto_schema = true
//! ```
//!
//! # Feature Flags
//!
//! | Feature      | 数据库       |
//! |-------------|-------------|
//! | `sqlite`    | SQLite（默认）|
//! | `postgresql` | PostgreSQL   |
//! | `mysql`      | MySQL        |
//! | `dynamodb`   | DynamoDB    |

mod config;
mod plugin;

pub use config::ToastyConfig;
pub use plugin::{ToastyPlugin, ToastyDb};