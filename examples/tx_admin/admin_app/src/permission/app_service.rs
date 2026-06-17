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
    /// 创建权限应用服务实例
    ///
    /// # 参数
    /// * `permission_service` - 权限领域服务，用于执行权限相关的业务逻辑
    pub fn new(permission_service: Arc<PermissionService>) -> Self {
        Self { permission_service }
    }

    // === 原有查询方法 ===

    /// 检查用户是否拥有指定权限
    ///
    /// # 参数
    /// * `request` - 权限检查请求，包含用户ID和权限编码
    ///
    /// # 执行逻辑
    /// 委托给权限领域服务执行权限检查，逻辑详见 `PermissionService::check_permission`
    ///
    /// # 返回
    /// 成功返回 `PermissionCheckResponse`，包含 `has_permission` 布尔值
    ///
    /// # 错误
    /// - 数据库查询异常
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
    ///
    /// # 参数
    /// * `user_id` - 用户ID
    ///
    /// # 执行逻辑
    /// 委托给权限领域服务查询用户权限集合，逻辑详见 `PermissionService::get_user_permissions`
    ///
    /// # 返回
    /// 成功返回 `UserPermissionsResponse`，包含用户ID和权限编码集合
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_user_permissions(
        &self,
        user_id: u64,
    ) -> AppResult<UserPermissionsResponse> {
        let permissions = self.permission_service.get_user_permissions(user_id).await?;
        Ok(UserPermissionsResponse { user_id, permissions: permissions.into_iter().collect() })
    }

    /// 获取用户权限及详情
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    ///
    /// # 执行逻辑
    /// 1. 调用领域服务获取用户的权限编码集合
    /// 2. 按编码批量查询权限详情（name、type 等）
    /// 3. 构建 `PermissionItem` 列表
    ///
    /// # 返回
    /// 成功返回 `(权限编码列表, 权限详情列表)` 元组
    pub async fn get_user_permission_items(
        &self,
        user_id: u64,
    ) -> AppResult<(Vec<String>, Vec<PermissionItem>)> {
        let permissions_set = self.permission_service.get_user_permissions(user_id).await?;
        let permissions: Vec<String> = permissions_set.into_iter().collect();

        let details = self.permission_service.get_permissions_by_codes(&permissions).await?;
        let items: Vec<PermissionItem> = details
            .into_iter()
            .map(|p| PermissionItem {
                code: p.permission_code,
                name: p.name,
                permission_type: format!("{:?}", p.permission_type),
            })
            .collect();

        Ok((permissions, items))
    }

    /// 获取所有可用权限（轻量级）
    ///
    /// # 执行逻辑
    /// 委托给权限领域服务查询所有权限，仅返回权限编码和名称等轻量信息
    ///
    /// # 返回
    /// 成功返回 `Vec<PermissionItem>`，包含所有权限的轻量级信息
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_all_permissions(&self) -> AppResult<Vec<PermissionItem>> {
        let permissions = self.permission_service.get_all_permissions().await?;
        Ok(permissions.into_iter().map(PermissionItem::from).collect())
    }

    // === 新增 CRUD 方法 ===

    /// 创建新权限
    ///
    /// # 参数
    /// * `cmd` - 创建权限命令，包含权限名称、权限编码、权限类型、父权限ID、排序号、描述
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 将整数权限类型转换为 `PermissionType` 枚举
    /// 2. 委托给权限领域服务执行创建操作
    ///
    /// # 返回
    /// 成功返回 `PermissionResponse`，包含权限完整信息
    ///
    /// # 错误
    /// - `DuplicatePermissionCode` - 权限编码已存在
    /// - 数据库写入异常
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

    /// 更新权限信息
    ///
    /// # 参数
    /// * `cmd` - 更新权限命令，包含权限ID、名称、权限编码、权限类型、父权限ID、排序号、描述
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 将整数权限类型转换为 `PermissionType` 枚举
    /// 2. 委托给权限领域服务执行更新操作
    ///
    /// # 返回
    /// 成功返回更新后的 `PermissionResponse`
    ///
    /// # 错误
    /// - `NotFoundPermission` - 权限ID对应的权限不存在
    /// - `DuplicatePermissionCode` - 权限编码与其他权限冲突
    /// - 数据库更新异常
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

    /// 删除权限（软删除）
    ///
    /// # 参数
    /// * `permission_id` - 要删除的权限ID
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给权限领域服务执行软删除操作，逻辑详见 `PermissionService::delete_permission`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundPermission` - 权限ID对应的权限不存在
    /// - 数据库更新异常
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
    ///
    /// # 参数
    /// * `permission_id` - 权限ID
    ///
    /// # 执行逻辑
    /// 委托给权限领域服务查询权限，逻辑详见 `PermissionService::get_permission`
    ///
    /// # 返回
    /// 成功返回 `PermissionResponse`
    ///
    /// # 错误
    /// - `NotFoundPermission` - 权限ID对应的权限不存在
    pub async fn get_permission(&self, permission_id: u64) -> AppResult<PermissionResponse> {
        let permission = self.permission_service.get_permission(permission_id).await?;
        Ok(PermissionResponse::from(permission))
    }

    /// 获取所有权限列表（完整实体）
    ///
    /// # 执行逻辑
    /// 委托给权限领域服务查询所有权限的完整详细信息
    ///
    /// # 返回
    /// 成功返回 `Vec<PermissionResponse>`，包含所有权限的完整信息列表
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_permission_list(&self) -> AppResult<Vec<PermissionResponse>> {
        let permissions = self.permission_service.get_all_permission_details().await?;
        Ok(permissions.into_iter().map(PermissionResponse::from).collect())
    }
}
