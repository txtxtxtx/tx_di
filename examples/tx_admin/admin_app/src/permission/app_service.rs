use std::sync::Arc;

use crate::permission::dto::*;
use admin_domain::permission::model::value_object::PermissionType;
use admin_domain::permission::service::PermissionService;
use tx_di_core::tx_comp;
use tx_error::AppResult;

#[tx_comp]
pub struct PermissionAppService {
    permission_service: Arc<PermissionService>,
}

impl PermissionAppService {
    pub fn new(permission_service: Arc<PermissionService>) -> Self {
        Self { permission_service }
    }

    // === 原有查询方法 ===

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

    /// Get all available permissions (lightweight)
    pub async fn get_all_permissions(&self) -> AppResult<Vec<PermissionItem>> {
        let permissions = self.permission_service.get_all_permissions().await?;
        Ok(permissions.into_iter().map(PermissionItem::from).collect())
    }

    // === 新增 CRUD 方法 ===

    /// Create a new permission
    pub async fn create_permission(
        &self,
        cmd: CreatePermissionCommand,
        creator: Option<String>,
    ) -> AppResult<PermissionResponse> {
        let permission_type = PermissionType::from(cmd.permission_type);
        let permission = self
            .permission_service
            .create_permission(
                cmd.name,
                cmd.permission_code,
                permission_type,
                cmd.parent_id,
                cmd.sort,
                cmd.description,
                creator,
            )
            .await?;
        Ok(PermissionResponse::from(permission))
    }

    /// Update permission
    pub async fn update_permission(
        &self,
        cmd: UpdatePermissionCommand,
        updater: Option<String>,
    ) -> AppResult<PermissionResponse> {
        let permission_type = PermissionType::from(cmd.permission_type);
        let permission = self
            .permission_service
            .update_permission(
                cmd.id,
                cmd.name,
                cmd.permission_code,
                permission_type,
                cmd.parent_id,
                cmd.sort,
                cmd.description,
                updater,
            )
            .await?;
        Ok(PermissionResponse::from(permission))
    }

    /// Delete permission (soft delete)
    pub async fn delete_permission(
        &self,
        permission_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.permission_service
            .delete_permission(permission_id, updater)
            .await
    }

    /// Get permission by ID
    pub async fn get_permission(&self, permission_id: u64) -> AppResult<PermissionResponse> {
        let permission = self.permission_service.get_permission(permission_id).await?;
        Ok(PermissionResponse::from(permission))
    }

    /// Get all permissions (full entities)
    pub async fn get_permission_list(&self) -> AppResult<Vec<PermissionResponse>> {
        let permissions = self.permission_service.get_all_permission_details().await?;
        Ok(permissions.into_iter().map(PermissionResponse::from).collect())
    }
}
