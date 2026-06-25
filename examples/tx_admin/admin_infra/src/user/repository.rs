use std::sync::Arc;
use async_trait::async_trait;
use admin_domain::shared::model::value_object::{DeletedStatus, TenantId};
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::{RepositoryError, db_err};
use admin_domain::user::model::aggregate::User;
use admin_domain::user::model::value_object::{UserQuery, UserStatus};
use admin_domain::user::repository::UserRepository;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::{SysUser, SysUserDept, SysUserRole};
use crate::common::{Sex, Status, Deleted};

/// Toasty 实现的 UserRepository
#[tx_comp(as_trait = dyn UserRepository)]
pub struct ToastyUserRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyUserRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    /// 将 toasty SysUser 转换为 domain User
    fn to_domain(u: &SysUser, role_ids: Vec<u64>, dept_ids: Vec<u64>) -> User {
        User::restore(
            u.id,
            u.username.clone(),
            u.password_hash.clone(),
            u.nickname.clone(),
            if u.remark.is_empty() { None } else { Some(u.remark.clone()) },
            if u.email.is_empty() { None } else { Some(u.email.clone()) },
            if u.mobile.is_empty() { None } else { Some(u.mobile.clone()) },
            admin_domain::user::model::value_object::Sex::from(u.sex),
            if u.avatar.is_empty() { None } else { Some(u.avatar.clone()) },
            UserStatus::from(u.status),
            if u.login_ip.is_empty() { None } else { Some(u.login_ip.clone()) },
            (u.login_date != jiff::Timestamp::UNIX_EPOCH).then(|| u.login_date),
            TenantId::new(u.tenant_id),
            AuditFields {
                creator: if u.creator.is_empty() { None } else { Some(u.creator.clone()) },
                create_time: u.created_at,
                updater: if u.updater.is_empty() { None } else { Some(u.updater.clone()) },
                update_time: u.updated_at,
                deleted: if u.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
            role_ids,
            dept_ids,
        )
    }

    /// 获取用户的角色ID列表
    async fn fetch_role_ids(&self, user_id: u64) -> AppResult<Vec<u64>> {
        let mut db = self.plugin.db().clone();
        let roles = SysUserRole::filter_by_user_id(user_id)
            .select(SysUserRole::fields().role_id())   // 只选择 role_id 列
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        Ok(roles)
    }

    /// 获取用户的部门ID列表
    async fn fetch_dept_ids(&self, user_id: u64) -> AppResult<Vec<u64>> {
        let mut db = self.plugin.db().clone();
        let depts = SysUserDept::filter_by_user_id(user_id)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        Ok(depts.into_iter().map(|d| d.dept_id).collect())
    }

    /// 获取完整的 domain User（含角色和部门）
    async fn to_full_domain(&self, u: &SysUser) -> AppResult<User> {
        let role_ids = self.fetch_role_ids(u.id).await?;
        let dept_ids = self.fetch_dept_ids(u.id).await?;
        Ok(Self::to_domain(u, role_ids, dept_ids))
    }
}

#[async_trait]
impl UserRepository for ToastyUserRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<User>> {
        let mut db = self.plugin.db().clone();
        match SysUser::get_by_id(&mut db, id).await {
            Ok(u) if u.deleted == Deleted::No => Ok(Some(self.to_full_domain(&u).await?)),
            Ok(_) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        let mut db = self.plugin.db().clone();
        let user = SysUser::filter_by_username(username)
            .first()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        match user {
            Some(u) if u.deleted == Deleted::No => Ok(Some(self.to_full_domain(&u).await?)),
            _ => Ok(None),
        }
    }

