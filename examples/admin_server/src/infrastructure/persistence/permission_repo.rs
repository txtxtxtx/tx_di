//! 权限仓储 — 内存实现

use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;
use tx_di_core::tx_comp;

use crate::domain::permission::{Permission, PermissionId, PermissionRepository, PermissionType};
use crate::domain::tenant::TenantId;

/// 权限仓储 — 内存实现
#[derive(Debug, Default)]
#[tx_comp]
pub struct InMemoryPermissionRepository {
    #[tx_cst(RwLock::new(HashMap::new()))]
    store: RwLock<HashMap<PermissionId, Permission>>,
}

impl InMemoryPermissionRepository {
    fn read_store(&self) -> std::sync::RwLockReadGuard<HashMap<PermissionId, Permission>> {
        self.store.read().unwrap()
    }
    fn write_store(&self) -> std::sync::RwLockWriteGuard<HashMap<PermissionId, Permission>> {
        self.store.write().unwrap()
    }
}

#[async_trait]
impl PermissionRepository for InMemoryPermissionRepository {
    async fn find_by_id(&self, id: &PermissionId) -> Result<Option<Permission>, anyhow::Error> {
        Ok(self.read_store().get(id).cloned())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: Option<&TenantId>,
    ) -> Result<Vec<Permission>, anyhow::Error> {
        Ok(self
            .read_store()
            .values()
            .filter(|p| p.tenant_id.as_deref() == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_role_ids(
        &self,
        role_ids: &[crate::domain::role::RoleId],
    ) -> Result<Vec<Permission>, anyhow::Error> {
        let store = self.read_store();
        Ok(store
            .values()
            .filter(|p| {
                // 权限编码匹配角色（简化实现）
                role_ids.iter().any(|r| p.code.starts_with(r) || r == "admin")
            })
            .cloned()
            .collect())
    }

    async fn find_menu_tree(
        &self,
        tenant_id: Option<&TenantId>,
    ) -> Result<Vec<Permission>, anyhow::Error> {
        Ok(self
            .read_store()
            .values()
            .filter(|p| {
                (p.tenant_id.as_deref() == tenant_id)
                    && (p.is_menu() || p.perm_type == PermissionType::Directory)
            })
            .cloned()
            .collect())
    }

    async fn save(&self, perm: &Permission) -> Result<(), anyhow::Error> {
        self.write_store().insert(perm.id.clone(), perm.clone());
        Ok(())
    }

    async fn delete(&self, id: &PermissionId) -> Result<(), anyhow::Error> {
        self.write_store().remove(id);
        Ok(())
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<Permission>, anyhow::Error> {
        Ok(self
            .read_store()
            .values()
            .find(|p| p.code == code)
            .cloned())
    }
}
