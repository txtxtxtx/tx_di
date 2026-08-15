//! Repository 层公共工具
//!
//! 各域仓储错误类型已下沉到对应域的 `repository` 模块，
//! 此处仅保留数据库错误转换辅助函数。

/// 重导出 `tx_error::log_err`
///
/// 日志格式: `[DOMAIN:CODE] MESSAGE: 原始错误信息`
///
/// # 用法
/// ```ignore
/// .map_err(|e| db_err(e, UserRepositoryError::DatabaseUser))?
/// ```
pub use tx_error::log_err as db_err;
