use std::any::Any;
use std::collections::HashSet;
use async_trait::async_trait;
use tx_error::AppResult;
use crate::permission::model::aggregate::Permission;
use crate::permission::model::value_object::PermissionCheck;

/// Permission repository trait
#[async_trait]
pub trait PermissionRepository: Any + Send + Sync {
    // === 原有查询方法 ===

    /// Get all permission codes for given role IDs
    async fn find_by_role_ids(&self, role_ids: &[u64]) -> AppResult<HashSet<String>>;

    /// Get all permission codes for a user (via their roles)
    async fn find_by_user_id(&self, user_id: u64) -> AppResult<HashSet<String>>;

    /// Get all available permissions (lightweight check items)
    async fn find_all(&self) -> AppResult<HashSet<PermissionCheck>>;

    // === 新增 CRUD 方法 ===

    /// Find permission by ID
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Permission>>;

    /// Find permission by code
    async fn find_by_code(&self, code: &str) -> AppResult<Option<Permission>>;

    /// Get all permissions (full entities, excluding soft-deleted)
    async fn find_all_permissions(&self) -> AppResult<Vec<Permission>>;

    /// Insert a new permission
    async fn insert(&self, permission: &Permission) -> AppResult<()>;

    /// Update an existing permission
    async fn update(&self, permission: &Permission) -> AppResult<()>;

    /// Soft delete a permission by ID
    async fn soft_delete(&self, id: u64) -> AppResult<()>;

    /// Check if a permission code already exists
    async fn exists_by_code(&self, code: &str) -> AppResult<bool>;
}
