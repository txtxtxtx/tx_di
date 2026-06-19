//! tx_di_file — 统一文件存储插件
//!
//! 基于 Apache OpenDAL 提供统一的 `FileStorage` trait，
//! 支持多种存储后端：
//! - **本地文件系统**（`local` feature，默认启用）
//! - **AWS S3 / MinIO**（`s3` feature，可选）
//! - 未来可扩展 GCS、Azure Blob、OBS 等
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
//! base_url = "http://localhost:8080/files"
//! # S3 配置（当 backend = "s3" 时生效）
//! [file_config.s3]
//! bucket = "my-bucket"
//! region = "ap-southeast-1"
//! endpoint = "http://localhost:9000"   # MinIO 等兼容端点
//! access_key = "minioadmin"
//! secret_key = "minioadmin"
//! force_path_style = true
//! ```
//!
//! # Feature Flags
//!
//! | Feature | 说明 |
//! |---------|------|
//! | `local` | 本地文件系统存储（默认） |
//! | `s3`    | AWS S3 / MinIO 存储 |
//!
//! # 流式 I/O
//!
//! `FileStorage` trait 支持流式读写，避免大文件消耗内存：
//!
//! ```rust,ignore
//! use tx_di_file::storage::FileStorage;
//!
//! // 流式上传
//! storage.write_stream("large_file.bin", &mut reader, Some("application/octet-stream")).await?;
//!
//! // 流式下载
//! let mut reader = storage.read_stream("large_file.bin").await?;
//! tokio::io::copy(&mut reader, &mut output).await?;
//!
//! // 小文件便捷方法
//! storage.upload("small.txt", b"hello", None).await?;
//! let data = storage.download("small.txt").await?;
//! ```

mod config;
mod plugin;
pub mod storage;

pub use config::FileConfig;
pub use plugin::FilePlugin;
pub use storage::{FileInfo, FileStorage, FileStorageErr, OpendalStorage};
