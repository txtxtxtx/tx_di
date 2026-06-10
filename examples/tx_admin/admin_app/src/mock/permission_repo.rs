use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::permission::model::value_object::{PermissionCheck, PermissionType};
use admin_domain::permission::repository::PermissionRepository;
use tx_error::AppResult;

/// Mock permission repository for testing
pub struct MockPermissionRepository {
    /// role_id -> list of permission codes
    role_permissions: RwLock<HashMap<u64, Vec<String>>>,
    /// user_id -> role_ids
    user_roles: RwLock<HashMap<u64, Vec<u64>>>,
    /// all available permissions
    all_permissions: Vec<PermissionCheck>,
}

impl MockPermissionRepository {
    pub fn new() -> Self {
        Self {
            role_permissions: RwLock::new(HashMap::new()),
            user_roles: RwLock::new(HashMap::new()),
            all_permissions: Self::default_permissions(),
        }
    }

    pub fn with_user_roles(self, user_id: u64, role_ids: Vec<u64>) -> Self {
        {
            let mut user_roles = self.user_roles.write().unwrap();
            user_roles.insert(user_id, role_ids);
        }
        self
    }

    pub fn with_role_permissions(self, role_id: u64, permissions: Vec<String>) -> Self {
        {
            let mut role_permissions = self.role_permissions.write().unwrap();
            role_permissions.insert(role_id, permissions);
        }
        self
    }

    fn default_permissions() -> Vec<PermissionCheck> {
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
        ]
    }
}

impl Default for MockPermissionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PermissionRepository for MockPermissionRepository {
    async fn find_by_role_ids(&self, role_ids: &[u64]) -> AppResult<Vec<String>> {
        let role_permissions = self.role_permissions.read().unwrap();
        let mut permissions = Vec::new();
        for role_id in role_ids {
            if let Some(perms) = role_permissions.get(role_id) {
                permissions.extend(perms.clone());
            }
        }
        permissions.sort();
        permissions.dedup();
        Ok(permissions)
    }

    async fn find_by_user_id(&self, user_id: u64) -> AppResult<Vec<String>> {
        let role_ids = {
            let user_roles = self.user_roles.read().unwrap();
            user_roles.get(&user_id).cloned().unwrap_or_default()
        };
        self.find_by_role_ids(&role_ids).await
    }

    async fn find_all(&self) -> AppResult<Vec<PermissionCheck>> {
        Ok(self.all_permissions.clone())
    }
}
