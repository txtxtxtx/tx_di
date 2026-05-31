//! 用户领域服务
//!
//! 处理跨实体的业务逻辑：认证、注册、权限检查等。
//! 不直接依赖 HTTP 层，纯领域逻辑。

use std::sync::Arc;

use crate::domain::error::AdminError;
use crate::domain::user::{User, UserRepository, UserStatus};
use crate::domain::role::{Role, RoleRepository};
use crate::domain::tenant::{Tenant, TenantRepository};

/// 认证结果
pub struct AuthResult {
    pub user: User,
    pub roles: Vec<Role>,
    pub tenant: Tenant,
}

/// 用户领域服务
pub struct UserService {
    pub user_repo: Arc<dyn UserRepository>,
    pub role_repo: Arc<dyn RoleRepository>,
    pub tenant_repo: Arc<dyn TenantRepository>,
}

impl UserService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        role_repo: Arc<dyn RoleRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
    ) -> Self {
        Self { user_repo, role_repo, tenant_repo }
    }

    /// 认证：验证凭证 + 检查状态 + 加载角色和租户
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<AuthResult, AdminError> {
        // 1. 查找用户
        let user = self.user_repo
            .find_by_username(username).await?
            .ok_or(AdminError::BadCredentials)?;

        // 2. 验证密码
        if !bcrypt::verify(password, &user.password_hash).unwrap_or(false) {
            return Err(AdminError::BadCredentials);
        }

        // 3. 检查用户状态
        if user.status != UserStatus::Active {
            return Err(AdminError::UserDisabled);
        }

        // 4. 检查租户状态
        let tenant = self.tenant_repo
            .find_by_id(user.tenant_id).await?
            .ok_or(AdminError::TenantDisabled)?;
        if !tenant.is_active() {
            return Err(AdminError::TenantDisabled);
        }

        // 5. 加载角色
        let roles = self.role_repo
            .find_by_tenant(user.tenant_id).await
            .unwrap_or_default();

        Ok(AuthResult { user, roles, tenant })
    }

    /// 获取用户信息（含角色）
    pub async fn get_user_with_roles(&self, user_id: u64) -> Result<(User, Vec<Role>), AdminError> {
        let user = self.user_repo
            .find_by_id(user_id).await?
            .ok_or(AdminError::UserNotFound(user_id.to_string()))?;
        let roles = self.role_repo
            .find_by_tenant(user.tenant_id).await
            .unwrap_or_default();
        Ok((user, roles))
    }

    /// 注册用户
    pub async fn register(
        &self,
        tenant_id: u64,
        username: &str,
        password: &str,
        nickname: &str,
    ) -> Result<User, AdminError> {
        // 检查用户名唯一性
        if self.user_repo.find_by_username(username).await?.is_some() {
            return Err(AdminError::UsernameDuplicate(username.to_string()));
        }

        // 哈希密码
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| AdminError::Database(e.to_string()))?;

        // 创建用户
        let user = User::new(tenant_id, username.to_string(), password_hash, nickname.to_string());
        self.user_repo.save(&user).await?;

        Ok(user)
    }

    /// 重置密码
    pub async fn reset_password(&self, user_id: u64, new_password: &str) -> Result<(), AdminError> {
        let mut user = self.user_repo
            .find_by_id(user_id).await?
            .ok_or(AdminError::UserNotFound(user_id.to_string()))?;

        let hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| AdminError::Database(e.to_string()))?;

        user.change_password(hash);
        self.user_repo.save(&user).await?;
        Ok(())
    }
}
