//! tx_di_file — 统一文件存储插件
//!
//! 基于 Apache OpenDAL 提供统一的 `FileStorage` trait，
//! 支持多种存储后端：
//! - **本地文件系统**（`local` feature，默认启用）
//! - **AWS S3 / MinIO**（`s3` feature，可选）
//! - 未来可扩展 GCS、Azure Blob、OBS 等
//!
//! # 多后端支持
//!
//! 插件支持同时管理多个存储后端，key 命名规范：
//!
//! | 前缀 | 来源 | 示例 | 可否移除 |
//! |------|------|------|---------|
//! | `sys:` | 配置文件自动注册 | `sys:local`, `sys:s3-main` | 否 |
//! | `user:` | 运行时动态添加 | `user:my-oss` | 是 |
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
//! base_path = "./uploads"
//! base_url = "http://localhost:8080/files"
//!
//! [[file_config.extra_storages]]
//! name = "s3-images"
//! backend = "s3"
//! bucket = "my-bucket"
//! region = "ap-southeast-1"
//! endpoint = "http://localhost:9000"
//! ```
//!
//! ```rust,ignore
//! use tx_di_file::FilePlugin;
//!
//! let plugin = app.inject::<FilePlugin>();
//!
//! // 获取默认存储
//! let local = plugin.default_storage().unwrap();
//!
//! // 获取指定后端
//! let s3 = plugin.get_storage("sys:s3-images").unwrap();
//!
//! // 动态添加用户后端
//! plugin.add_storage(
//!     tx_di_file::user_key("my-oss"),
//!     Arc::new(OpendalStorage::new_s3(&s3_cfg, "")?),
//! );
//!
//! // 动态移除
//! plugin.remove_storage("user:my-oss")?;
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
//!
//! # 迁移说明（从旧版单后端迁移）
//!
//! - `create_storage()` 函数已移除，请改用 `OpendalStorage::from_json_config()` 或 `FilePlugin::add_storage()`
//! - `FilePlugin::storage()` 方法已移除，请改用 `get_storage()` / `default_storage()`

mod config;
mod error;
mod plugin;
pub mod storage;

pub use config::{FileConfig, S3Config, StorageBackend, StorageConfig};
pub use error::{FilePluginErr, FileStorageErr};
pub use plugin::FilePlugin;
pub use storage::{FileInfo, FileStorage, OpendalStorage};

use std::sync::Arc;

// ============================================================================
// 常量与辅助方法
// ============================================================================

/// 系统存储后端 key 前缀
///
/// 由配置文件自动注册的后端使用此前缀，不可通过 `remove_storage` 移除。
pub const SYS_PREFIX: &str = "sys:";

/// 用户自定义存储后端 key 前缀
///
/// 运行时通过 `add_storage` 动态添加的后端使用此前缀，可通过 `remove_storage` 移除。
pub const USER_PREFIX: &str = "user:";

/// 创建系统存储后端 key（内部使用）
///
/// # 示例
/// ```rust,ignore
/// sys_key("local")  // => "sys:local"
/// sys_key("s3-main") // => "sys:s3-main"
/// ```
#[inline]
pub(crate) fn sys_key(name: &str) -> String {
    format!("{}{}", SYS_PREFIX, name)
}

/// 创建用户自定义存储后端 key
///
/// # 示例
/// ```rust,ignore
/// user_key("my-oss")     // => "user:my-oss"
/// user_key("backup-bk")  // => "user:backup-bk"
/// ```
#[inline]
pub fn user_key(name: &str) -> String {
    format!("{}{}", USER_PREFIX, name)
}
