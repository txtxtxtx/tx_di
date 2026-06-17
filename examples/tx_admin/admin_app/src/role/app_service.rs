use std::sync::Arc;

use crate::role::dto::*;
use crate::user::dto::user_to_response;
use admin_domain::role::model::value_object::RoleQuery;
use admin_domain::role::service::RoleService;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

#[tx_comp]
pub struct RoleAppService {
    role_service: Arc<RoleService>,
}

impl RoleAppService {
    /// 创建角色应用服务实例
    ///
    /// # 参数
    /// * `role_service` - 角色领域服务，用于执行角色相关的业务逻辑
    pub fn new(role_service: Arc<RoleService>) -> Self {
        Self { role_service }
    }

    /// 创建新角色
    ///
    /// # 参数
    /// * `cmd` - 创建角色命令，包含角色名称、角色编码、排序号、菜单ID列表
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 调用角色领域服务创建角色（含名称、编码、排序号）
    /// 2. 若提供了菜单ID列表，则为角色分配菜单权限
    /// 3. 将领域模型转换为响应DTO返回
    ///
    /// # 返回
    /// 成功返回 `RoleResponse`，包含角色完整信息
    ///
    /// # 错误
    /// - `DuplicateRoleCode` - 角色编码已存在
    /// - 数据库写入异常
    pub async fn create_role(
        &self,
        cmd: CreateRoleCommand,
        creator: Option<String>,
    ) -> AppResult<RoleResponse> {
        let mut role = self
            .role_service
            .create_role(cmd.name, cmd.code, cmd.sort, creator)
            .await?;

        if let Some(menu_ids) = cmd.menu_ids {
            role = self.role_service.assign_menus(role.id, menu_ids).await?;
        }

        Ok(RoleResponse::from(role))
    }

    /// 更新角色信息
    ///
    /// # 参数
    /// * `cmd` - 更新角色命令，包含角色ID、名称、编码、排序号、数据范围、备注
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给角色领域服务执行更新操作，逻辑详见 `RoleService::update_role`
    ///
    /// # 返回
    /// 成功返回更新后的 `RoleResponse`
    ///
    /// # 错误
    /// - `NotFoundRole` - 角色ID对应的角色不存在
    /// - `DuplicateRoleCode` - 角色编码与其他角色冲突
    /// - 数据库更新异常
    pub async fn update_role(
        &self,
        cmd: UpdateRoleCommand,
        updater: Option<String>,
    ) -> AppResult<RoleResponse> {
        let role = self
            .role_service
            .update_role(cmd.role_id, cmd.name, cmd.code, cmd.sort, cmd.data_scope, cmd.remark, updater)
            .await?;
        Ok(RoleResponse::from(role))
    }

