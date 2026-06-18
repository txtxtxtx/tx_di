//! 权限管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第3节）:
//!   3.1 权限查询       ✅ (获取所有权限/权限检查/用户权限)

mod common;

use std::sync::Arc;
use admin_proto::{PermissionCheckRequest, CreatePermissionRequest};
use admin_app::permission::app_service::PermissionAppService;
use admin_app::user::dto::CreateUserCommand;
use admin_proto::CreateRoleRequest;
use admin_proto::CreateMenuRequest;
use admin_domain::permission::service::PermissionService;
use admin_domain::user::service::UserService;
use admin_domain::role::service::RoleService;
use admin_domain::menu::service::MenuService;
use admin_domain::user::repository::UserRepository;
use admin_domain::role::repository::RoleRepository;
use admin_infra::user::repository::ToastyUserRepository;
use admin_infra::role::repository::ToastyRoleRepository;
use admin_infra::menu::repository::ToastyMenuRepository;
use admin_infra::permission::repository::ToastyPermissionRepository;

/// 创建共享数据库的权限测试环境
///
/// 返回 (permission_app, user_app, role_app, menu_app, user_repo, role_repo)
async fn create_permission_test_env() -> (
    PermissionAppService,
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
    let permission_repo = Arc::new(ToastyPermissionRepository::new(plugin));

    let user_svc = Arc::new(UserService::new(user_repo.clone(), permission_repo.clone()));
    let role_svc = Arc::new(RoleService::new(role_repo.clone()));
    let menu_svc = Arc::new(MenuService::new(menu_repo.clone()));
    let perm_svc = Arc::new(PermissionService::new(permission_repo));

    let user_app = admin_app::user::app_service::UserAppService::new(user_svc);
    let role_app = admin_app::role::app_service::RoleAppService::new(role_svc);
    let menu_app = admin_app::menu::app_service::MenuAppService::new(menu_svc);
    let perm_app = PermissionAppService::new(perm_svc);

    (perm_app, user_app, role_app, menu_app, user_repo, role_repo)
}

// ── 权限列表 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_all_permissions_should_return_list() {
    let (perm_app, _, _, _, _, _) = create_permission_test_env().await;

    // 在 SysPermission 表中插入权限数据
    perm_app.create_permission(CreatePermissionRequest {
        name: "用户列表".into(),
        permission_code: "system:user:list".into(),
        r#type: 1,
        parent_id: 0,
        sort: 1,
        description: "".into(),
    }, Some("admin".into())).await.unwrap();

    perm_app.create_permission(CreatePermissionRequest {
        name: "角色创建".into(),
        permission_code: "system:role:create".into(),
        r#type: 1,
        parent_id: 0,
        sort: 2,
        description: "".into(),
    }, Some("admin".into())).await.unwrap();

    let perms = perm_app.get_all_permissions().await.unwrap();
    assert!(!perms.is_empty());
    assert!(perms.iter().any(|p| p.code == "system:user:list"));
    assert!(perms.iter().any(|p| p.code == "system:role:create"));
}

