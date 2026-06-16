use std::sync::Arc;
use tx_common::id;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::role::model::aggregate::Role;
use crate::role::model::value_object::RoleQuery;
use crate::role::repository::RoleRepository;
use crate::user::repository::UserRepository;
use crate::user::model::value_object::UserStatus;

/// Role domain service
#[tx_comp]
pub struct RoleService {
    role_repo: Arc<dyn RoleRepository>,
    user_repo: Arc<dyn UserRepository>,
}

impl RoleService {
    pub fn new(role_repo: Arc<dyn RoleRepository>, user_repo: Arc<dyn UserRepository>) -> Self {
        Self { role_repo, user_repo }
    }

    /// Create a new role
    pub async fn create_role(
        &self,
        name: String,
        code: String,
        sort: i32,
        creator: Option<String>,
    ) -> AppResult<Role> {
        if self.role_repo.exists_by_code(&code).await? {
            return Err(RepositoryError::DuplicateRoleCode)?;
        }

        let role_id = id::next_id();
        let role = Role::create(role_id, name, code, sort, creator);
        self.role_repo.insert(&role).await?;
        Ok(role)
    }

    /// Update role
    pub async fn update_role(
        &self,
        role_id: u64,
        name: String,
        code: String,
        sort: i32,
        data_scope: i32,
        remark: Option<String>,
        updater: Option<String>,
    ) -> AppResult<Role> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?;

        // Check if code is taken by another role
        if let Some(existing) = self.role_repo.find_by_code(&code).await? {
            if existing.id != role_id {
                return Err(RepositoryError::DuplicateRoleCode)?;
            }
        }

        role.update_info(name, code, sort, data_scope, remark, updater);
        self.role_repo.update(&role).await?;
        Ok(role)
    }

    /// Delete role
    pub async fn delete_role(
        &self,
        role_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?;

        role.soft_delete(updater);
        self.role_repo.update(&role).await?;
        Ok(())
    }

    /// Change role status
    pub async fn change_status(
        &self,
        role_id: u64,
        status: i32,
        updater: Option<String>,
    ) -> AppResult<Role> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?;

        role.change_status(status, updater);
        self.role_repo.update(&role).await?;
        Ok(role)
    }

    /// Assign menu permissions to role
    ///
    /// 校验：角色必须为启用状态（status == 0）
    pub async fn assign_menus(
        &self,
        role_id: u64,
        menu_ids: Vec<u64>,
    ) -> AppResult<Role> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?;

        // 角色必须为启用状态才能分配菜单
        if role.status != 0 {
            return Err(RepositoryError::ValidationRoleDisabled)?;
        }

        role.set_menus(menu_ids.clone());
        self.role_repo.bind_menus(role_id, &menu_ids).await?;
        self.role_repo.update(&role).await?;
        Ok(role)
    }

    /// Get role page
    pub async fn get_role_page(
        &self,
        query: &RoleQuery,
        page: Page<Role>,
    ) -> AppResult<Page<Role>> {
        self.role_repo.find_page(query, page).await
    }

    /// Get role by ID
    pub async fn get_role(&self, role_id: u64) -> AppResult<Role> {
        Ok(self.role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?)
    }

    /// Get roles by IDs
    pub async fn get_roles_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Role>> {
        self.role_repo.find_by_ids(ids).await
    }

    /// Get all roles
    pub async fn get_all_roles(&self, query: &RoleQuery) -> AppResult<Vec<Role>> {
        self.role_repo.find_all(query).await
    }

    /// Get users associated with a role
    pub async fn get_role_users(&self, role_id: u64) -> AppResult<Vec<crate::user::model::aggregate::User>> {
        // Verify role exists
        let _role = self.role_repo.find_by_id(role_id).await?.ok_or_else(|| RepositoryError::NotFoundRole)?;
        self.role_repo.find_users_by_role_id(role_id).await
    }

    /// Add users to a role
    ///
    /// 校验：角色必须为启用状态，且每个用户必须存在且为 Active 状态
    pub async fn add_users_to_role(&self, role_id: u64, user_ids: Vec<u64>) -> AppResult<()> {
        let role = self.role_repo.find_by_id(role_id).await?.ok_or_else(|| RepositoryError::NotFoundRole)?;

        // 角色必须为启用状态才能添加用户
        if role.status != 0 {
            return Err(RepositoryError::ValidationRoleDisabled)?;
        }

        // 校验每个用户存在且为 Active 状态
        for &uid in &user_ids {
            if let Some(user) = self.user_repo.find_by_id(uid).await? {
                if user.status != UserStatus::Active {
                    return Err(RepositoryError::ValidationRoleDisabled)?;
                }
            } else {
                return Err(RepositoryError::NotFoundRole)?;
            }
        }

        self.role_repo.bind_users(role_id, &user_ids).await
    }

    /// Remove users from a role
    pub async fn remove_users_from_role(&self, role_id: u64, user_ids: Vec<u64>) -> AppResult<()> {
        // Verify role exists
        let _role = self.role_repo.find_by_id(role_id).await?.ok_or_else(|| RepositoryError::NotFoundRole)?;
        self.role_repo.unbind_users(role_id, &user_ids).await
    }
}

#[cfg(test)]
mod tests;
