use std::sync::Arc;
use async_trait::async_trait;
use std::collections::HashSet;

use admin_domain::permission::model::aggregate::Permission;
use admin_domain::permission::model::value_object::{PermissionCheck, PermissionType};
use admin_domain::permission::repository::PermissionRepository;
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::RepositoryError;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::SysPermission;
use crate::menu::model::SysMenu;
use crate::role::model::SysRoleMenu;
use crate::user::model::SysUserRole;

/// Toasty 实现的 PermissionRepository
#[tx_comp(as_trait = dyn PermissionRepository)]
pub struct ToastyPermissionRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyPermissionRepository {
    fn to_domain(p: &SysPermission) -> Permission {
        Permission::restore(
            p.id as u64,
            p.name.clone(),
            p.permission_code.clone(),
            PermissionType::from(p.permission_type),
            p.parent_id as u64,
            p.sort,
            if p.description.is_empty() { None } else { Some(p.description.clone()) },
            p.status,
            AuditFields {
                creator: if p.creator.is_empty() { None } else { Some(p.creator.clone()) },
                create_time: p.created_at.parse().unwrap_or_default(),
                updater: if p.updater.is_empty() { None } else { Some(p.updater.clone()) },
                update_time: p.updated_at.parse().unwrap_or_default(),
                deleted: if p.deleted == 1 { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }

    fn to_check(p: &SysPermission) -> PermissionCheck {
        PermissionCheck {
            code: p.permission_code.clone(),
            name: p.name.clone(),
            permission_type: PermissionType::from(p.permission_type),
        }
    }

    async fn get_permission_codes_by_role_ids(&self, role_ids: &[u64]) -> AppResult<HashSet<String>> {
        let mut db = self.plugin.db().clone();
        let mut codes = HashSet::new();

        for &role_id in role_ids {
            let role_menus = SysRoleMenu::filter_by_role_id(role_id as i64)
                .exec(&mut db)
                .await
                .map_err(|_| RepositoryError::Database)?;

            for rm in role_menus {
                if let Ok(menu) = SysMenu::get_by_id(&mut db, rm.menu_id).await {
                    if menu.types == 2 && menu.deleted == 0 && !menu.permission.is_empty() {
                        codes.insert(menu.permission.clone());
                    }
                }
            }
        }

        Ok(codes)
    }
}

#[async_trait]
impl PermissionRepository for ToastyPermissionRepository {
    async fn find_by_role_ids(&self, role_ids: &[u64]) -> AppResult<HashSet<String>> {
        self.get_permission_codes_by_role_ids(role_ids).await
    }

    async fn find_by_user_id(&self, user_id: u64) -> AppResult<HashSet<String>> {
        let mut db = self.plugin.db().clone();
        let user_roles = SysUserRole::filter_by_user_id(user_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        let role_ids: Vec<u64> = user_roles.into_iter().map(|ur| ur.role_id as u64).collect();
        self.get_permission_codes_by_role_ids(&role_ids).await
    }

    async fn find_all(&self) -> AppResult<HashSet<PermissionCheck>> {
        let mut db = self.plugin.db().clone();
        let all = SysPermission::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|p| p.deleted == 0)
            .map(Self::to_check)
            .collect())
    }

    async fn find_by_id(&self, id: u64) -> AppResult<Option<Permission>> {
        let mut db = self.plugin.db().clone();
        match SysPermission::get_by_id(&mut db, id as i64).await {
            Ok(p) if p.deleted == 0 => Ok(Some(Self::to_domain(&p))),
            _ => Ok(None),
        }
    }

    async fn find_by_code(&self, code: &str) -> AppResult<Option<Permission>> {
        let mut db = self.plugin.db().clone();
        let perm = SysPermission::filter_by_permission_code(code)
            .first()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        match perm {
            Some(p) if p.deleted == 0 => Ok(Some(Self::to_domain(&p))),
            _ => Ok(None),
        }
    }

    async fn find_all_permissions(&self) -> AppResult<Vec<Permission>> {
        let mut db = self.plugin.db().clone();
        let all = SysPermission::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|p| p.deleted == 0)
            .map(Self::to_domain)
            .collect())
    }

    async fn insert(&self, permission: &Permission) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysPermission::create()
            .name(permission.name.clone())
            .permission_code(permission.permission_code.clone())
            .permission_type(permission.permission_type as i32)
            .parent_id(permission.parent_id as i64)
            .sort(permission.sort)
            .description(permission.description.clone().unwrap_or_default())
            .status(permission.status)
            .creator(permission.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(permission.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(permission.audit.deleted as i32)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn update(&self, permission: &Permission) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysPermission::get_by_id(&mut db, permission.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .name(permission.name.clone())
            .permission_code(permission.permission_code.clone())
            .permission_type(permission.permission_type as i32)
            .parent_id(permission.parent_id as i64)
            .sort(permission.sort)
            .description(permission.description.clone().unwrap_or_default())
            .status(permission.status)
            .updater(permission.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(permission.audit.deleted as i32)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut perm = SysPermission::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        perm.update()
            .deleted(1)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn exists_by_code(&self, code: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let perm = SysPermission::filter_by_permission_code(code)
            .first()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(perm.map(|p| p.deleted == 0).unwrap_or(false))
    }
}
