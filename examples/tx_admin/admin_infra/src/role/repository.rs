use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::RepositoryError;
use admin_domain::role::model::aggregate::Role;
use admin_domain::role::model::value_object::RoleQuery;
use admin_domain::role::repository::RoleRepository;
use admin_domain::user::model::aggregate::User;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::{SysRole, SysRoleMenu};
use crate::user::model::{SysUser, SysUserRole};
use crate::common::{Status, Deleted};

/// Toasty 实现的 RoleRepository
#[tx_comp(as_trait = dyn RoleRepository)]
pub struct ToastyRoleRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyRoleRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(r: &SysRole, menu_ids: Vec<u64>) -> Role {
        Role::restore(
            r.id as u64,
            r.name.clone(),
            r.code.clone(),
            r.sort,
            r.data_scope,
            if r.data_scope_dept_ids.is_empty() { None } else { Some(r.data_scope_dept_ids.clone()) },
            i32::from(r.status),
            if r.remark.is_empty() { None } else { Some(r.remark.clone()) },
            r.tenant_id,
            AuditFields {
                creator: if r.creator.is_empty() { None } else { Some(r.creator.clone()) },
                create_time: r.created_at.parse().unwrap_or_default(),
                updater: if r.updater.is_empty() { None } else { Some(r.updater.clone()) },
                update_time: r.updated_at.parse().unwrap_or_default(),
                deleted: if r.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
            menu_ids,
        )
    }

    async fn fetch_menu_ids(&self, role_id: i64) -> AppResult<Vec<u64>> {
        let mut db = self.plugin.db().clone();
        let menus = SysRoleMenu::filter_by_role_id(role_id)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;
        Ok(menus.into_iter().map(|m| m.menu_id as u64).collect())
    }

    async fn to_full_domain(&self, r: &SysRole) -> AppResult<Role> {
        let menu_ids = self.fetch_menu_ids(r.id).await?;
        Ok(Self::to_domain(r, menu_ids))
    }

    fn sys_user_to_domain(u: &SysUser) -> User {
        User::restore(
            u.id as u64,
            u.username.clone(),
            u.password_hash.clone(),
            u.nickname.clone(),
            if u.remark.is_empty() { None } else { Some(u.remark.clone()) },
            if u.email.is_empty() { None } else { Some(u.email.clone()) },
            if u.mobile.is_empty() { None } else { Some(u.mobile.clone()) },
            admin_domain::user::model::value_object::Sex::from(u.sex),
            if u.avatar.is_empty() { None } else { Some(u.avatar.clone()) },
            admin_domain::user::model::value_object::UserStatus::from(u.status),
            if u.login_ip.is_empty() { None } else { Some(u.login_ip.clone()) },
            if u.login_date.is_empty() { None } else { u.login_date.parse().ok() },
            admin_domain::shared::model::value_object::TenantId::new(u.tenant_id as u64),
            AuditFields {
                creator: if u.creator.is_empty() { None } else { Some(u.creator.clone()) },
                create_time: u.created_at.parse().unwrap_or_default(),
                updater: if u.updater.is_empty() { None } else { Some(u.updater.clone()) },
                update_time: u.updated_at.parse().unwrap_or_default(),
                deleted: if u.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
            Vec::new(),
            Vec::new(),
        )
    }
}

#[async_trait]
impl RoleRepository for ToastyRoleRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Role>> {
        let mut db = self.plugin.db().clone();
        match SysRole::get_by_id(&mut db, id as i64).await {
            Ok(r) if r.deleted == Deleted::No => Ok(Some(self.to_full_domain(&r).await?)),
            _ => Ok(None),
        }
    }

    async fn find_by_code(&self, code: &str) -> AppResult<Option<Role>> {
        let mut db = self.plugin.db().clone();
        let role = SysRole::filter_by_code(code)
            .first()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;
        match role {
            Some(r) if r.deleted == Deleted::No => Ok(Some(self.to_full_domain(&r).await?)),
            _ => Ok(None),
        }
    }

    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Role>> {
        let mut db = self.plugin.db().clone();
        let all = SysRole::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;

        let mut roles = Vec::new();
        for r in all {
            if r.deleted == Deleted::No && ids.contains(&(r.id as u64)) {
                roles.push(self.to_full_domain(&r).await?);
            }
        }
        Ok(roles)
    }

