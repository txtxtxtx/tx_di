//! 权限聚合（兼容旧代码，实际使用 menu 模块）
//!
//! 为了保持向后兼容性，此模块重新导出 menu 模块的内容。
//! 新代码应直接使用 menu 模块。

// 重新导出 menu 模块的内容
pub use super::menu::*;

/// 权限唯一标识（兼容旧代码）
pub type PermissionId = u64;

/// 权限类型（兼容旧代码，实际是 MenuType）
pub type PermissionType = super::menu::MenuType;

/// 权限实体（兼容旧代码，实际是 Menu）
pub type Permission = super::menu::Menu;

/// 权限仓储 trait（兼容旧代码）
#[async_trait::async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Permission>, anyhow::Error>;
    async fn find_all(&self) -> Result<Vec<Permission>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: Option<u64>) -> Result<Vec<Permission>, anyhow::Error>;
    async fn find_by_role_ids(&self, role_ids: &[u64]) -> Result<Vec<Permission>, anyhow::Error>;
    async fn find_menu_tree(&self) -> Result<Vec<Permission>, anyhow::Error>;
    async fn save(&self, perm: &Permission) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_by_code(&self, code: &str) -> Result<Option<Permission>, anyhow::Error>;
}
