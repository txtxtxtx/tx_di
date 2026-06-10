use std::sync::Arc;

use crate::auth::dto::*;
use admin_domain::user::service::UserService;
use admin_domain::role::service::RoleService;
use admin_domain::permission::service::PermissionService;
use admin_domain::shared::repository::RepositoryError;
use tx_error::AppResult;

/// Authentication application service
pub struct AuthAppService {
    user_service: Arc<UserService>,
    role_service: Arc<RoleService>,
    permission_service: Arc<PermissionService>,
}

impl AuthAppService {
    pub fn new(
        user_service: Arc<UserService>,
        role_service: Arc<RoleService>,
        permission_service: Arc<PermissionService>,
    ) -> Self {
        Self {
            user_service,
            role_service,
            permission_service,
        }
    }

    /// User login
    pub async fn login(&self, cmd: LoginCommand) -> AppResult<LoginResponse> {
        // Find user by username
        let user = self
            .user_service
            .get_by_username(&cmd.username)
            .await?
            .ok_or_else(|| RepositoryError::NotFound)?;

        // Check if user is active
        if !user.is_active() {
            return Err(RepositoryError::Validation)?;
        }

        if user.is_locked() {
            return Err(RepositoryError::Validation)?;
        }

        // Verify password (in real app, compare hashed passwords)
        if user.password != cmd.password {
            return Err(RepositoryError::Validation)?;
        }

        // Build login user info
        let login_user = self.user_service.build_login_user(&user).await?;

        // Record login
        self.user_service.record_login(user.id, cmd.login_ip).await?;

        Ok(LoginResponse {
            user_id: login_user.user_id,
            username: login_user.username,
            nickname: login_user.nickname,
            tenant_id: login_user.tenant_id,
            role_ids: login_user.role_ids,
            permissions: login_user.permissions,
            dept_ids: login_user.dept_ids,
        })
    }

    /// Get user info (for authenticated user)
    pub async fn get_user_info(&self, user_id: u64) -> AppResult<UserInfoResponse> {
        let user = self.user_service.get_user(user_id).await?;
        let role_ids = self.user_service.get_user(user_id).await?.role_ids;
        let permissions = self.permission_service.get_user_permissions(user_id).await?;

        // Get role names
        let roles = self.role_service.get_roles_by_ids(&role_ids).await?;
        let role_names: Vec<String> = roles.into_iter().map(|r| r.code).collect();

        Ok(UserInfoResponse {
            user_id: user.id,
            username: user.username,
            nickname: user.nickname,
            email: user.email,
            mobile: user.mobile,
            avatar: user.avatar,
            roles: role_names,
            permissions,
        })
    }

    /// User logout
    pub async fn logout(&self, _cmd: LogoutCommand) -> AppResult<()> {
        // In a real app, invalidate token/session
        Ok(())
    }
}
