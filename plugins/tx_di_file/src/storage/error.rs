//! 文件存储错误类型
//!
//! 业务错误码通过 `#[derive(CodeMsg)]` 定义，
//! 动态信息（如路径、大小等）通过 `AppError::with_context` 附加。

use tx_error::{AppError, CodeMsg};

/// 文件存储业务错误码
///
/// 使用 `#[derive(CodeMsg)]` 自动生成 `CodeMsg` trait 实现，
/// 从而可无缝转为 `AppError`。
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("FILE")]
pub enum FileStorageErr {
    /// 文件未找到
    #[err(4001, "文件未找到")]
    NotFound,

    /// 文件已存在且不允许覆盖
    #[err(4002, "文件已存在")]
    AlreadyExists,

    /// 文件大小超限
    #[err(4003, "文件大小超限")]
    FileTooLarge,

    /// 扩展名不允许
    #[err(4004, "不允许的文件类型")]
    InvalidExtension,
}

/// 将 OpenDAL 错误映射为 `AppError`
///
/// 根据错误类型区分：
/// - `NotFound` / `AlreadyExists` → 业务错误码 + 路径上下文
/// - `PermissionDenied` → `Internal`（IO 权限错误）
/// - `Unsupported` / 其他 → `Internal`（存储服务错误）
pub fn map_opendal_error(e: opendal::Error, path: impl Into<String>) -> AppError {
    let path = path.into();
    match e.kind() {
        opendal::ErrorKind::NotFound => {
            AppError::with_context(FileStorageErr::NotFound, path)
        }
        opendal::ErrorKind::AlreadyExists => {
            AppError::with_context(FileStorageErr::AlreadyExists, path)
        }
        opendal::ErrorKind::PermissionDenied => {
            AppError::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))
        }
        opendal::ErrorKind::Unsupported => {
            AppError::from_anyhow(anyhow::anyhow!(e))
        }
        _ => {
            let msg = e.to_string();
            if msg.contains("not found") || msg.contains("NotFound") {
                AppError::with_context(FileStorageErr::NotFound, path)
            } else if msg.contains("already exists") {
                AppError::with_context(FileStorageErr::AlreadyExists, path)
            } else {
                AppError::from_anyhow(anyhow::anyhow!(e))
            }
        }
    }
}
