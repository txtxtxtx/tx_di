use std::sync::Arc;

use crate::role::dto::*;
use crate::user::dto::user_to_response;
use admin_domain::role::model::value_object::RoleQuery;
use admin_domain::role::service::RoleService;
use admin_domain::user::repository::UserRepository;
use admin_domain::user::model::value_object::UserStatus;
use admin_domain::shared::repository::RepositoryError;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

/// Role application service - 编排领域操作 + 跨聚合校验
#[tx_comp]
pub struct RoleAppService {
    role_service: Arc<RoleService>,
    user_repo: Arc<dyn UserRepository>,
}

impl RoleAppService {
    pub fn new(
        role_service: Arc<RoleService>,
        user_repo: Arc<dyn UserRepository>,
    ) -> Self {
        Self { role_service, user_repo }
    }

    /// 创建新角色
    pub async fn create_role(
        &self,
        req: CreateRoleRequest,
        creator: Option<String>,
    ) -> AppResult<RoleResponse> {
        let mut role = self
            .role_service
            .create_role(req.name, req.code, req.sort, creator)
            .await?;

        if !req.menu_ids.is_empty() {
            role = self.role_service.assign_menus(role.id, req.menu_ids).await?;
        }

        Ok(role_to_response(role))
    }

    /// 更新角色信息
    pub async fn update_role(
        &self,
        req: UpdateRoleRequest,
        updater: Option<String>,
    ) -> AppResult<RoleResponse> {
        let role = self
            .role_service
            .update_role(req.role_id, req.name, req.code, req.sort, req.data_scope, req.remark, updater)
            .await?;
        Ok(role_to_response(role))
    }

    /// 删除角色
    pub async fn delete_role(&self, role_id: u64, updater: Option<String>) -> AppResult<()> {
        self.role_service.delete_role(role_id, updater).await
    }

    /// 变更角色状态
    pub async fn change_status(
        &self,
        role_id: u64,
        status: i32,
        updater: Option<String>,
    ) -> AppResult<RoleResponse> {
        let role = self.role_service.change_status(role_id, status, updater).await?;
        Ok(role_to_response(role))
    }

    /// 为角色分配菜单权限（跨聚合操作）
    pub async fn assign_menus(&self, role_id: u64, menu_ids: Vec<u64>) -> AppResult<RoleResponse> {
        let role = self.role_service.assign_menus(role_id, menu_ids).await?;
        Ok(role_to_response(role))
    }

    /// 根据ID获取角色信息
    pub async fn get_role(&self, role_id: u64) -> AppResult<RoleResponse> {
        let role = self.role_service.get_role(role_id).await?;
        Ok(role_to_response(role))
    }

    /// 分页查询角色列表
    pub async fn get_role_page(
        &self,
        request: ListRolesRequest,
    ) -> AppResult<Page<RoleResponse>> {
        let query = RoleQuery {
            name: request.name,
            code: request.code,
            status: request.status,
        };
        let page = Page::request(request.page, request.page_size);
        let result = self.role_service.get_role_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(role_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 获取所有角色列表
    pub async fn get_all_roles(&self) -> AppResult<Vec<RoleResponse>> {
        let roles = self.role_service.get_all_roles(&RoleQuery::default()).await?;
        Ok(roles.into_iter().map(role_to_response).collect())
    }

    /// 获取角色关联的用户列表
    pub async fn get_role_users(&self, role_id: u64) -> AppResult<Vec<crate::user::dto::UserResponse>> {
        let users = self.role_service.get_role_users(role_id).await?;
        Ok(users.into_iter().map(user_to_response).collect())
    }

    /// 将用户添加到角色（跨聚合校验）
    ///
    /// # 执行逻辑
    /// 1. 校验每个用户存在且状态为 Active（跨聚合）
    /// 2. 委托 RoleService 校验角色 + 绑定关联
    pub async fn add_users_to_role(&self, role_id: u64, user_ids: Vec<u64>) -> AppResult<()> {
        // 跨聚合校验：每个用户必须存在且为 Active 状态
        for &uid in &user_ids {
            let user = self.user_repo.find_by_id(uid)
                .await?
                .ok_or(RepositoryError::NotFoundUser)?;
            if user.status != UserStatus::Active {
                return Err(RepositoryError::ValidationUserStatus)?;
            }
        }

        self.role_service.add_users_to_role(role_id, user_ids).await
    }

    /// 从角色中移除用户
    pub async fn remove_users_from_role(&self, role_id: u64, user_ids: Vec<u64>) -> AppResult<()> {
        self.role_service.remove_users_from_role(role_id, user_ids).await
    }
}
