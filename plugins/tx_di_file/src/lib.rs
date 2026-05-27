//! tx_di_file — 统一文件存储插件
//!
//! 提供统一的 `FileStorage` trait，支持多种存储后端：
//! - **本地文件系统**（`local` feature，默认启用）
//! - **AWS S3**（`s3` feature，可选）
//!
//! # 快速开始
//!
//! ```toml
//! # Cargo.toml
//! tx_di_file = { path = "plugins/tx_di_file" }
//! # 或启用 S3 支持
//! tx_di_file = { path = "plugins/tx_di_file", features = ["s3"] }
//! ```
//!
//! ```toml
//! # config/config.toml
//! [file_config]
//! backend = "local"              # "local" 或 "s3"
//! base_path = "./uploads"
//! # S3 配置（当 backend = "s3" 时生效）
//! [file_config.s3]
//! bucket = "my-bucket"
//! region = "ap-southeast-1"
//! endpoint = ""                  # 可选，兼容 MinIO 等 S3 兼容存储
//! access_key = ""
//! secret_key = ""
//! ```
//!
//! # Feature Flags
//!
//! | Feature | 说明 |
//! |---------|------|
//! | `local` | 本地文件系统存储（默认） |
//! | `s3`    | AWS S3 / MinIO 存储 |

mod config;
mod plugin;
pub mod storage;

pub use config::FileConfig;
pub use plugin::FilePlugin;
pub use storage::{FileInfo, FileStorage, LocalFileStorage};

#[cfg(feature = "s3")]
pub use storage::S3FileStorage;
