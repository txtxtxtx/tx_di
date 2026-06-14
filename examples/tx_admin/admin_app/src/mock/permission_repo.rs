use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::permission::model::aggregate::Permission;
use admin_domain::permission::model::value_object::{PermissionCheck, PermissionType};
use admin_domain::permission::repository::PermissionRepository;
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::repository::RepositoryError;
use tx_di_core::{tx_comp, tx_cst};
use tx_error::AppResult;

/// Mock permission repository for testing
#[tx_comp(as_trait = dyn PermissionRepository)]
pub struct MockPermissionRepository {
    /// role_id -> set of permission codes
    #[tx_cst(RwLock::new(HashMap::new()))]
    role_permissions: RwLock<HashMap<u64, HashSet<String>>>,
    /// user_id -> role_ids
    #[tx_cst(RwLock::new(HashMap::new()))]
    user_roles: RwLock<HashMap<u64, Vec<u64>>>,
    /// all available permissions (lightweight check items)
    #[tx_cst(Self::default_permissions())]
    all_permissions: HashSet<PermissionCheck>,
    /// Permission CRUD storage: id -> Permission
    #[tx_cst(RwLock::new(HashMap::new()))]
    permissions: RwLock<HashMap<u64, Permission>>,
}

impl MockPermissionRepository {
    pub fn new() -> Self {
        Self {
            role_permissions: RwLock::new(HashMap::new()),
            user_roles: RwLock::new(HashMap::new()),
            all_permissions: Self::default_permissions(),
            permissions: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_user_roles(self, user_id: u64, role_ids: Vec<u64>) -> Self {
        {
            let mut user_roles = self.user_roles.write().unwrap();
            user_roles.insert(user_id, role_ids);
        }
        self
    }

    pub fn with_role_permissions(self, role_id: u64, permissions: HashSet<String>) -> Self {
        {
            let mut role_permissions = self.role_permissions.write().unwrap();
            role_permissions.insert(role_id, permissions);
        }
        self
    }

    pub fn with_permission(self, permission: Permission) -> Self {
        {
            let mut permissions = self.permissions.write().unwrap();
            permissions.insert(permission.id, permission);
        }
        self
    }

    fn default_permissions() -> HashSet<PermissionCheck> {
        vec![
            PermissionCheck { code: "system:user:list".into(), name: "用户查询".into(), permission_type: PermissionType::Menu },
            PermissionCheck { code: "system:user:create".into(), name: "用户新增".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:user:update".into(), name: "用户修改".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:user:delete".into(), name: "用户删除".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:user:export".into(), name: "用户导出".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:role:list".into(), name: "角色查询".into(), permission_type: PermissionType::Menu },
            PermissionCheck { code: "system:role:create".into(), name: "角色新增".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:role:update".into(), name: "角色修改".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:role:delete".into(), name: "角色删除".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:menu:list".into(), name: "菜单查询".into(), permission_type: PermissionType::Menu },
            PermissionCheck { code: "system:menu:create".into(), name: "菜单新增".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:menu:update".into(), name: "菜单修改".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:menu:delete".into(), name: "菜单删除".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:dept:list".into(), name: "部门查询".into(), permission_type: PermissionType::Menu },
            PermissionCheck { code: "system:dept:create".into(), name: "部门新增".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:dept:update".into(), name: "部门修改".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:dept:delete".into(), name: "部门删除".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "infra:config:list".into(), name: "配置查询".into(), permission_type: PermissionType::Menu },
            PermissionCheck { code: "infra:config:create".into(), name: "配置新增".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "infra:config:update".into(), name: "配置修改".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "infra:config:delete".into(), name: "配置删除".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:dict:list".into(), name: "字典查询".into(), permission_type: PermissionType::Menu },
            PermissionCheck { code: "system:dict:create".into(), name: "字典新增".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:dict:update".into(), name: "字典修改".into(), permission_type: PermissionType::Button },
            PermissionCheck { code: "system:dict:delete".into(), name: "字典删除".into(), permission_type: PermissionType::Button },
        ].into_iter().collect()
    }
}

impl Default for MockPermissionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PermissionRepository for MockPermissionRepository {
    // === 原有查询方法 ===

    async fn find_by_role_ids(&self, role_ids: &[u64]) -> AppResult<HashSet<String>> {
        let role_permissions = self.role_permissions.read().unwrap();
        let mut permissions = HashSet::new();
        for role_id in role_ids {
            if let Some(perms) = role_permissions.get(role_id) {
                permissions.extend(perms.clone());
            }
        }
        Ok(permissions)
    }

    async fn find_by_user_id(&self, user_id: u64) -> AppResult<HashSet<String>> {
        let role_ids = {
            let user_roles = self.user_roles.read().unwrap();
            user_roles.get(&user_id).cloned().unwrap_or_default()
        };
        self.find_by_role_ids(&role_ids).await
    }

    async fn find_all(&self) -> AppResult<HashSet<PermissionCheck>> {
        Ok(self.all_permissions.clone())
    }

    // === 新增 CRUD 方法 ===

    async fn find_by_id(&self, id: u64) -> AppResult<Option<Permission>> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions
            .get(&id)
            .filter(|p| p.audit.deleted == DeletedStatus::Normal)
            .cloned())
    }

    async fn find_by_code(&self, code: &str) -> AppResult<Option<Permission>> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions
            .values()
            .find(|p| p.permission_code == code && p.audit.deleted == DeletedStatus::Normal)
            .cloned())
    }

    async fn find_all_permissions(&self) -> AppResult<Vec<Permission>> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions
            .values()
            .filter(|p| p.audit.deleted == DeletedStatus::Normal)
            .cloned()
            .collect())
    }

    async fn insert(&self, permission: &Permission) -> AppResult<()> {
        let mut permissions = self.permissions.write().unwrap();
        permissions.insert(permission.id, permission.clone());
        Ok(())
    }

    async fn update(&self, permission: &Permission) -> AppResult<()> {
        let mut permissions = self.permissions.write().unwrap();
        permissions.insert(permission.id, permission.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut permissions = self.permissions.write().unwrap();
        if let Some(perm) = permissions.get_mut(&id) {
            perm.audit.deleted = DeletedStatus::Deleted;
            Ok(())
        } else {
            Err(RepositoryError::NotFound)?
        }
    }

    async fn exists_by_code(&self, code: &str) -> AppResult<bool> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions
            .values()
            .any(|p| p.permission_code == code && p.audit.deleted == DeletedStatus::Normal))
    }
}
