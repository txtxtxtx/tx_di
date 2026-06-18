use std::sync::Arc;

use crate::user::dto::*;
use admin_proto::{
    CreateUserRequest, UpdateUserRequest, ChangePasswordRequest,
    AssignRolesRequest, AssignDeptsRequest, ListUsersRequest,
};
use admin_domain::user::model::value_object::{Sex, UserQuery, UserStatus};
use admin_domain::user::service::UserService;
use admin_domain::shared::repository::RepositoryError;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

/// User application service - orchestrates domain operations
#[tx_comp]
pub struct UserAppService {
    user_service: Arc<UserService>,
}

impl UserAppService {
    /// 创建用户应用服务实例
    pub fn new(user_service: Arc<UserService>) -> Self {
        Self { user_service }
    }

    /// 创建新用户
    ///
    /// proto3 的 `repeated` 字段无法区分"未设置"和"空列表"，
    /// 因此 `role_ids`/`dept_ids` 为空 Vec 时视为未提供。
    pub async fn create_user(
        &self,
        req: CreateUserRequest,
        creator: Option<String>,
    ) -> AppResult<UserResponse> {
        let email = req.email.filter(|s| !s.is_empty());
        let mobile = req.mobile.filter(|s| !s.is_empty());
        let remark = req.remark.filter(|s| !s.is_empty());
        let sex = req.sex.map(Sex::from).unwrap_or_default();

        // Check email uniqueness
        if let Some(ref e) = email {
            if self.user_service.exists_by_email(e).await? {
                return Err(RepositoryError::DuplicateEmail)?;
            }
        }

        // Check mobile uniqueness
        if let Some(ref m) = mobile {
            if self.user_service.exists_by_mobile(m).await? {
                return Err(RepositoryError::DuplicateMobile)?;
            }
        }

        let mut user = self
            .user_service
            .create_user(req.username, req.password, req.nickname, creator.clone())
            .await?;

        // Set optional fields and persist to repository
        if email.is_some() || mobile.is_some() || req.sex.is_some() || remark.is_some() {
            user.email = email;
            user.mobile = mobile;
            user.sex = sex;
            user.remark = remark;
            user = self
                .user_service
                .update_user(
                    user.id,
                    user.nickname.clone(),
                    user.email.clone(),
                    user.mobile.clone(),
                    user.sex,
                    user.remark.clone(),
                    creator.clone(),
                )
                .await?;
        }

        // Assign roles if provided (empty vec = not provided in proto3)
        if !req.role_ids.is_empty() {
            self.user_service.assign_roles(user.id, req.role_ids.clone()).await?;
            user.role_ids = req.role_ids;
        }

        // Assign departments if provided
        if !req.dept_ids.is_empty() {
            self.user_service.assign_departments(user.id, req.dept_ids.clone()).await?;
            user.dept_ids = req.dept_ids;
        }

        Ok(user_to_response(user))
    }

    /// 更新用户信息
    pub async fn update_user(
        &self,
        req: UpdateUserRequest,
        updater: Option<String>,
    ) -> AppResult<UserResponse> {
        let user = self
            .user_service
            .update_user(
                req.user_id,
                req.nickname.unwrap_or_default(),
                req.email.filter(|s| !s.is_empty()),
                req.mobile.filter(|s| !s.is_empty()),
                req.sex.map(Sex::from).unwrap_or_default(),
                req.remark.filter(|s| !s.is_empty()),
                updater,
            )
            .await?;
        Ok(user_to_response(user))
    }

    /// 删除用户
    pub async fn delete_user(
        &self,
        user_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.user_service.delete_user(user_id, updater).await
    }

    /// 变更用户状态
    pub async fn change_status(
        &self,
        user_id: u64,
        status: UserStatus,
        updater: Option<String>,
    ) -> AppResult<UserResponse> {
        let user = self.user_service.change_status(user_id, status, updater).await?;
        Ok(user_to_response(user))
    }

    /// 修改用户密码
    pub async fn change_password(
        &self,
        req: ChangePasswordRequest,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.user_service
            .change_password(req.user_id, req.new_password, updater)
            .await?;
        Ok(())
    }

    /// 为用户分配角色
    pub async fn assign_roles(&self, req: AssignRolesRequest) -> AppResult<()> {
        self.user_service.assign_roles(req.user_id, req.role_ids).await
    }

    /// 为用户分配部门
    pub async fn assign_departments(&self, req: AssignDeptsRequest) -> AppResult<()> {
        self.user_service.assign_departments(req.user_id, req.dept_ids).await
    }

    /// 根据ID获取用户信息
    pub async fn get_user(&self, user_id: u64) -> AppResult<UserResponse> {
        let user = self.user_service.get_user(user_id).await?;
        Ok(user_to_response(user))
    }

    /// 分页查询用户列表
    pub async fn get_user_page(
        &self,
        req: ListUsersRequest,
    ) -> AppResult<Page<UserResponse>> {
        let query = UserQuery {
            username: req.username,
            nickname: req.nickname,
            mobile: req.mobile,
            status: req.status.map(UserStatus::from),
            dept_id: req.dept_id,
            begin_time: None,
            end_time: None,
        };
        let pi = req.page_info.unwrap_or_default();
        let page = Page::request(pi.page, pi.size);
        let result = self.user_service.get_user_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(user_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }
}
