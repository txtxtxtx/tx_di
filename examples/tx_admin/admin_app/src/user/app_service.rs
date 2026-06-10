use std::sync::Arc;

use crate::user::dto::*;
use admin_domain::user::model::value_object::UserQuery;
use admin_domain::user::service::UserService;
use admin_domain::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

/// User application service - orchestrates domain operations
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
    ) -> Result<UserResponse, RepositoryError> {
        let mut user = self
            .user_service
            .create_user(cmd.username, cmd.password, cmd.nickname, creator)
            .await?;

        // Set optional fields
        if let Some(email) = cmd.email {
            user.email = Some(email);
        }
        if let Some(mobile) = cmd.mobile {
            user.mobile = Some(mobile);
        }
        if let Some(sex) = cmd.sex {
            user.sex = sex;
        }
        if let Some(remark) = cmd.remark {
            user.remark = Some(remark);
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

        Ok(UserResponse::from(user))
    }

    /// Update user
    pub async fn update_user(
        &self,
        cmd: UpdateUserCommand,
        updater: Option<String>,
    ) -> Result<UserResponse, RepositoryError> {
        let user = self
            .user_service
            .update_user(
                cmd.user_id,
                cmd.nickname,
                cmd.email,
                cmd.mobile,
                cmd.sex,
                cmd.remark,
                updater,
            )
            .await?;
        Ok(UserResponse::from(user))
    }

    /// Delete user
    pub async fn delete_user(
        &self,
        user_id: u64,
        updater: Option<String>,
    ) -> Result<(), RepositoryError> {
        self.user_service.delete_user(user_id, updater).await
    }

    /// Change user status
    pub async fn change_status(
        &self,
        user_id: u64,
        status: i32,
        updater: Option<String>,
    ) -> Result<UserResponse, RepositoryError> {
        let user = self.user_service.change_status(user_id, status, updater).await?;
        Ok(UserResponse::from(user))
    }

    /// Change password
    pub async fn change_password(
        &self,
        cmd: ChangePasswordCommand,
        updater: Option<String>,
    ) -> Result<(), RepositoryError> {
        self.user_service
            .change_password(cmd.user_id, cmd.new_password, updater)
            .await?;
        Ok(())
    }

    /// Assign roles to user
    pub async fn assign_roles(&self, cmd: AssignRolesCommand) -> Result<(), RepositoryError> {
        self.user_service.assign_roles(cmd.user_id, cmd.role_ids).await
    }

    /// Assign departments to user
    pub async fn assign_departments(&self, cmd: AssignDeptsCommand) -> Result<(), RepositoryError> {
        self.user_service.assign_departments(cmd.user_id, cmd.dept_ids).await
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: u64) -> Result<UserResponse, RepositoryError> {
        let user = self.user_service.get_user(user_id).await?;
        Ok(UserResponse::from(user))
    }

    /// Get user page
    pub async fn get_user_page(
        &self,
        request: UserQueryRequest,
    ) -> Result<PageResponse<UserResponse>, RepositoryError> {
        let query = UserQuery {
            username: request.username,
            nickname: request.nickname,
            mobile: request.mobile,
            status: request.status,
            dept_id: request.dept_id,
            begin_time: None,
            end_time: None,
        };
        let page = PageRequest::new(request.page, request.page_size);
        let result = self.user_service.get_user_page(&query, &page).await?;

        Ok(PageResponse::new(
            result.list.into_iter().map(UserResponse::from).collect(),
            result.total,
            result.page,
            result.page_size,
        ))
    }
}
