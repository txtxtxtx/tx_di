//! 权限管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第3节）:
//!   3.1 权限查询       ✅ (获取所有权限/权限检查/用户权限)

mod common;

use std::collections::HashSet;
use std::sync::Arc;
use admin_app::permission::dto::*;
use admin_app::permission::app_service::PermissionAppService;
use admin_app::mock::permission_repo::MockPermissionRepository;
use admin_domain::permission::service::PermissionService;

// ── 权限列表 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_all_permissions_should_return_list() {
    let (svc, _) = common::create_permission_service();
    let app = PermissionAppService::new(svc);
    let perms = app.get_all_permissions().await.unwrap();
    assert!(!perms.is_empty());
    assert!(perms.iter().any(|p| p.code == "system:user:list"));
    assert!(perms.iter().any(|p| p.code == "system:role:create"));
}

// ── 权限检查 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn check_permission_has_permission() {
    let repo = Arc::new(
        MockPermissionRepository::new()
            .with_user_roles(1, vec![1])
            .with_role_permissions(1, HashSet::from([
                "system:user:list".into(),
                "system:user:create".into(),
            ]))
    );
    let svc = Arc::new(PermissionService::new(repo));
    let app = PermissionAppService::new(svc);

    let r = app.check_permission(PermissionCheckRequest {
        user_id: 1, permission: "system:user:list".into(),
    }).await.unwrap();
    assert!(r.has_permission);
}

#[tokio::test]
async fn check_permission_no_permission() {
    let repo = Arc::new(
        MockPermissionRepository::new()
            .with_user_roles(1, vec![1])
            .with_role_permissions(1, HashSet::from(["system:user:list".into()]))
    );
    let svc = Arc::new(PermissionService::new(repo));
    let app = PermissionAppService::new(svc);

    let r = app.check_permission(PermissionCheckRequest {
        user_id: 1, permission: "system:user:delete".into(),
    }).await.unwrap();
    assert!(!r.has_permission);
}

#[tokio::test]
async fn check_permission_user_without_roles() {
    let (app, _, _) = common::create_permission_app();
    let r = app.check_permission(PermissionCheckRequest {
        user_id: 999, permission: "system:user:list".into(),
    }).await.unwrap();
    assert!(!r.has_permission);
}

// ── 用户权限列表 ───────────────────────────────────────────────────────────

#[tokio::test]
async fn get_user_permissions_aggregate_from_roles() {
    let repo = Arc::new(
        MockPermissionRepository::new()
            .with_user_roles(1, vec![1, 2])
            .with_role_permissions(1, HashSet::from(["system:user:list".into()]))
            .with_role_permissions(2, HashSet::from(["system:role:list".into()]))
    );
    let svc = Arc::new(PermissionService::new(repo));
    let app = PermissionAppService::new(svc);

    let r = app.get_user_permissions(1).await.unwrap();
    assert_eq!(r.permissions.len(), 2);
    assert!(r.permissions.contains(&"system:user:list".to_string()));
    assert!(r.permissions.contains(&"system:role:list".to_string()));
}

#[tokio::test]
async fn get_user_permissions_deduplicate_same_permission() {
    let repo = Arc::new(
        MockPermissionRepository::new()
            .with_user_roles(1, vec![1, 2])
            .with_role_permissions(1, HashSet::from(["system:user:list".into()]))
            .with_role_permissions(2, HashSet::from(["system:user:list".into()]))
    );
    let svc = Arc::new(PermissionService::new(repo));
    let app = PermissionAppService::new(svc);

    let r = app.get_user_permissions(1).await.unwrap();
    assert_eq!(r.permissions.len(), 1);
}

#[tokio::test]
async fn get_user_permissions_no_roles() {
    let (app, _, _) = common::create_permission_app();
    let r = app.get_user_permissions(999).await.unwrap();
    assert!(r.permissions.is_empty());
}
