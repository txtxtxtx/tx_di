//! 权限检查工具
//!
//! 提供 `ensure_permission` 函数，支持管理员 `*` 通配符跳过权限验证。

use tx_di_sa_token::StpUtil;
use crate::error::ApiErr;

/// 检查当前登录用户是否拥有指定权限
///
/// - 管理员（拥有 `*` 权限）直接通过
/// - 普通用户检查具体权限码
///
/// # 用法
/// ```ignore
/// async fn handler(...) -> Result<R<T>, ApiErr> {
///     ensure_permission("user:create").await?;
///     // ... 业务逻辑
/// }
/// ```
pub async fn ensure_permission(permission: &str) -> Result<(), ApiErr> {
    let login_id = StpUtil::get_login_id_as_string().await?;

    // 获取用户权限列表
    let perms = StpUtil::get_permissions(&login_id).await;

    // 管理员通配符：拥有 * 则跳过所有权限检查
    if perms.contains(&"*".to_string()) {
        return Ok(());
    }

    // 普通用户：检查具体权限
    StpUtil::check_permission(&login_id, permission).await?;
    Ok(())
}
