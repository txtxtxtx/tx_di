use std::sync::Arc;

use crate::permission::dto::*;
use admin_domain::permission::service::PermissionService;
use tx_error::AppResult;

pub struct PermissionAppService {
    permission_service: Arc<PermissionService>,
}

impl PermissionAppService {
    pub fn new(permission_service: Arc<PermissionService>) -> Self {
        Self { permission_service }
    }

    /// Check if user has specific permission
    pub async fn check_permission(
        &self,
        request: PermissionCheckRequest,
    ) -> AppResult<PermissionCheckResponse> {
        let has_permission = self
            .permission_service
            .check_permission(request.user_id, &request.permission)
            .await?;
        Ok(PermissionCheckResponse { has_permission })
    }

    /// Get all permissions for a user
    pub async fn get_user_permissions(
        &self,
        user_id: u64,
    ) -> AppResult<UserPermissionsResponse> {
        let permissions = self.permission_service.get_user_permissions(user_id).await?;
        Ok(UserPermissionsResponse { user_id, permissions: permissions.into_iter().collect() })
    }

    /// Get all available permissions
    pub async fn get_all_permissions(&self) -> AppResult<Vec<PermissionItem>> {
        let permissions = self.permission_service.get_all_permissions().await?;
        Ok(permissions.into_iter().map(PermissionItem::from).collect())
    }
}
