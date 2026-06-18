//! 认证与授权集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第10节）:
//!   10.1 认证功能     ✅ (登录/登出/用户信息)

mod common;

use std::sync::Arc;
use admin_proto::{LoginRequest, LogoutRequest, CreateRoleRequest, CreateMenuRequest, CreateUserRequest};
use admin_app::auth::app_service::AuthAppService;
use admin_domain::user::service::UserService;
use admin_domain::role::service::RoleService;
use admin_domain::menu::service::MenuService;
use admin_domain::user::repository::UserRepository;
use admin_domain::role::repository::RoleRepository;
use admin_domain::user::model::value_object::UserStatus;
use admin_infra::user::repository::ToastyUserRepository;
use admin_infra::role::repository::ToastyRoleRepository;
use admin_infra::menu::repository::ToastyMenuRepository;
use admin_infra::department::repository::ToastyDepartmentRepository;
use admin_infra::log::repository::ToastyLoginLogRepository;
use admin_domain::log::service::LoginLogService;
use admin_app::log::app_service::LoginLogAppService;

/// 创建认证测试环境
///
/// 返回 (auth_app, user_app, role_app, menu_app, user_repo, role_repo)
async fn create_auth_test_env() -> (
    AuthAppService,
    admin_app::user::app_service::UserAppService,
    admin_app::role::app_service::RoleAppService,
    admin_app::menu::app_service::MenuAppService,
    Arc<ToastyUserRepository>,
    Arc<ToastyRoleRepository>,
) {
    let plugin = common::create_db_plugin().await;

    let user_repo = Arc::new(ToastyUserRepository::new(plugin.clone()));
    let role_repo = Arc::new(ToastyRoleRepository::new(plugin.clone()));
    let menu_repo = Arc::new(ToastyMenuRepository::new(plugin.clone()));
    let dept_repo = Arc::new(ToastyDepartmentRepository::new(plugin.clone()));
    let login_log_repo = Arc::new(ToastyLoginLogRepository::new(plugin));

    let user_svc = Arc::new(UserService::new(user_repo.clone(), role_repo.clone(), dept_repo, menu_repo.clone()));
    let role_svc = Arc::new(RoleService::new(role_repo.clone(), user_repo.clone()));
    let menu_svc = Arc::new(MenuService::new(menu_repo));
    let login_log_svc = Arc::new(LoginLogService::new(login_log_repo));
    let login_log_app = Arc::new(LoginLogAppService::new(login_log_svc));

    let user_app = admin_app::user::app_service::UserAppService::new(user_svc.clone());
    let role_app = admin_app::role::app_service::RoleAppService::new(role_svc.clone());
    let menu_app = admin_app::menu::app_service::MenuAppService::new(menu_svc);
    let auth_app = AuthAppService::new(user_svc, role_svc, menu_svc, login_log_app);

    (auth_app, user_app, role_app, menu_app, user_repo, role_repo)
}

