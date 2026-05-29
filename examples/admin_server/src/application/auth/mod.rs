//! 认证应用服务
//!
//! 处理登录、登出、Token 管理等认证相关用例。

use std::sync::Arc;
use tx_di_core::tx_comp;

use crate::domain::role::Role;
use crate::domain::tenant::Tenant;
use crate::domain::user::{User, UserStatus};
use crate::domain::UserRepository;
use crate::infrastructure::persistence::{
    InMemoryUserRepository, InMemoryRoleRepository, InMemoryTenantRepository,
};

/// 登录请求
#[derive(Debug, Clone)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub tenant_id: Option<String>,
}

/// 登录响应
#[derive(Debug, Clone)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/// 用户信息（不含敏感字段）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub tenant_id: String,
}

/// 认证应用服务
///
/// DI 组件：通过 `#[tx_comp]` 自动注入仓储依赖。
#[derive(Debug)]
#[tx_comp]
pub struct AuthService {
    pub user_repo: Arc<InMemoryUserRepository>,
    pub role_repo: Arc<InMemoryRoleRepository>,
    pub tenant_repo: Arc<InMemoryTenantRepository>,
}

impl AuthService {
    /// 用户登录
    pub async fn login(&self, req: LoginRequest) -> Result<LoginResponse, anyhow::Error> {
        // 1. 查找用户
        let user = self
            .user_repo
            .find_by_username(&req.username)
            .await?
            .ok_or_else(|| anyhow::anyhow!("用户名或密码错误"))?;

        // 2. 验证密码
        let valid = bcrypt::verify(&req.password, &user.password_hash)
            .unwrap_or(false);
        if !valid {
            return Err(anyhow::anyhow!("用户名或密码错误"));
        }

        // 3. 检查用户状态
        if !user.is_active() {
            return Err(anyhow::anyhow!("用户已被禁用"));
        }

        // 4. 检查租户状态
        let tenant = self
            .tenant_repo
            .find_by_id(&user.tenant_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("租户不存在"))?;
        if !tenant.is_active() {
            return Err(anyhow::anyhow!("租户已被禁用或已过期"));
        }

        // 5. 获取角色和权限
        let roles = self.role_repo.find_by_ids(&user.role_ids).await?;
        let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();

        // 6. 获取所有权限编码
        let perm_ids: Vec<String> = roles
            .iter()
            .flat_map(|r| r.permission_ids.clone())
            .collect();
        // 简单实现：直接用角色编码作为权限（生产环境应查询权限表）
        let permissions: Vec<String> = role_codes
            .iter()
            .flat_map(|r| {
                if r == "admin" {
                    vec!["*:*:*".to_string()]
                } else {
                    vec![
                        format!("{}:read", r),
                        format!("{}:write", r),
                    ]
                }
            })
            .collect();

        // 7. 生成 Token（使用用户名+时间戳的简单方案，可对接 sa-token）
        let token = format!(
            "{}.{}.{}",
            user.id,
            user.username,
            chrono::Utc::now().timestamp()
        );

        let user_info = UserInfo {
            id: user.id.clone(),
            username: user.username.clone(),
            nickname: user.nickname.clone(),
            avatar: user.avatar.clone(),
            roles: role_codes,
            permissions,
            tenant_id: user.tenant_id.clone(),
        };

        Ok(LoginResponse {
            token,
            user: user_info,
        })
    }

    /// 获取当前用户信息
    pub async fn get_user_info(
        &self,
        user_id: &str,
    ) -> Result<UserInfo, anyhow::Error> {
        let user = self
            .user_repo
            .find_by_id(&user_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("用户不存在"))?;

        let roles = self.role_repo.find_by_ids(&user.role_ids).await?;
        let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();

        let permissions: Vec<String> = role_codes
            .iter()
            .flat_map(|r| {
                if r == "admin" {
                    vec!["*:*:*".to_string()]
                } else {
                    vec![format!("{}:read", r)]
                }
            })
            .collect();

        Ok(UserInfo {
            id: user.id.clone(),
            username: user.username.clone(),
            nickname: user.nickname.clone(),
            avatar: user.avatar.clone(),
            roles: role_codes,
            permissions,
            tenant_id: user.tenant_id.clone(),
        })
    }
}