// ── 权限检查 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn check_permission_has_permission() {
    let (perm_app, user_app, role_app, menu_app, user_repo, role_repo) =
        create_permission_test_env().await;

    // 创建用户
    let user = user_app.create_user(CreateUserCommand {
        username: "testuser".into(),
        password: "pwd".into(),
        nickname: "测试用户".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: None, dept_ids: None,
    }, Some("admin".into())).await.unwrap();

    // 创建角色
    let role = role_app.create_role(CreateRoleRequest {
        name: "测试角色".into(), code: "test_role".into(), sort: 1,
        remark: None, menu_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    // 创建权限菜单 (types=2 = 按钮/权限)
    let menu = menu_app.create_menu(CreateMenuRequest {
        name: "用户列表".into(), permission: "system:user:list".into(),
        types: 2, sort: 1, parent_id: 0,
        path: None, icon: None, component: None, component_name: None,
    }, Some("admin".into())).await.unwrap();

    // 绑定用户到角色
    user_repo.bind_roles(user.id, &[role.id]).await.unwrap();
    // 绑定菜单到角色
    role_repo.bind_menus(role.id, &[menu.id]).await.unwrap();

    let r = perm_app.check_permission(PermissionCheckRequest {
        user_id: user.id, permission: "system:user:list".into(),
    }).await.unwrap();
    assert!(r.has_permission);
}

#[tokio::test]
async fn check_permission_no_permission() {
    let (perm_app, user_app, role_app, menu_app, user_repo, role_repo) =
        create_permission_test_env().await;

    let user = user_app.create_user(CreateUserCommand {
        username: "testuser".into(),
        password: "pwd".into(),
        nickname: "测试用户".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: None, dept_ids: None,
    }, Some("admin".into())).await.unwrap();

    let role = role_app.create_role(CreateRoleRequest {
        name: "测试角色".into(), code: "test_role".into(), sort: 1,
        remark: None, menu_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    // 创建一个权限菜单（只有 system:user:list）
    let menu = menu_app.create_menu(CreateMenuRequest {
        name: "用户列表".into(), permission: "system:user:list".into(),
        types: 2, sort: 1, parent_id: 0,
        path: None, icon: None, component: None, component_name: None,
    }, Some("admin".into())).await.unwrap();

    user_repo.bind_roles(user.id, &[role.id]).await.unwrap();
    role_repo.bind_menus(role.id, &[menu.id]).await.unwrap();

    // 检查用户没有的权限
    let r = perm_app.check_permission(PermissionCheckRequest {
        user_id: user.id, permission: "system:user:delete".into(),
    }).await.unwrap();
    assert!(!r.has_permission);
}

#[tokio::test]
async fn check_permission_user_without_roles() {
    let (perm_app, _, _, _, _, _) = create_permission_test_env().await;
    let r = perm_app.check_permission(PermissionCheckRequest {
        user_id: 999, permission: "system:user:list".into(),
    }).await.unwrap();
    assert!(!r.has_permission);
}

// ── 用户权限列表 ───────────────────────────────────────────────────────────

#[tokio::test]
async fn get_user_permissions_aggregate_from_roles() {
    let (perm_app, user_app, role_app, menu_app, user_repo, role_repo) =
        create_permission_test_env().await;

    let user = user_app.create_user(CreateUserCommand {
        username: "testuser".into(),
        password: "pwd".into(),
        nickname: "测试用户".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: None, dept_ids: None,
    }, Some("admin".into())).await.unwrap();

    // 创建两个角色
    let role1 = role_app.create_role(CreateRoleRequest {
        name: "角色1".into(), code: "role1".into(), sort: 1,
        remark: None, menu_ids: vec![],
    }, Some("admin".into())).await.unwrap();
    let role2 = role_app.create_role(CreateRoleRequest {
        name: "角色2".into(), code: "role2".into(), sort: 2,
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

    // 绑定用户到两个角色
    user_repo.bind_roles(user.id, &[role1.id, role2.id]).await.unwrap();
    // 绑定菜单到角色
    role_repo.bind_menus(role1.id, &[menu1.id]).await.unwrap();
    role_repo.bind_menus(role2.id, &[menu2.id]).await.unwrap();

    let r = perm_app.get_user_permissions(user.id).await.unwrap();
    assert_eq!(r.permissions.len(), 2);
    assert!(r.permissions.contains(&"system:user:list".to_string()));
    assert!(r.permissions.contains(&"system:role:list".to_string()));
}

#[tokio::test]
async fn get_user_permissions_deduplicate_same_permission() {
    let (perm_app, user_app, role_app, menu_app, user_repo, role_repo) =
        create_permission_test_env().await;

    let user = user_app.create_user(CreateUserCommand {
        username: "testuser".into(),
        password: "pwd".into(),
        nickname: "测试用户".into(),
        email: None, mobile: None, sex: None, remark: None,
        role_ids: None, dept_ids: None,
    }, Some("admin".into())).await.unwrap();

    let role1 = role_app.create_role(CreateRoleRequest {
        name: "角色1".into(), code: "role1".into(), sort: 1,
        remark: None, menu_ids: vec![],
    }, Some("admin".into())).await.unwrap();
    let role2 = role_app.create_role(CreateRoleRequest {
        name: "角色2".into(), code: "role2".into(), sort: 2,
        remark: None, menu_ids: vec![],
    }, Some("admin".into())).await.unwrap();

    // 同一个权限菜单绑定到两个角色
    let menu = menu_app.create_menu(CreateMenuRequest {
        name: "用户列表".into(), permission: "system:user:list".into(),
        types: 2, sort: 1, parent_id: 0,
        path: None, icon: None, component: None, component_name: None,
    }, Some("admin".into())).await.unwrap();

    user_repo.bind_roles(user.id, &[role1.id, role2.id]).await.unwrap();
    role_repo.bind_menus(role1.id, &[menu.id]).await.unwrap();
    role_repo.bind_menus(role2.id, &[menu.id]).await.unwrap();

    let r = perm_app.get_user_permissions(user.id).await.unwrap();
    assert_eq!(r.permissions.len(), 1);
}

#[tokio::test]
async fn get_user_permissions_no_roles() {
    let (perm_app, _, _, _, _, _) = create_permission_test_env().await;
    let r = perm_app.get_user_permissions(999).await.unwrap();
    assert!(r.permissions.is_empty());
}
