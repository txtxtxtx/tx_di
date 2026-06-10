use std::sync::Arc;
use tx_common::id;
use tx_common::page::Page;
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::role::model::aggregate::Role;
use crate::role::model::value_object::RoleQuery;
use crate::role::repository::RoleRepository;
use crate::shared::repository::RepositoryError::NotFound;

/// Role domain service
pub struct RoleService {
    role_repo: Arc<dyn RoleRepository>,
}

impl RoleService {
    pub fn new(role_repo: Arc<dyn RoleRepository>) -> Self {
        Self { role_repo }
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
            return Err(RepositoryError::Duplicate)?;
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
            .ok_or_else(|| NotFound)?;

        // Check if code is taken by another role
        if let Some(existing) = self.role_repo.find_by_code(&code).await? {
            if existing.id != role_id {
                return Err(RepositoryError::Duplicate)?;
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
            .ok_or_else(|| NotFound)?;

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
            .ok_or_else(|| NotFound)?;

        role.change_status(status, updater);
        self.role_repo.update(&role).await?;
        Ok(role)
    }

    /// Assign menu permissions to role
    pub async fn assign_menus(
        &self,
        role_id: u64,
        menu_ids: Vec<u64>,
    ) -> AppResult<Role> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| NotFound)?;

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
            .ok_or_else(|| NotFound)?)
    }

    /// Get roles by IDs
    pub async fn get_roles_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Role>> {
        self.role_repo.find_by_ids(ids).await
    }

    /// Get all roles
    pub async fn get_all_roles(&self, query: &RoleQuery) -> AppResult<Vec<Role>> {
        self.role_repo.find_all(query).await
    }
}
