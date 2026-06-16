use std::sync::Arc;

use crate::user::dto::*;
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
    pub fn new(user_service: Arc<UserService>) -> Self {
        Self { user_service }
    }

    /// Create a new user
    pub async fn create_user(
        &self,
        cmd: CreateUserCommand,
        creator: Option<String>,
    ) -> AppResult<UserResponse> {
        // Check email uniqueness
        if let Some(ref email) = cmd.email {
            if self.user_service.exists_by_email(email).await? {
                return Err(RepositoryError::DuplicateUsername)?;
            }
        }

        // Check mobile uniqueness
        if let Some(ref mobile) = cmd.mobile {
            if self.user_service.exists_by_mobile(mobile).await? {
                return Err(RepositoryError::DuplicateUsername)?;
            }
        }

        let mut user = self
            .user_service
            .create_user(cmd.username, cmd.password, cmd.nickname, creator.clone())
            .await?;

        // Set optional fields and persist to repository
        if cmd.email.is_some() || cmd.mobile.is_some() || cmd.sex.is_some() || cmd.remark.is_some() {
            user.email = cmd.email;
            user.mobile = cmd.mobile;
            user.sex = cmd.sex.unwrap_or(Sex::Unknown);
            user.remark = cmd.remark;
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

        // Assign roles if provided
        if let Some(role_ids) = cmd.role_ids {
            self.user_service.assign_roles(user.id, role_ids.clone()).await?;
            user.role_ids = role_ids;
        }

        // Assign departments if provided
        if let Some(dept_ids) = cmd.dept_ids {
            self.user_service.assign_departments(user.id, dept_ids.clone()).await?;
            user.dept_ids = dept_ids;
        }

        Ok(user_to_response(user))
    }

    /// Update user
    pub async fn update_user(
        &self,
        cmd: UpdateUserCommand,
        updater: Option<String>,
    ) -> AppResult<UserResponse> {
        let user = self
            .user_service
            .update_user(
                cmd.user_id,
                cmd.nickname.unwrap_or_default(),
                cmd.email,
                cmd.mobile,
                cmd.sex.unwrap_or(Sex::Unknown),
                cmd.remark,
                updater,
            )
            .await?;
        Ok(user_to_response(user))
    }

    /// Delete user
    pub async fn delete_user(
        &self,
        user_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.user_service.delete_user(user_id, updater).await
    }

    /// Change user status
    pub async fn change_status(
        &self,
        user_id: u64,
        status: UserStatus,
        updater: Option<String>,
    ) -> AppResult<UserResponse> {
        let user = self.user_service.change_status(user_id, status, updater).await?;
        Ok(user_to_response(user))
    }

    /// Change password
    pub async fn change_password(
        &self,
        cmd: ChangePasswordCommand,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.user_service
            .change_password(cmd.user_id, cmd.new_password, updater)
            .await?;
        Ok(())
    }

    /// Assign roles to user
    pub async fn assign_roles(&self, cmd: AssignRolesCommand) -> AppResult<()> {
        self.user_service.assign_roles(cmd.user_id, cmd.role_ids).await
    }

    /// Assign departments to user
    pub async fn assign_departments(&self, cmd: AssignDeptsCommand) -> AppResult<()> {
        self.user_service.assign_departments(cmd.user_id, cmd.dept_ids).await
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: u64) -> AppResult<UserResponse> {
        let user = self.user_service.get_user(user_id).await?;
        Ok(user_to_response(user))
    }

    /// Get user page
    pub async fn get_user_page(
        &self,
        request: UserQueryRequest,
    ) -> AppResult<Page<UserResponse>> {
        let query = UserQuery {
            username: request.username,
            nickname: request.nickname,
            mobile: request.mobile,
            status: request.status,
            dept_id: request.dept_id,
            begin_time: None,
            end_time: None,
        };
        let page = Page::request(request.page, request.size);
        let result = self.user_service.get_user_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(user_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }
}
