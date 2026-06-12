//! 认证与授权集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第10节）:
//!   10.1 认证功能     ✅ (登录/登出/用户信息)

mod common;

use std::collections::HashSet;
use std::sync::Arc;
use admin_app::auth::dto::*;
use admin_app::mock::{user_repo, permission_repo, role_repo};
use admin_app::auth::app_service::AuthAppService;
use admin_domain::user::model::aggregate::User;
use admin_domain::user::model::value_object::UserStatus;
use admin_domain::user::service::UserService;
use admin_domain::role::service::RoleService;
use admin_domain::permission::service::PermissionService;
use admin_domain::password::hash_password;

/// 创建带哈希密码的用户（测试辅助函数）
fn create_user_with_hashed_password(id: u64, username: &str, password: &str, nickname: &str) -> User {
    let hashed_password = hash_password(password).expect("密码哈希失败");
    User::create(id, username.into(), hashed_password, nickname.into(), None)
}

fn make_auth_app(
    user: User,
    user_roles: Vec<u64>,
    role_perms: Vec<(u64, Vec<&str>)>,
) -> AuthAppService {
    let uid = user.id;
    let user_roles_clone = user_roles.clone();
    let mut perm_repo = permission_repo::MockPermissionRepository::new()
        .with_user_roles(uid, user_roles);
    for (rid, perms) in role_perms {
        perm_repo = perm_repo.with_role_permissions(
            rid,
            HashSet::from_iter(perms.into_iter().map(|s| s.to_string())),
        );
    }
    let user_repo = Arc::new(user_repo::MockUserRepository::new()
        .with_user(user)
        .with_user_roles(uid, user_roles_clone));
    let perm_repo = Arc::new(perm_repo);
    let role_repo = Arc::new(role_repo::MockRoleRepository::new());
    let user_svc = Arc::new(UserService::new(user_repo, perm_repo.clone()));
    let role_svc = Arc::new(RoleService::new(role_repo));
    let perm_svc = Arc::new(PermissionService::new(perm_repo));
    AuthAppService::new(user_svc, role_svc, perm_svc)
}

// ── 登录 ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn login_success() {
    let user = create_user_with_hashed_password(1, "admin", "password123", "管理员");
    let app = make_auth_app(user, vec![1], vec![(1, vec!["*"])]);

    let resp = app.login(LoginCommand {
        username: "admin".into(), password: "password123".into(), login_ip: "127.0.0.1".into(),
    }).await.unwrap();

    assert_eq!(resp.user_id, 1);
    assert_eq!(resp.username, "admin");
    assert_eq!(resp.nickname, "管理员");
    assert!(!resp.permissions.is_empty());
    assert!(!resp.role_ids.is_empty());
}

#[tokio::test]
async fn login_wrong_password() {
    let user = create_user_with_hashed_password(1, "admin", "password123", "管理员");
    let app = make_auth_app(user, vec![], vec![]);

    let r = app.login(LoginCommand {
        username: "admin".into(), password: "wrong_pwd".into(), login_ip: "127.0.0.1".into(),
    }).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn login_nonexistent_user() {
    let user = create_user_with_hashed_password(1, "admin", "pwd", "管理员");
    let app = make_auth_app(user, vec![], vec![]);

    let r = app.login(LoginCommand {
        username: "ghost".into(), password: "pwd".into(), login_ip: "127.0.0.1".into(),
    }).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn login_disabled_user() {
    let mut user = create_user_with_hashed_password(1, "admin", "password123", "管理员");
    user.change_status(UserStatus::Disabled, None);
    let app = make_auth_app(user, vec![], vec![]);

    let r = app.login(LoginCommand {
        username: "admin".into(), password: "password123".into(), login_ip: "127.0.0.1".into(),
    }).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn login_locked_user() {
    let mut user = create_user_with_hashed_password(1, "admin", "password123", "管理员");
    user.change_status(UserStatus::Locked, None);
    let app = make_auth_app(user, vec![], vec![]);

    let r = app.login(LoginCommand {
        username: "admin".into(), password: "password123".into(), login_ip: "127.0.0.1".into(),
    }).await;
    assert!(r.is_err());
}

// ── 用户信息 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_user_info() {
    let user = User::create(1, "admin".into(), "pwd".into(), "管理员".into(), None);
    let app = make_auth_app(user, vec![1], vec![(1, vec!["system:user:list", "system:role:list"])]);

    let info = app.get_user_info(1).await.unwrap();
    assert_eq!(info.user_id, 1);
    assert_eq!(info.username, "admin");
    assert_eq!(info.nickname, "管理员");
    assert!(!info.permissions.is_empty());
}

#[tokio::test]
async fn get_user_info_not_found() {
    let user = User::create(1, "admin".into(), "pwd".into(), "管理员".into(), None);
    let app = make_auth_app(user, vec![], vec![]);

    let r = app.get_user_info(999).await;
    assert!(r.is_err());
}

// ── 登出 ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn logout_success() {
    let user = User::create(1, "admin".into(), "pwd".into(), "管理员".into(), None);
    let app = make_auth_app(user, vec![], vec![]);

    app.logout(LogoutCommand { user_id: 1 }).await.unwrap();
}
