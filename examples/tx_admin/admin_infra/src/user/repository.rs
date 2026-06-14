use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::shared::model::value_object::{DeletedStatus, TenantId};
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::RepositoryError;
use admin_domain::user::model::aggregate::User;
use admin_domain::user::model::value_object::{Sex, UserQuery, UserStatus};
use admin_domain::user::repository::UserRepository;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::{SysUser, SysUserDept, SysUserRole};

/// Toasty 实现的 UserRepository
#[tx_comp(as_trait = dyn UserRepository)]
pub struct ToastyUserRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyUserRepository {
    /// 将 toasty SysUser 转换为 domain User
    fn to_domain(u: &SysUser, role_ids: Vec<u64>, dept_ids: Vec<u64>) -> User {
        User::restore(
            u.id as u64,
            u.username.clone(),
            u.password_hash.clone(),
            u.nickname.clone(),
            if u.remark.is_empty() { None } else { Some(u.remark.clone()) },
            if u.email.is_empty() { None } else { Some(u.email.clone()) },
            if u.mobile.is_empty() { None } else { Some(u.mobile.clone()) },
            Sex::from(u.sex),
            if u.avatar.is_empty() { None } else { Some(u.avatar.clone()) },
            UserStatus::from(u.status),
            if u.login_ip.is_empty() { None } else { Some(u.login_ip.clone()) },
            if u.login_date.is_empty() { None } else { u.login_date.parse().ok() },
            TenantId::new(u.tenant_id as u64),
            AuditFields {
                creator: if u.creator.is_empty() { None } else { Some(u.creator.clone()) },
                create_time: u.created_at.parse().unwrap_or_default(),
                updater: if u.updater.is_empty() { None } else { Some(u.updater.clone()) },
                update_time: u.updated_at.parse().unwrap_or_default(),
                deleted: if u.deleted == 1 { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
            role_ids,
            dept_ids,
        )
    }

    /// 获取用户的角色ID列表
    async fn fetch_role_ids(&self, user_id: i64) -> AppResult<Vec<u64>> {
        let mut db = self.plugin.db().clone();
        let roles = SysUserRole::filter_by_user_id(user_id)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(roles.into_iter().map(|r| r.role_id as u64).collect())
    }

    /// 获取用户的部门ID列表
    async fn fetch_dept_ids(&self, user_id: i64) -> AppResult<Vec<u64>> {
        let mut db = self.plugin.db().clone();
        let depts = SysUserDept::filter_by_user_id(user_id)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(depts.into_iter().map(|d| d.dept_id as u64).collect())
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
        match SysUser::get_by_id(&mut db, id as i64).await {
            Ok(u) if u.deleted == 0 => Ok(Some(self.to_full_domain(&u).await?)),
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
            .map_err(|_| RepositoryError::Database)?;
        match user {
            Some(u) if u.deleted == 0 => Ok(Some(self.to_full_domain(&u).await?)),
            _ => Ok(None),
        }
    }

    async fn find_page(&self, query: &UserQuery, page: Page<User>) -> AppResult<Page<User>> {
        let mut db = self.plugin.db().clone();
        let all = SysUser::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        let filtered: Vec<SysUser> = all
            .into_iter()
            .filter(|u| u.deleted == 0)
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
                    if u.status != status as i32 { return false; }
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
            .map_err(|_| RepositoryError::Database)?;

        let mut users = Vec::new();
        for u in all.into_iter().filter(|u| u.deleted == 0) {
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
                if u.status != status as i32 { continue; }
            }
            users.push(self.to_full_domain(&u).await?);
        }
        Ok(users)
    }

    async fn insert(&self, user: &User) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        let model = SysUser::create()
            .username(user.username.clone())
            .password_hash(user.password.clone())
            .nickname(user.nickname.clone())
            .remark(user.remark.clone().unwrap_or_default())
            .email(user.email.clone().unwrap_or_default())
            .mobile(user.mobile.clone().unwrap_or_default())
            .sex(user.sex as i32)
            .avatar(user.avatar.clone().unwrap_or_default())
            .status(user.status as i32)
            .login_ip(user.login_ip.clone().unwrap_or_default())
            .login_date(user.login_date.map(|t| t.to_string()).unwrap_or_default())
            .tenant_id(user.tenant_id.into_inner() as i64)
            .creator(user.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(user.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(user.audit.deleted as i32)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        // 插入角色关联
        for &role_id in &user.role_ids {
            SysUserRole::create()
                .user_id(model.id)
                .role_id(role_id as i64)
                .exec(&mut db)
                .await
                .map_err(|_| RepositoryError::Database)?;
        }

        // 插入部门关联
        for &dept_id in &user.dept_ids {
            SysUserDept::create()
                .user_id(model.id)
                .dept_id(dept_id as i64)
                .exec(&mut db)
                .await
                .map_err(|_| RepositoryError::Database)?;
        }

        Ok(())
    }

    async fn update(&self, user: &User) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysUser::get_by_id(&mut db, user.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .username(user.username.clone())
            .password_hash(user.password.clone())
            .nickname(user.nickname.clone())
            .remark(user.remark.clone().unwrap_or_default())
            .email(user.email.clone().unwrap_or_default())
            .mobile(user.mobile.clone().unwrap_or_default())
            .sex(user.sex as i32)
            .avatar(user.avatar.clone().unwrap_or_default())
            .status(user.status as i32)
            .login_ip(user.login_ip.clone().unwrap_or_default())
            .login_date(user.login_date.map(|t| t.to_string()).unwrap_or_default())
            .tenant_id(user.tenant_id.into_inner() as i64)
            .updater(user.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(user.audit.deleted as i32)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut user = SysUser::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        user.update()
            .deleted(1)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(())
    }

    async fn exists_by_username(&self, username: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let user = SysUser::filter_by_username(username)
            .first()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(user.map(|u| u.deleted == 0).unwrap_or(false))
    }

    async fn exists_by_email(&self, email: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let all = SysUser::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(all.iter().any(|u| u.deleted == 0 && u.email == email))
    }

    async fn exists_by_mobile(&self, mobile: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let all = SysUser::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(all.iter().any(|u| u.deleted == 0 && u.mobile == mobile))
    }

    async fn count(&self, query: &UserQuery) -> AppResult<i64> {
        let mut db = self.plugin.db().clone();
        let all = SysUser::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        let count = all
            .iter()
            .filter(|u| u.deleted == 0)
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
                    if u.status != status as i32 { return false; }
                }
                true
            })
            .count();

        Ok(count as i64)
    }

    async fn find_by_role_id(&self, role_id: u64) -> AppResult<Vec<User>> {
        let mut db = self.plugin.db().clone();
        let user_roles = SysUserRole::filter_by_role_id(role_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        let mut users = Vec::new();
        for ur in user_roles {
            if let Ok(u) = SysUser::get_by_id(&mut db, ur.user_id).await {
                if u.deleted == 0 {
                    users.push(self.to_full_domain(&u).await?);
                }
            }
        }
        Ok(users)
    }

    async fn find_by_dept_id(&self, dept_id: u64) -> AppResult<Vec<User>> {
        let mut db = self.plugin.db().clone();
        let user_depts = SysUserDept::filter_by_dept_id(dept_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        let mut users = Vec::new();
        for ud in user_depts {
            if let Ok(u) = SysUser::get_by_id(&mut db, ud.user_id).await {
                if u.deleted == 0 {
                    users.push(self.to_full_domain(&u).await?);
                }
            }
        }
        Ok(users)
    }

    async fn bind_roles(&self, user_id: u64, role_ids: &[u64]) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        // 先删除旧的关联
        let old = SysUserRole::filter_by_user_id(user_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        for ur in old {
            ur.delete().exec(&mut db)
                .await
                .map_err(|_| RepositoryError::Database)?;
        }

        // 插入新的关联
        for &role_id in role_ids {
            SysUserRole::create()
                .user_id(user_id as i64)
                .role_id(role_id as i64)
                .exec(&mut db)
                .await
                .map_err(|_| RepositoryError::Database)?;
        }

        Ok(())
    }

    async fn bind_departments(&self, user_id: u64, dept_ids: &[u64]) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        // 先删除旧的关联
        let old = SysUserDept::filter_by_user_id(user_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        for ud in old {
            ud.delete().exec(&mut db)
                .await
                .map_err(|_| RepositoryError::Database)?;
        }

        // 插入新的关联
        for &dept_id in dept_ids {
            SysUserDept::create()
                .user_id(user_id as i64)
                .dept_id(dept_id as i64)
                .exec(&mut db)
                .await
                .map_err(|_| RepositoryError::Database)?;
        }

        Ok(())
    }

    async fn get_role_ids(&self, user_id: u64) -> AppResult<Vec<u64>> {
        self.fetch_role_ids(user_id as i64).await
    }

    async fn get_dept_ids(&self, user_id: u64) -> AppResult<Vec<u64>> {
        self.fetch_dept_ids(user_id as i64).await
    }
}
