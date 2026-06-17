use std::sync::Arc;

use crate::auth::dto::*;
use crate::log::app_service::LoginLogAppService;
use crate::log::dto::CreateLoginLogCommand;
use admin_domain::user::service::UserService;
use admin_domain::role::service::RoleService;
use admin_domain::permission::service::PermissionService;
use admin_domain::shared::repository::RepositoryError;
use admin_domain::password;
use tx_di_core::tx_comp;
use tx_error::AppResult;

/// Authentication application service
#[tx_comp]
pub struct AuthAppService {
    user_service: Arc<UserService>,
    role_service: Arc<RoleService>,
    permission_service: Arc<PermissionService>,
    login_log_service: Arc<LoginLogAppService>,
}

impl AuthAppService {
    /// 创建认证应用服务实例
    ///
    /// # 参数
    /// * `user_service` - 用户领域服务，用于查询和管理用户
    /// * `role_service` - 角色领域服务，用于查询角色信息
    /// * `permission_service` - 权限领域服务，用于查询用户权限
    pub fn new(
        user_service: Arc<UserService>,
        role_service: Arc<RoleService>,
        permission_service: Arc<PermissionService>,
        login_log_service: Arc<LoginLogAppService>,
    ) -> Self {
        Self {
            user_service,
            role_service,
            permission_service,
            login_log_service,
        }
    }

    /// 用户登录
    ///
    /// # 参数
    /// * `cmd` - 登录命令，包含用户名、密码和登录IP
    ///
    /// # 执行逻辑
    /// 1. 根据用户名查找用户，若不存在则返回 `NotFoundUser` 错误
    /// 2. 校验用户是否处于激活状态，未激活则返回 `ValidationLogin` 错误
    /// 4. 使用 Argon2id 算法验证密码，密码错误则返回 `ValidationPassword` 错误
    /// 5. 调用用户服务构建登录用户信息（含角色、权限等）
    /// 6. 记录本次登录信息（用户ID、登录IP）
    /// 7. 组装并返回登录响应
    ///
    /// # 返回
    /// 成功返回 `LoginResponse`，包含用户ID、用户名、昵称、租户ID、角色ID列表、权限集合和部门ID列表
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户名不存在
    /// - `ValidationLogin` - 用户未激活或已被锁定
    /// - `ValidationPassword` - 密码验证失败或哈希计算出错
    pub async fn login(&self, cmd: LoginCommand) -> AppResult<LoginResponse> {
        // Find user by username
        let user = self
            .user_service
            .get_by_username(&cmd.username)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;

        // Check if user is active
        if !user.is_active() {
            return Err(RepositoryError::ValidationLogin)?;
        }

        // Verify password using Argon2id hash verification
        let is_valid = password::verify_password(&cmd.password, &user.password)
            .map_err(|_| RepositoryError::ValidationPassword)?;

        if !is_valid {
            return Err(RepositoryError::ValidationPassword)?;
        }

        // Build login user info
        let login_user = self.user_service.build_login_user(&user).await?;

        // Record login
        self.user_service.record_login(user.id, cmd.login_ip).await?;

        // 查询角色编码列表
        let roles = self.role_service.get_roles_by_ids(&login_user.role_ids).await?;
        let role_codes: Vec<String> = roles.into_iter().map(|r| r.code).collect();

        Ok(LoginResponse {
            user_id: login_user.user_id,
            username: login_user.username,
            nickname: login_user.nickname,
            tenant_id: login_user.tenant_id,
            role_ids: login_user.role_ids,
            role_codes,
            permissions: login_user.permissions.into_iter().collect(),
            dept_ids: login_user.dept_ids,
        })
    }

    /// 获取当前已认证用户的详细信息
    ///
    /// # 参数
    /// * `user_id` - 当前登录用户的ID
    ///
    /// # 执行逻辑
    /// 1. 根据用户ID查询用户实体
    /// 2. 通过权限服务获取该用户拥有的所有权限编码
    /// 3. 根据用户关联的角色ID列表批量查询角色信息，提取角色编码
    /// 4. 组装用户信息响应
    ///
    /// # 返回
    /// 成功返回 `UserInfoResponse`，包含用户基本信息、角色列表和权限集合
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 数据库查询异常
    pub async fn get_user_info(&self, user_id: u64) -> AppResult<UserInfoResponse> {
        let user = self.user_service.get_user(user_id).await?;
        let role_ids = user.role_ids.clone();
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
            permissions: permissions.into_iter().collect(),
        })
    }

    /// 用户登出
    ///
    /// # 参数
    /// * `cmd` - 登出命令，包含用户 ID
    ///
    /// # 执行逻辑
    /// 1. 根据用户 ID 查询用户信息（用于记录日志的用户名）
    /// 2. 记录登出类型的登录日志（login_type = "logout"，result = 0 表示成功）
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户 ID 对应的用户不存在
    /// - 日志写入异常
    pub async fn logout(&self, cmd: LogoutCommand) -> AppResult<()> {
        // 查询用户信息用于日志记录
        let user = self.user_service.get_user(cmd.user_id).await?;

        // 记录登出日志
        let log_cmd = CreateLoginLogCommand {
            user_id: cmd.user_id,
            user_type: 0,
            username: user.username,
            login_ip: String::new(),
            login_type: "logout".to_string(),
            result: 0, // 成功
        };
        let _ = self.login_log_service.create_log(log_cmd).await;

        Ok(())
    }
}
