//! 权限检查工具
//!
//! 提供 `ensure_permission` 函数，支持管理员 `*` 通配符跳过权限验证。

use crate::error::ApiErr;
use tracing::debug;
use tx_di_sa_token::StpUtil;
/// 管理员权限
pub static ADMIN_ROLE: &str = "admin";
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
    if StpUtil::check_role(login_id.clone(), ADMIN_ROLE).await.is_ok() {
        debug!("{} is admin, skip permission check",login_id);
        return Ok(());
    }
    StpUtil::check_permission(login_id, permission).await?;
    Ok(())
}