    async fn find_page(&self, query: &UserQuery, page: Page<User>) -> AppResult<Page<User>> {
        let mut db = self.plugin.db().clone();
        let all = SysUser::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        let filtered: Vec<SysUser> = all
            .into_iter()
            .filter(|u| u.deleted == Deleted::No)
            .filter(|u| {
                if let Some(ref username) = query.username {
                    if !u.username.contains(username.as_str()) { return false; }
                }
                if let Some(ref nickname) = query.nickname {
                    if !u.nickname.contains(nickname.as_str()) { return false; }
                }
                if let Some(ref mobile) = query.mobile {
                    if !u.mobile.contains(mobile.as_str()) { return false; }
                }
                if let Some(status) = query.status {
                    if u.status != Status::from(status) { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let mut users = Vec::new();
        for u in filtered.into_iter().skip(offset).take(size) {
            users.push(self.to_full_domain(&u).await?);
        }

        Ok(Page::new(users, page.page, page.size, total))
    }

    async fn find_all(&self, query: &UserQuery) -> AppResult<Vec<User>> {
        let mut db = self.plugin.db().clone();
        let all = SysUser::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        let mut users = Vec::new();
        for u in all.into_iter().filter(|u| u.deleted == Deleted::No) {
            if let Some(ref username) = query.username {
                if !u.username.contains(username.as_str()) { continue; }
            }
            if let Some(ref nickname) = query.nickname {
                if !u.nickname.contains(nickname.as_str()) { continue; }
            }
            if let Some(ref mobile) = query.mobile {
                if !u.mobile.contains(mobile.as_str()) { continue; }
            }
            if let Some(status) = query.status {
                if u.status != Status::from(status) { continue; }
            }
            users.push(self.to_full_domain(&u).await?);
        }
        Ok(users)
    }

    async fn insert(&self, user: &User) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let model = SysUser::create()
            .id(user.id)
            .username(user.username.clone())
            .password_hash(user.password.clone())
            .nickname(user.nickname.clone())
            .remark(user.remark.clone().unwrap_or_default())
            .email(user.email.clone().unwrap_or_default())
            .mobile(user.mobile.clone().unwrap_or_default())
            .sex(Sex::from(user.sex))
            .avatar(user.avatar.clone().unwrap_or_default())
            .status(Status::from(user.status))
            .login_ip(user.login_ip.clone().unwrap_or_default())
            .login_date(user.login_date.unwrap_or(jiff::Timestamp::UNIX_EPOCH))
            .tenant_id(user.tenant_id.into_inner())
            .creator(user.audit.creator.clone().unwrap_or_default())
            .updater(user.audit.updater.clone().unwrap_or_default())
            .deleted(Deleted::from(user.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        // 插入角色关联
        for &role_id in &user.role_ids {
            SysUserRole::create()
                .user_id(model.id)
                .role_id(role_id)
                .exec(&mut db)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        }

        // 插入部门关联
        for &dept_id in &user.dept_ids {
            SysUserDept::create()
                .user_id(model.id)
                .dept_id(dept_id)
                .exec(&mut db)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        }

        Ok(())
    }

    async fn update(&self, user: &User) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysUser::get_by_id(&mut db, user.id)
            .await
            .map_err(|_| RepositoryError::NotFoundUser)?;

        existing
            .update()
            .username(user.username.clone())
            .password_hash(user.password.clone())
            .nickname(user.nickname.clone())
            .remark(user.remark.clone().unwrap_or_default())
            .email(user.email.clone().unwrap_or_default())
            .mobile(user.mobile.clone().unwrap_or_default())
            .sex(Sex::from(user.sex))
            .avatar(user.avatar.clone().unwrap_or_default())
            .status(Status::from(user.status))
            .login_ip(user.login_ip.clone().unwrap_or_default())
            .login_date(user.login_date.unwrap_or(jiff::Timestamp::UNIX_EPOCH))
            .tenant_id(user.tenant_id.into_inner())
            .updater(user.audit.updater.clone().unwrap_or_default())
            .deleted(Deleted::from(user.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut user = SysUser::get_by_id(&mut db, id)
            .await
            .map_err(|_| RepositoryError::NotFoundUser)?;

        user.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        Ok(())
    }

    async fn exists_by_username(&self, username: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let user = SysUser::filter_by_username(username)
            .first()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        Ok(user.map(|u| u.deleted == Deleted::No).unwrap_or(false))
    }

    async fn exists_by_email(&self, email: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let all = SysUser::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        Ok(all.iter().any(|u| u.deleted == Deleted::No && u.email == email))
    }

    async fn exists_by_mobile(&self, mobile: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let all = SysUser::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        Ok(all.iter().any(|u| u.deleted == Deleted::No && u.mobile == mobile))
    }

    async fn count(&self, query: &UserQuery) -> AppResult<i64> {
        let mut db = self.plugin.db().clone();
        let all = SysUser::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        let count = all
            .iter()
            .filter(|u| u.deleted == Deleted::No)
            .filter(|u| {
                if let Some(ref username) = query.username {
                    if !u.username.contains(username.as_str()) { return false; }
                }
                if let Some(ref nickname) = query.nickname {
                    if !u.nickname.contains(nickname.as_str()) { return false; }
                }
                if let Some(ref mobile) = query.mobile {
                    if !u.mobile.contains(mobile.as_str()) { return false; }
                }
                if let Some(status) = query.status {
                    if u.status != Status::from(status) { return false; }
                }
                true
            })
            .count();

        Ok(count as i64)
    }

    async fn find_by_role_id(&self, role_id: u64) -> AppResult<Vec<User>> {
        let mut db = self.plugin.db().clone();
        let user_roles = SysUserRole::filter_by_role_id(role_id)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        let mut users = Vec::new();
        for ur in user_roles {
            if let Ok(u) = SysUser::get_by_id(&mut db, ur.user_id).await {
                if u.deleted == Deleted::No {
                    users.push(self.to_full_domain(&u).await?);
                }
            }
        }
        Ok(users)
    }

    async fn find_by_dept_id(&self, dept_id: u64) -> AppResult<Vec<User>> {
        let mut db = self.plugin.db().clone();
        let user_depts = SysUserDept::filter_by_dept_id(dept_id)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        let mut users = Vec::new();
        for ud in user_depts {
            if let Ok(u) = SysUser::get_by_id(&mut db, ud.user_id).await {
                if u.deleted == Deleted::No {
                    users.push(self.to_full_domain(&u).await?);
                }
            }
        }
        Ok(users)
    }

    async fn bind_roles(&self, user_id: u64, role_ids: &[u64]) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        // 先删除旧的关联
        let old = SysUserRole::filter_by_user_id(user_id)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        for ur in old {
            ur.delete().exec(&mut db)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        }

        // 插入新的关联
        for &role_id in role_ids {
            SysUserRole::create()
                .user_id(user_id)
                .role_id(role_id)
                .exec(&mut db)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        }

        Ok(())
    }

    async fn bind_departments(&self, user_id: u64, dept_ids: &[u64]) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        // 先删除旧的关联
        let old = SysUserDept::filter_by_user_id(user_id)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;

        for ud in old {
            ud.delete().exec(&mut db)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        }

        // 插入新的关联
        for &dept_id in dept_ids {
            SysUserDept::create()
                .user_id(user_id)
                .dept_id(dept_id)
                .exec(&mut db)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?;
        }

        Ok(())
    }

    async fn get_role_ids(&self, user_id: u64) -> AppResult<Vec<u64>> {
        self.fetch_role_ids(user_id).await
    }

    async fn get_dept_ids(&self, user_id: u64) -> AppResult<Vec<u64>> {
        self.fetch_dept_ids(user_id).await
    }
}