    /// 删除角色
    ///
    /// # 参数
    /// * `role_id` - 要删除的角色ID
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给角色领域服务执行删除操作，逻辑详见 `RoleService::delete_role`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundRole` - 角色ID对应的角色不存在
    /// - 数据库删除异常
    pub async fn delete_role(&self, role_id: u64, updater: Option<String>) -> AppResult<()> {
        self.role_service.delete_role(role_id, updater).await
    }

    /// 变更角色状态（启用/禁用）
    ///
    /// # 参数
    /// * `role_id` - 目标角色ID
    /// * `status` - 目标状态值（整数类型）
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给角色领域服务执行状态变更，逻辑详见 `RoleService::change_status`
    ///
    /// # 返回
    /// 成功返回变更后的 `RoleResponse`
    ///
    /// # 错误
    /// - `NotFoundRole` - 角色ID对应的角色不存在
    /// - 数据库更新异常
    pub async fn change_status(
        &self,
        role_id: u64,
        status: i32,
        updater: Option<String>,
    ) -> AppResult<RoleResponse> {
        let role = self.role_service.change_status(role_id, status, updater).await?;
        Ok(RoleResponse::from(role))
    }

    /// 为角色分配菜单权限
    ///
    /// # 参数
    /// * `cmd` - 分配菜单命令，包含角色ID和菜单ID列表
    ///
    /// # 执行逻辑
    /// 委托给角色领域服务执行菜单分配，逻辑详见 `RoleService::assign_menus`
    ///
    /// # 返回
    /// 成功返回更新后的 `RoleResponse`
    ///
    /// # 错误
    /// - `NotFoundRole` - 角色ID对应的角色不存在
    /// - 菜单ID不存在
    pub async fn assign_menus(&self, cmd: AssignMenusCommand) -> AppResult<RoleResponse> {
        let role = self.role_service.assign_menus(cmd.role_id, cmd.menu_ids).await?;
        Ok(RoleResponse::from(role))
    }

    /// 根据ID获取角色信息
    ///
    /// # 参数
    /// * `role_id` - 角色ID
    ///
    /// # 执行逻辑
    /// 委托给角色领域服务查询角色，逻辑详见 `RoleService::get_role`
    ///
    /// # 返回
    /// 成功返回 `RoleResponse`
    ///
    /// # 错误
    /// - `NotFoundRole` - 角色ID对应的角色不存在
    pub async fn get_role(&self, role_id: u64) -> AppResult<RoleResponse> {
        let role = self.role_service.get_role(role_id).await?;
        Ok(RoleResponse::from(role))
    }

    /// 分页查询角色列表
    ///
    /// # 参数
    /// * `request` - 分页查询请求，包含角色名称、角色编码、状态等筛选条件，以及页码和每页大小
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `RoleQuery`
    /// 2. 构建分页参数 `Page`
    /// 3. 委托给角色领域服务执行分页查询
    /// 4. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Page<RoleResponse>`，包含角色列表、页码、每页大小和总记录数
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_role_page(
        &self,
        request: RoleQueryRequest,
    ) -> AppResult<Page<RoleResponse>> {
        let query = RoleQuery {
            name: request.name,
            code: request.code,
            status: request.status,
        };
        let page = Page::request(request.page, request.size);
        let result = self.role_service.get_role_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(RoleResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 获取所有角色列表
    ///
    /// # 执行逻辑
    /// 使用默认查询条件（无筛选）调用角色领域服务获取全部角色
    ///
    /// # 返回
    /// 成功返回 `Vec<RoleResponse>`，包含所有角色列表
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_all_roles(&self) -> AppResult<Vec<RoleResponse>> {
        let roles = self.role_service.get_all_roles(&RoleQuery::default()).await?;
        Ok(roles.into_iter().map(RoleResponse::from).collect())
    }

    /// 获取角色关联的用户列表
    ///
    /// # 参数
    /// * `role_id` - 角色ID
    ///
    /// # 执行逻辑
    /// 委托给角色领域服务查询该角色下的所有用户，逻辑详见 `RoleService::get_role_users`
    ///
    /// # 返回
    /// 成功返回 `Vec<UserResponse>`，包含该角色关联的所有用户
    ///
    /// # 错误
    /// - `NotFoundRole` - 角色ID对应的角色不存在
    /// - 数据库查询异常
    pub async fn get_role_users(&self, role_id: u64) -> AppResult<Vec<crate::user::dto::UserResponse>> {
        let users = self.role_service.get_role_users(role_id).await?;
        Ok(users.into_iter().map(user_to_response).collect())
    }

    /// 将用户添加到角色
    ///
    /// # 参数
    /// * `role_id` - 目标角色ID
    /// * `user_ids` - 要添加的用户ID列表
    ///
    /// # 执行逻辑
    /// 委托给角色领域服务执行用户添加，逻辑详见 `RoleService::add_users_to_role`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundRole` - 角色ID对应的角色不存在
    /// - 用户ID不存在
    /// - 数据库写入异常
    pub async fn add_users_to_role(&self, role_id: u64, user_ids: Vec<u64>) -> AppResult<()> {
        self.role_service.add_users_to_role(role_id, user_ids).await
    }

    /// 从角色中移除用户
    ///
    /// # 参数
    /// * `role_id` - 目标角色ID
    /// * `user_ids` - 要移除的用户ID列表
    ///
    /// # 执行逻辑
    /// 委托给角色领域服务执行用户移除，逻辑详见 `RoleService::remove_users_from_role`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundRole` - 角色ID对应的角色不存在
    /// - 数据库删除异常
    pub async fn remove_users_from_role(&self, role_id: u64, user_ids: Vec<u64>) -> AppResult<()> {
        self.role_service.remove_users_from_role(role_id, user_ids).await
    }
}
