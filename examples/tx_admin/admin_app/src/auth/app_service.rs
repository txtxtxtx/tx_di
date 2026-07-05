use std::sync::Arc;

use admin_proto::{LoginRequest, LoginResponse, LogoutRequest, UserInfoResponse, CreateLoginLogRequest};
use admin_domain::auth::service::AuthService;
use admin_domain::menu::model::value_object::MenuTreeNode;
use admin_domain::menu::model::value_object::MenuQuery;
use admin_domain::shared::model::value_object::SessionEctData;
use admin_domain::role::service::RoleService;
use admin_domain::menu::service::MenuService;
use crate::auth::session_service::AuthSessionService;
use crate::log::app_service::LoginLogAppService;
use crate::user::app_service::UserAppService;
use tx_di_core::{Component, DepsTuple};
use tx_error::AppResult;

/// 认证应用服务
///
/// 编排登录/登出所需的多个领域服务和会话服务。
/// Session 管理由 `AuthSessionService` 封装，不再暴露到 API 层。
#[derive(Component)]
pub struct AuthAppService {
    auth_service: Arc<AuthService>,
    user_app: Arc<UserAppService>,
    role_service: Arc<RoleService>,
    menu_service: Arc<MenuService>,
    login_log_service: Arc<LoginLogAppService>,
    session_service: Arc<AuthSessionService>,
}

impl AuthAppService {
    /// 生产环境构造（含 session 服务）
    pub fn new(
        auth_service: Arc<AuthService>,
        user_app: Arc<UserAppService>,
        role_service: Arc<RoleService>,
        menu_service: Arc<MenuService>,
        login_log_service: Arc<LoginLogAppService>,
        session_service: Arc<AuthSessionService>,
    ) -> Self {
        Self {
            auth_service,
            user_app,
            role_service,
            menu_service,
            login_log_service,
            session_service,
        }
    }

    /// 用户登录
    ///
    /// # 执行逻辑
    /// 1. `AuthService::authenticate()` — 认证用户（存在性/状态/密码）
    /// 2. `UserAppService::build_login_user()` — 构建跨聚合 LoginUser
    /// 3. 记录旁路副作用（登录 IP / 登录日志）
    /// 4. 通过 `AuthSessionService` 创建 session 并获取 token
    ///
    /// # 返回
    /// 成功返回 `LoginResponse`（含 token）
    pub async fn login(&self, req: LoginRequest) -> AppResult<LoginResponse> {
        // ── 1. 认证（领域层封装，返回明确的 AuthError）────────
        let user = self.auth_service.authenticate(&req.username, &req.password).await?;

        // ── 2. 构建跨聚合 LoginUser ──────────────────────────
        let login_user = self.user_app.build_login_user(&user).await?;

        // ── 3. 旁路副作用（发后即忘，不影响主流程）────────────
        let login_ip = req.login_ip.clone();
        let _ = self.user_app.user_service().record_login(user.id, login_ip.clone()).await;
        let log_cmd = CreateLoginLogRequest {
            user_id: user.id,
            user_type: if user.id > 0 { 1 } else { 0 },
            username: user.username.clone(),
            login_ip: login_ip.clone(),
            login_type: "login".to_string(),
            result: 1,
        };
        let _ = self.login_log_service.create_log(log_cmd).await;

        // ── 4. 查询角色编码（构建 session 所需）───────────────
        let roles = self.role_service.get_roles_by_ids(&login_user.role_ids).await?;
        let role_codes: Vec<String> = roles.into_iter().map(|r| r.code).collect();
        let is_admin = RoleService::has_admin_role(&role_codes);

        // ── 5. 创建 session → 获取 token ─────────────────────
        let session_extra = SessionEctData {
            login_ip,
            tenant_id: login_user.tenant_id,
            dept_ids: login_user.dept_ids.clone(),
            role_ids: login_user.role_ids.clone(),
            username: login_user.username.clone(),
        };

        let token = self.session_service
            .login(
                login_user.user_id,
                is_admin,
                session_extra,
                login_user.permissions.into_iter().collect(),
                role_codes.clone(),
            )
            .await?;

        Ok(LoginResponse {
            user_id: login_user.user_id,
            username: login_user.username,
            nickname: login_user.nickname,
            tenant_id: login_user.tenant_id.into(),
            role_ids: login_user.role_ids,
            role_codes,
            permissions: Vec::new(), // 由 session 管理，不再返回明文
            dept_ids: login_user.dept_ids,
            token,
        })
    }

    /// 获取当前已认证用户的详细信息
    pub async fn get_user_info(&self, user_id: u64) -> AppResult<UserInfoResponse> {
        let user = self.user_app.user_service().get_user(user_id).await?;
        let role_ids = user.role_ids.clone();
        let permissions = self.menu_service.get_user_permission_codes(user_id).await?;

        // Get role codes
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
    pub async fn get_user_menus(&self, user_id: u64) -> AppResult<Vec<MenuTreeNode>> {
        let user = self.user_app.user_service().get_user(user_id).await?;
        let role_ids = &user.role_ids;

        // 收集所有角色关联的菜单 ID
        let mut menu_ids = std::collections::HashSet::new();
        for &role_id in role_ids {
            let ids = self.role_service.get_menu_ids(role_id).await?;
            menu_ids.extend(ids);
        }

        // 获取完整菜单树
        let all_tree = self.menu_service.get_menu_tree(&MenuQuery::default()).await?;

        // 过滤
        Ok(Self::filter_tree(&all_tree, &menu_ids))
    }

    /// 递归过滤菜单树
    fn filter_tree(nodes: &[MenuTreeNode], menu_ids: &std::collections::HashSet<u64>) -> Vec<MenuTreeNode> {
        nodes.iter().filter_map(|n| {
            let children = Self::filter_tree(&n.children, menu_ids);
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
    /// # 执行逻辑
    /// 1. 记录登出日志
    /// 2. 销毁 session（由 API 层在调用前完成）
    pub async fn logout(&self, req: LogoutRequest) -> AppResult<()> {
        let user = self.user_app.user_service().get_user(req.user_id).await?;

        let log_cmd = CreateLoginLogRequest {
            user_id: req.user_id,
            user_type: if user.id > 0 { 1 } else { 0 },
            username: user.username,
            login_ip: String::new(),
            login_type: "logout".to_string(),
            result: 1,
        };
        let _ = self.login_log_service.create_log(log_cmd).await;

        Ok(())
    }

    /// 获取 AuthSessionService 引用（供 API 层登出时销毁 session）
    pub fn session_service(&self) -> &Arc<AuthSessionService> {
        &self.session_service
    }

    /// 获取 UserAppService 引用
    pub fn user_app(&self) -> &Arc<UserAppService> {
        &self.user_app
    }
}