// ── 登录 ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn login_success() {
    let (auth_app, user_app, role_app, menu_app, user_repo, role_repo) =
        create_auth_test_env().await;

    // 创建用户（密码由 UserAppService 自动哈希）
    let user = user_app.create_user(CreateUserRequest {
        username: "admin".into(),
        password: "password123".into(),
        nickname: "管理员".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: vec![], dept_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    // 创建角色
    let role = role_app.create_role(CreateRoleRequest {
        name: "管理员".into(), code: "admin".into(), sort: 1,
        remark: None, menu_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    // 创建通配权限菜单
    let menu = menu_app.create_menu(CreateMenuRequest {
        name: "全部权限".into(), permission: "*".into(),
        types: 2, sort: 1, parent_id: 0,
        path: None, icon: None, component: None, component_name: None,
    }, Some("admin".into())).await.unwrap();

    // 绑定用户到角色，绑定菜单到角色
    user_repo.bind_roles(user.id, &[role.id]).await.unwrap();
    role_repo.bind_menus(role.id, &[menu.id]).await.unwrap();

    let resp = auth_app.login(LoginRequest {
        username: "admin".into(),
        password: "password123".into(),
        login_ip: "127.0.0.1".into(),
    }).await.unwrap();

    assert_eq!(resp.user_id, user.id);
    assert_eq!(resp.username, "admin");
    assert_eq!(resp.nickname, "管理员");
    assert!(!resp.permissions.is_empty());
    assert!(!resp.role_ids.is_empty());
}

#[tokio::test]
async fn login_wrong_password() {
    let (auth_app, user_app, _, _, _, _) = create_auth_test_env().await;

    user_app.create_user(CreateUserRequest {
        username: "admin".into(),
        password: "password123".into(),
        nickname: "管理员".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: vec![], dept_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    let r = auth_app.login(LoginRequest {
        username: "admin".into(),
        password: "wrong_pwd".into(),
        login_ip: "127.0.0.1".into(),
    }).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn login_nonexistent_user() {
    let (auth_app, _, _, _, _, _) = create_auth_test_env().await;

    let r = auth_app.login(LoginRequest {
        username: "ghost".into(),
        password: "pwd".into(),
        login_ip: "127.0.0.1".into(),
    }).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn login_disabled_user() {
    let (auth_app, user_app, _, _, _, _) = create_auth_test_env().await;

    let user = user_app.create_user(CreateUserRequest {
        username: "admin".into(),
        password: "password123".into(),
        nickname: "管理员".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: vec![], dept_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    // 禁用用户
    user_app.change_status(user.id, UserStatus::Disabled, Some("admin".into()))
        .await.unwrap();

    let r = auth_app.login(LoginRequest {
        username: "admin".into(),
        password: "password123".into(),
        login_ip: "127.0.0.1".into(),
    }).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn login_locked_user() {
    let (auth_app, user_app, _, _, _, _) = create_auth_test_env().await;

    let user = user_app.create_user(CreateUserRequest {
        username: "admin".into(),
        password: "password123".into(),
        nickname: "管理员".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: vec![], dept_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    // 锁定用户
    user_app.change_status(user.id, UserStatus::Locked, Some("admin".into()))
        .await.unwrap();

    let r = auth_app.login(LoginRequest {
        username: "admin".into(),
        password: "password123".into(),
        login_ip: "127.0.0.1".into(),
    }).await;
    assert!(r.is_err());
}

// ── 用户信息 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_user_info() {
    let (auth_app, user_app, role_app, menu_app, user_repo, role_repo) =
        create_auth_test_env().await;

    let user = user_app.create_user(CreateUserRequest {
        username: "admin".into(),
        password: "pwd".into(),
        nickname: "管理员".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: vec![], dept_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    let role = role_app.create_role(CreateRoleRequest {
        name: "管理员".into(), code: "admin".into(), sort: 1,
        remark: None, menu_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    // 创建权限菜单
    let menu1 = menu_app.create_menu(CreateMenuRequest {
        name: "用户列表".into(), permission: "system:user:list".into(),
        types: 2, sort: 1, parent_id: 0,
        path: None, icon: None, component: None, component_name: None,
    }, Some("admin".into())).await.unwrap();
    let menu2 = menu_app.create_menu(CreateMenuRequest {
        name: "角色列表".into(), permission: "system:role:list".into(),
        types: 2, sort: 2, parent_id: 0,
        path: None, icon: None, component: None, component_name: None,
    }, Some("admin".into())).await.unwrap();

    user_repo.bind_roles(user.id, &[role.id]).await.unwrap();
    role_repo.bind_menus(role.id, &[menu1.id, menu2.id]).await.unwrap();

    let info = auth_app.get_user_info(user.id).await.unwrap();
    assert_eq!(info.user_id, user.id);
    assert_eq!(info.username, "admin");
    assert_eq!(info.nickname, "管理员");
    assert!(!info.permissions.is_empty());
}

#[tokio::test]
async fn get_user_info_not_found() {
    let (auth_app, _, _, _, _, _) = create_auth_test_env().await;
    let r = auth_app.get_user_info(999).await;
    assert!(r.is_err());
}

// ── 登出 ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn logout_success() {
    let (auth_app, user_app, _, _, _, _) = create_auth_test_env().await;

    let user = user_app.create_user(CreateUserRequest {
        username: "admin".into(),
        password: "pwd".into(),
        nickname: "管理员".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: vec![], dept_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    auth_app.logout(LogoutRequest { user_id: user.id }).await.unwrap();
}
