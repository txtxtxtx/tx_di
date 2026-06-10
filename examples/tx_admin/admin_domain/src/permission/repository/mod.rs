use std::collections::HashSet;
use async_trait::async_trait;
use tx_error::AppResult;
use crate::permission::model::value_object::PermissionCheck;

/// Permission repository trait
#[async_trait]
pub trait PermissionRepository: Send + Sync {
    /// Get all permission codes for given role IDs
    async fn find_by_role_ids(&self, role_ids: &[u64]) -> AppResult<HashSet<String>>;

    /// Get all permission codes for a user (via their roles)
    async fn find_by_user_id(&self, user_id: u64) -> AppResult<HashSet<String>>;

    /// Get all available permissions
    async fn find_all(&self) -> AppResult<HashSet<PermissionCheck>>;
}