    async fn find_page(&self, query: &RoleQuery, page: Page<Role>) -> AppResult<Page<Role>> {
        let mut db = self.plugin.db().clone();
        let all = SysRole::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;

        let filtered: Vec<SysRole> = all
            .into_iter()
            .filter(|r| r.deleted == Deleted::No)
            .filter(|r| {
                if let Some(ref name) = query.name {
                    if !r.name.contains(name.as_str()) { return false; }
                }
                if let Some(ref code) = query.code {
                    if !r.code.contains(code.as_str()) { return false; }
                }
                if let Some(status) = query.status {
                    if i32::from(r.status) != status { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let mut roles = Vec::new();
        for r in filtered.into_iter().skip(offset).take(size) {
            roles.push(self.to_full_domain(&r).await?);
        }

        Ok(Page::new(roles, page.page, page.size, total))
    }

    async fn find_all(&self, query: &RoleQuery) -> AppResult<Vec<Role>> {
        let mut db = self.plugin.db().clone();
        let all = SysRole::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;

        let mut roles = Vec::new();
        for r in all.into_iter().filter(|r| r.deleted == Deleted::No) {
            if let Some(ref name) = query.name {
                if !r.name.contains(name.as_str()) { continue; }
            }
            if let Some(ref code) = query.code {
                if !r.code.contains(code.as_str()) { continue; }
            }
            if let Some(status) = query.status {
                if i32::from(r.status) != status { continue; }
            }
            roles.push(self.to_full_domain(&r).await?);
        }
        Ok(roles)
    }

    async fn insert(&self, role: &Role) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        let model = SysRole::create()
            .id(role.id as i64)
            .name(role.name.clone())
            .code(role.code.clone())
            .sort(role.sort)
            .data_scope(role.data_scope)
            .data_scope_dept_ids(role.data_scope_dept_ids.clone().unwrap_or_default())
            .status(Status::from(role.status))
            .remark(role.remark.clone().unwrap_or_default())
            .tenant_id(role.tenant_id)
            .creator(role.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(role.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(role.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;

        for &menu_id in &role.menu_ids {
            SysRoleMenu::create()
                .role_id(model.id)
                .menu_id(menu_id as i64)
                .exec(&mut db)
                .await
                .map_err(|_| RepositoryError::DatabaseRole)?;
        }

        Ok(())
    }

    async fn update(&self, role: &Role) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysRole::get_by_id(&mut db, role.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFoundRole)?;

        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .name(role.name.clone())
            .code(role.code.clone())
            .sort(role.sort)
            .data_scope(role.data_scope)
            .data_scope_dept_ids(role.data_scope_dept_ids.clone().unwrap_or_default())
            .status(Status::from(role.status))
            .remark(role.remark.clone().unwrap_or_default())
            .tenant_id(role.tenant_id)
            .updater(role.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(role.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;

        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut role = SysRole::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFoundRole)?;

        role.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;

        Ok(())
    }

    async fn exists_by_code(&self, code: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let role = SysRole::filter_by_code(code)
            .first()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;
        Ok(role.map(|r| r.deleted == Deleted::No).unwrap_or(false))
    }

    async fn bind_menus(&self, role_id: u64, menu_ids: &[u64]) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let old = SysRoleMenu::filter_by_role_id(role_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;

        for rm in old {
            rm.delete().exec(&mut db)
                .await
                .map_err(|_| RepositoryError::DatabaseRole)?;
        }

        for &menu_id in menu_ids {
            SysRoleMenu::create()
                .role_id(role_id as i64)
                .menu_id(menu_id as i64)
                .exec(&mut db)
                .await
                .map_err(|_| RepositoryError::DatabaseRole)?;
        }

        Ok(())
    }

    async fn get_menu_ids(&self, role_id: u64) -> AppResult<Vec<u64>> {
        self.fetch_menu_ids(role_id as i64).await
    }

    async fn get_user_ids(&self, role_id: u64) -> AppResult<Vec<u64>> {
        let mut db = self.plugin.db().clone();
        let user_roles = SysUserRole::filter_by_role_id(role_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;
        Ok(user_roles.into_iter().map(|ur| ur.user_id as u64).collect())
    }

    async fn find_users_by_role_id(&self, role_id: u64) -> AppResult<Vec<User>> {
        let mut db = self.plugin.db().clone();
        let user_roles = SysUserRole::filter_by_role_id(role_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;

        let mut users = Vec::new();
        for ur in user_roles {
            if let Ok(u) = SysUser::get_by_id(&mut db, ur.user_id).await {
                if u.deleted == Deleted::No {
                    users.push(Self::sys_user_to_domain(&u));
                }
            }
        }
        Ok(users)
    }

    async fn bind_users(&self, role_id: u64, user_ids: &[u64]) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        for &user_id in user_ids {
            SysUserRole::create()
                .user_id(user_id as i64)
                .role_id(role_id as i64)
                .exec(&mut db)
                .await
                .map_err(|_| RepositoryError::DatabaseRole)?;
        }
        Ok(())
    }

    async fn unbind_users(&self, role_id: u64, user_ids: &[u64]) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let user_roles = SysUserRole::filter_by_role_id(role_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::DatabaseRole)?;

        for ur in user_roles {
            if user_ids.contains(&(ur.user_id as u64)) {
                ur.delete().exec(&mut db)
                    .await
                    .map_err(|_| RepositoryError::DatabaseRole)?;
            }
        }
        Ok(())
    }
}
