use std::sync::Arc;

use crate::permission::dto::permission_to_detail;
use admin_domain::permission::model::value_object::PermissionType;
use admin_domain::permission::service::PermissionService;
use admin_proto::{
    PermissionCheckRequest, PermissionCheckResponse,
    UserPermissionsResponse, UserPermissionItem,
    CreatePermissionRequest, UpdatePermissionRequest, PermissionDetail,
};
use tx_di_core::tx_comp;
use tx_error::AppResult;

#[tx_comp]
pub struct PermissionAppService {
    permission_service: Arc<PermissionService>,
}

impl PermissionAppService {
    /// 创建权限应用服务实例
    ///
    /// # 参数
    /// * `permission_service` - 权限领域服务，用于执行权限相关的业务逻辑
    pub fn new(permission_service: Arc<PermissionService>) -> Self {
        Self { permission_service }
    }

    // === 原有查询方法 ===

    /// 检查用户是否拥有指定权限
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

    /// 获取用户拥有的所有权限
    pub async fn get_user_permissions(
        &self,
        user_id: u64,
    ) -> AppResult<UserPermissionsResponse> {
        let permissions_set = self.permission_service.get_user_permissions(user_id).await?;
        let permissions: Vec<String> = permissions_set.into_iter().collect();

        let details = self.permission_service.get_permissions_by_codes(&permissions).await?;
        let items: Vec<UserPermissionItem> = details
            .into_iter()
            .map(|p| UserPermissionItem {
                code: p.permission_code,
                name: p.name,
                permission_type: format!("{:?}", p.permission_type),
            })
            .collect();

        Ok(UserPermissionsResponse {
            user_id,
            permissions,
            items,
        })
    }

    /// 获取用户权限及详情
    ///
    /// # 返回
    /// 成功返回 `(权限编码列表, 权限详情列表)` 元组
    pub async fn get_user_permission_items(
        &self,
        user_id: u64,
    ) -> AppResult<(Vec<String>, Vec<UserPermissionItem>)> {
        let permissions_set = self.permission_service.get_user_permissions(user_id).await?;
        let permissions: Vec<String> = permissions_set.into_iter().collect();

        let details = self.permission_service.get_permissions_by_codes(&permissions).await?;
        let items: Vec<UserPermissionItem> = details
            .into_iter()
            .map(|p| UserPermissionItem {
                code: p.permission_code,
                name: p.name,
                permission_type: format!("{:?}", p.permission_type),
            })
            .collect();

        Ok((permissions, items))
    }

    /// 获取所有可用权限（轻量级）
    pub async fn get_all_permissions(&self) -> AppResult<Vec<UserPermissionItem>> {
        let permissions = self.permission_service.get_all_permissions().await?;
        Ok(permissions.into_iter().map(crate::permission::dto::permission_check_to_item).collect())
    }

    // === CRUD 方法 ===

    /// 创建新权限
    pub async fn create_permission(
        &self,
        req: CreatePermissionRequest,
        creator: Option<String>,
    ) -> AppResult<PermissionDetail> {
        let permission_type = PermissionType::from(req.r#type);
        let permission = self
            .permission_service
            .create_permission(
                req.name,
                req.permission_code,
                permission_type,
                req.parent_id,
                req.sort,
                if req.description.is_empty() { None } else { Some(req.description) },
                creator,
            )
            .await?;
        Ok(permission_to_detail(permission))
    }

    /// 更新权限信息
    pub async fn update_permission(
        &self,
        req: UpdatePermissionRequest,
        updater: Option<String>,
    ) -> AppResult<PermissionDetail> {
        let permission_type = PermissionType::from(req.r#type);
        let permission = self
            .permission_service
            .update_permission(
                req.id,
                req.name,
                req.permission_code,
                permission_type,
                req.parent_id,
                req.sort,
                if req.description.is_empty() { None } else { Some(req.description) },
                updater,
            )
            .await?;
        Ok(permission_to_detail(permission))
    }

    /// 删除权限（软删除）
    pub async fn delete_permission(
        &self,
        permission_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.permission_service
            .delete_permission(permission_id, updater)
            .await
    }

    /// 根据ID获取权限信息
    pub async fn get_permission(&self, permission_id: u64) -> AppResult<PermissionDetail> {
        let permission = self.permission_service.get_permission(permission_id).await?;
        Ok(permission_to_detail(permission))
    }

    /// 获取所有权限列表（完整实体）
    pub async fn get_permission_list(&self) -> AppResult<Vec<PermissionDetail>> {
        let permissions = self.permission_service.get_all_permission_details().await?;
        Ok(permissions.into_iter().map(permission_to_detail).collect())
    }
}
