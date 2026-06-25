//! 文件插件统一错误类型
//!
//! 所有插件层面的错误码集中在此模块：
//!
//! | 错误码 | 说明 |
//! |--------|------|
//! | FILE-4001 | 文件未找到 |
//! | FILE-4002 | 文件已存在 |
//! | FILE-4003 | 文件大小超限 |
//! | FILE-4004 | 不允许的文件类型 |
//! | FILE-5001 | 存储后端不存在 |
//! | FILE-5002 | 系统存储后端不允许移除 |
//! | FILE-5003 | 存储后端初始化失败 |

use tx_error::{AppError, CodeMsg};

/// 文件存储业务错误码（底层 I/O 操作相关）
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

/// 文件插件业务错误码（后端管理相关）
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("FILE")]
pub enum FilePluginErr {
    /// 指定的存储后端 key 不存在
    #[err(5001, "存储后端不存在")]
    StorageNotFound,

    /// 尝试移除系统存储后端（`sys:` 前缀）
    #[err(5002, "系统存储后端不允许移除")]
    CannotRemoveSystemStorage,

    /// 存储后端初始化失败（配置错误或功能未启用）
    #[err(5003, "存储后端初始化失败")]
    StorageInitFailed,
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
