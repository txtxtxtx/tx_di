use std::sync::Arc;

use admin_proto::{LoginRequest, LoginResponse, LogoutRequest, UserInfoResponse, CreateLoginLogRequest, MenuTreeNode};
use crate::log::app_service::LoginLogAppService;
use admin_domain::user::service::UserService;
use admin_domain::role::service::RoleService;
use admin_domain::permission::service::PermissionService;
use admin_domain::menu::service::MenuService;
use admin_domain::menu::model::value_object::MenuQuery;
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
    menu_service: Arc<MenuService>,
    login_log_service: Arc<LoginLogAppService>,
}

impl AuthAppService {
    /// 创建认证应用服务实例
    ///
    /// # 参数
    /// * `user_service` - 用户领域服务，用于查询和管理用户
    /// * `role_service` - 角色领域服务，用于查询角色信息
    /// * `permission_service` - 权限领域服务，用于查询用户权限
    /// * `menu_service` - 菜单领域服务，用于查询菜单树
    pub fn new(
        user_service: Arc<UserService>,
        role_service: Arc<RoleService>,
        permission_service: Arc<PermissionService>,
        menu_service: Arc<MenuService>,
        login_log_service: Arc<LoginLogAppService>,
    ) -> Self {
        Self {
            user_service,
            role_service,
            permission_service,
            menu_service,
            login_log_service,
        }
    }

    /// 用户登录
    ///
    /// # 参数
    /// * `req` - 登录请求，包含用户名、密码和登录IP
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
    pub async fn login(&self, req: LoginRequest) -> AppResult<LoginResponse> {
        // Find user by username
        let user = self
            .user_service
            .get_by_username(&req.username)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;

        // Check if user is active
        if !user.is_active() {
            return Err(RepositoryError::ValidationLogin)?;
        }

        // Verify password using Argon2id hash verification
        let is_valid = password::verify_password(&req.password, &user.password)
            .map_err(|_| RepositoryError::ValidationPassword)?;

        if !is_valid {
            return Err(RepositoryError::ValidationPassword)?;
        }

        // Build login user info
        let login_user = self.user_service.build_login_user(&user).await?;

        // Record login
        self.user_service.record_login(user.id, req.login_ip).await?;

        // 查询角色编码列表
        let roles = self.role_service.get_roles_by_ids(&login_user.role_ids).await?;
        let role_codes: Vec<String> = roles.into_iter().map(|r| r.code).collect();

        Ok(LoginResponse {
            user_id: login_user.user_id,
            username: login_user.username,
            nickname: login_user.nickname,
            tenant_id: login_user.tenant_id.into(),
            role_ids: login_user.role_ids,
            role_codes,
            permissions: login_user.permissions.into_iter().collect(),
            dept_ids: login_user.dept_ids,
            token: String::new(), // API handler fills token
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
            tenant_id: user.tenant_id.into(),
        })
    }

    /// 获取当前用户的菜单树
    ///
    /// # 参数
    /// * `user_id` - 当前登录用户的 ID
    ///
    /// # 执行逻辑
    /// 1. 获取用户的角色 ID 列表
    /// 2. 遍历角色，收集所有关联的菜单 ID（去重）
    /// 3. 获取完整菜单树，过滤出用户拥有的菜单（保留树结构）
    ///
    /// # 返回
    /// 成功返回过滤后的菜单树 `Vec<MenuTreeNode>`
    pub async fn get_user_menus(&self, user_id: u64) -> AppResult<Vec<MenuTreeNode>> {
        let user = self.user_service.get_user(user_id).await?;
        let role_ids = &user.role_ids;

        // 收集所有角色关联的菜单 ID
        let mut menu_ids = std::collections::HashSet::new();
        for &role_id in role_ids {
            let ids = self.role_service.get_menu_ids(role_id).await?;
            menu_ids.extend(ids);
        }

        // 获取完整菜单树
        let all_tree = self.menu_service.get_menu_tree(&MenuQuery::default()).await?;

        // 过滤：只保留用户拥有的菜单，递归保留父节点
        Ok(Self::filter_tree(&all_tree, &menu_ids))
    }

    /// 递归过滤菜单树，只保留 menu_ids 中的节点及其祖先
    fn filter_tree(nodes: &[MenuTreeNode], menu_ids: &std::collections::HashSet<u64>) -> Vec<MenuTreeNode> {
        nodes.iter().filter_map(|n| {
            let children = Self::filter_tree(&n.children, menu_ids);
            // 保留条件：自身在 menu_ids 中，或有保留下来的子节点
            if menu_ids.contains(&n.id) || !children.is_empty() {
                Some(MenuTreeNode {
                    id: n.id,
                    name: n.name.clone(),
                    permission: n.permission.clone(),
                    types: n.types,
                    sort: n.sort,
                    parent_id: n.parent_id,
                    path: n.path.clone(),
                    icon: n.icon.clone(),
                    component: n.component.clone(),
                    component_name: n.component_name.clone(),
                    status: n.status,
                    visible: n.visible,
                    keep_alive: n.keep_alive,
                    children,
                })
            } else {
                None
            }
        }).collect()
    }

    /// 用户登出
    ///
    /// # 参数
    /// * `req` - 登出请求，包含用户 ID
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
    pub async fn logout(&self, req: LogoutRequest) -> AppResult<()> {
        // 查询用户信息用于日志记录
        let user = self.user_service.get_user(req.user_id).await?;

        // 记录登出日志
        let log_cmd = CreateLoginLogRequest {
            user_id: req.user_id,
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
