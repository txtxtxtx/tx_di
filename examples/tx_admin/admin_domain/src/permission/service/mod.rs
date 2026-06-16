use std::collections::HashSet;
use std::sync::Arc;
use tx_common::id;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use crate::permission::model::aggregate::Permission;
use crate::permission::model::value_object::{PermissionCheck, PermissionType};
use crate::permission::repository::PermissionRepository;
use crate::shared::repository::RepositoryError;

/// Permission domain service
#[tx_comp]
pub struct PermissionService {
    permission_repo: Arc<dyn PermissionRepository>,
}

impl PermissionService {
    pub fn new(permission_repo: Arc<dyn PermissionRepository>) -> Self {
        Self { permission_repo }
    }

    // === 原有查询方法 ===

    /// 获取指定用户的所有权限
    pub async fn get_user_permissions(
        &self,
        user_id: u64,
    ) -> AppResult<HashSet<String>> {
        self.permission_repo.find_by_user_id(user_id).await
    }

    /// Check if user has specific permission
    pub async fn check_permission(
        &self,
        user_id: u64,
        code: &str,
    ) -> AppResult<bool> {
        let permissions = self.permission_repo.find_by_user_id(user_id).await?;
        Ok(permissions.iter().any(|p| p == code))
    }

    /// Get permissions for role set
    pub async fn get_role_permissions(
        &self,
        role_ids: &[u64],
    ) -> AppResult<HashSet<String>> {
        self.permission_repo.find_by_role_ids(role_ids).await
    }

    /// Get all available permissions (lightweight)
    pub async fn get_all_permissions(
        &self,
    ) -> AppResult<HashSet<PermissionCheck>> {
        self.permission_repo.find_all().await
    }

    // === 新增 CRUD 方法 ===

    /// Create a new permission
    pub async fn create_permission(
        &self,
        name: String,
        permission_code: String,
        permission_type: PermissionType,
        parent_id: u64,
        sort: i32,
        description: Option<String>,
        creator: Option<String>,
    ) -> AppResult<Permission> {
        if self.permission_repo.exists_by_code(&permission_code).await? {
            return Err(RepositoryError::DuplicatePermCode)?;
        }

        let id = id::next_id();
        let permission = Permission::create(
            id,
            name,
            permission_code,
            permission_type,
            parent_id,
            sort,
            description,
            creator,
        );
        self.permission_repo.insert(&permission).await?;
        Ok(permission)
    }

    /// Update permission
    pub async fn update_permission(
        &self,
        permission_id: u64,
        name: String,
        permission_code: String,
        permission_type: PermissionType,
        parent_id: u64,
        sort: i32,
        description: Option<String>,
        updater: Option<String>,
    ) -> AppResult<Permission> {
        let mut permission = self
            .permission_repo
            .find_by_id(permission_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundPerm)?;

        // Check if code is taken by another permission
        if let Some(existing) = self.permission_repo.find_by_code(&permission_code).await? {
            if existing.id != permission_id {
                return Err(RepositoryError::DuplicatePermCode)?;
            }
        }

        permission.update_info(
            name,
            permission_code,
            permission_type,
            parent_id,
            sort,
            description,
            updater,
        );
        self.permission_repo.update(&permission).await?;
        Ok(permission)
    }

    /// Delete permission (soft delete)
    pub async fn delete_permission(
        &self,
        permission_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        let mut permission = self
            .permission_repo
            .find_by_id(permission_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundPerm)?;

        permission.soft_delete(updater);
        self.permission_repo.update(&permission).await?;
        Ok(())
    }

    /// Get permission by ID
    pub async fn get_permission(&self, permission_id: u64) -> AppResult<Permission> {
        Ok(self.permission_repo
            .find_by_id(permission_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundPerm)?)
    }

    /// Get all permissions (full entities)
    pub async fn get_all_permission_details(&self) -> AppResult<Vec<Permission>> {
        self.permission_repo.find_all_permissions().await
    }
}

#[cfg(test)]
mod tests;
