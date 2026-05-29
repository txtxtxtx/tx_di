//! 用户应用服务

use std::sync::Arc;
use tx_di_core::tx_comp;

use crate::domain::data_permission::{DataPermissionContext, DataPermissionService};
use crate::domain::role::Role;
use crate::domain::user::{User, UserId, UserStatus};
use crate::infrastructure::persistence::{InMemoryUserRepository, InMemoryRoleRepository};

/// 创建用户请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub role_ids: Option<Vec<String>>,
}

/// 更新用户请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateUserRequest {
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub status: Option<UserStatus>,
    pub role_ids: Option<Vec<String>>,
}

/// 用户应用服务
#[derive(Debug)]
#[tx_comp]
pub struct UserService {
    pub user_repo: Arc<InMemoryUserRepository>,
    pub role_repo: Arc<InMemoryRoleRepository>,
}

impl UserService {
    /// 查询用户列表（带数据权限过滤）
    pub async fn list_users(
        &self,
        tenant_id: &str,
        keyword: Option<&str>,
        status: Option<UserStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<User>, u64), anyhow::Error> {
        self.user_repo
            .find_page(tenant_id, keyword, status, page, page_size)
            .await
    }

    /// 创建用户
    pub async fn create_user(
        &self,
        tenant_id: &str,
        req: CreateUserRequest,
    ) -> Result<User, anyhow::Error> {
        // 1. 检查用户名是否已存在
        if self
            .user_repo
            .find_by_username(&req.username)
            .await?
            .is_some()
        {
            return Err(anyhow::anyhow!("用户名已存在"));
        }

        // 2. 加密密码
        let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)?;

        // 3. 创建用户
        let user_id = uuid::Uuid::new_v4().to_string();
        let mut user = User::new(
            user_id,
            tenant_id.to_string(),
            req.username,
            password_hash,
            req.nickname,
        );
        user.email = req.email;
        user.mobile = req.mobile;

        if let Some(role_ids) = req.role_ids {
            user.assign_roles(role_ids);
        }

        self.user_repo.save(&user).await?;
        tracing::info!(user_id = %user.id, username = %user.username, "用户已创建");

        Ok(user)
    }

    /// 更新用户
    pub async fn update_user(
        &self,
        user_id: &str,
        req: UpdateUserRequest,
    ) -> Result<User, anyhow::Error> {
        let mut user = self
            .user_repo
            .find_by_id(&user_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("用户不存在"))?;

        if let Some(nickname) = req.nickname {
            user.nickname = nickname;
        }
        if let Some(email) = req.email {
            user.email = Some(email);
        }
        if let Some(mobile) = req.mobile {
            user.mobile = Some(mobile);
        }
        if let Some(status) = req.status {
            user.status = status;
        }
        if let Some(role_ids) = req.role_ids {
            user.assign_roles(role_ids);
        }

        self.user_repo.save(&user).await?;
        Ok(user)
    }

    /// 删除用户
    pub async fn delete_user(&self, user_id: &str) -> Result<(), anyhow::Error> {
        self.user_repo
            .find_by_id(&user_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("用户不存在"))?;

        self.user_repo.delete(&user_id.to_string()).await?;
        tracing::info!(user_id = %user_id, "用户已删除");
        Ok(())
    }

    /// 重置密码
    pub async fn reset_password(
        &self,
        user_id: &str,
        new_password: &str,
    ) -> Result<(), anyhow::Error> {
        let mut user = self
            .user_repo
            .find_by_id(&user_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("用户不存在"))?;

        user.password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)?;
        self.user_repo.save(&user).await?;
        Ok(())
    }
}
