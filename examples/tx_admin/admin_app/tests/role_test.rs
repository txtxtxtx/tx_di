//! 角色管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第2节）:
//!   2.1 角色CRUD     ✅
//!   2.2 角色状态     ❌ (领域模型支持，无独立 API)
//!   2.3 权限分配     ❌ (未实现)

mod common;
use admin_app::role::dto::*;

// ── 2.1 角色 CRUD ──────────────────────────────────────────────────────────

#[tokio::test]
async fn create_role_success() {
    let (app, _, _) = common::create_role_app();
    let role = app.create_role(CreateRoleCommand {
        name: "管理员".into(), code: "admin".into(), sort: 1,
        remark: Some("系统管理员".into()), menu_ids: None,
    }, Some("admin".into())).await.unwrap();
    assert_eq!(role.name, "管理员");
    assert_eq!(role.code, "admin");
    assert_eq!(role.sort, 1);
}

#[tokio::test]
async fn create_role_with_menus() {
    let (app, _, _) = common::create_role_app();
    let role = app.create_role(CreateRoleCommand {
        name: "运营".into(), code: "operator".into(), sort: 2,
        remark: None, menu_ids: Some(vec![1, 2, 3]),
    }, Some("admin".into())).await.unwrap();
    assert_eq!(role.code, "operator");
    assert!(!role.menu_ids.is_empty());
}

#[tokio::test]
async fn create_duplicate_code_should_fail() {
    let (app, _, _) = common::create_role_app();
    let cmd = |code: &str, name: &str| CreateRoleCommand {
        name: name.into(), code: code.into(), sort: 1, remark: None, menu_ids: None,
    };
    app.create_role(cmd("admin", "管理员"), Some("admin".into())).await.unwrap();
    assert!(app.create_role(cmd("admin", "超级管理员"), Some("admin".into())).await.is_err());
}

#[tokio::test]
async fn update_role_success() {
    let (app, _, _) = common::create_role_app();
    let role = app.create_role(CreateRoleCommand {
        name: "编辑角色".into(), code: "editor".into(), sort: 1, remark: None, menu_ids: None,
    }, Some("admin".into())).await.unwrap();

    let updated = app.update_role(UpdateRoleCommand {
        role_id: role.id,
        name: "高级编辑".into(),
        code: "senior_editor".into(),
        sort: 10,
        data_scope: 2,
        remark: Some("已升级".into()),
    }, Some("admin".into())).await.unwrap();

    assert_eq!(updated.name, "高级编辑");
    assert_eq!(updated.code, "senior_editor");
    assert_eq!(updated.sort, 10);
    assert_eq!(updated.data_scope, 2);
}

#[tokio::test]
async fn delete_role_success() {
    let (app, _, _) = common::create_role_app();
    let role = app.create_role(CreateRoleCommand {
        name: "待删除".into(), code: "todel".into(), sort: 99, remark: None, menu_ids: None,
    }, Some("admin".into())).await.unwrap();
    app.delete_role(role.id, Some("admin".into())).await.unwrap();
    assert!(app.get_role(role.id).await.is_err());
}

#[tokio::test]
async fn get_role_detail() {
    let (app, _, _) = common::create_role_app();
    let role = app.create_role(CreateRoleCommand {
        name: "详情角色".into(), code: "detail_role".into(), sort: 1,
        remark: Some("备注信息".into()), menu_ids: None,
    }, Some("admin".into())).await.unwrap();

    let found = app.get_role(role.id).await.unwrap();
    assert_eq!(found.name, "详情角色");
    assert_eq!(found.code, "detail_role");
}

#[tokio::test]
async fn paginate_roles() {
    let (app, _, _) = common::create_role_app();
    for i in 0..5 {
        app.create_role(CreateRoleCommand {
            name: format!("角色{}", i), code: format!("r{}", i), sort: i,
            remark: None, menu_ids: None,
        }, Some("admin".into())).await.unwrap();
    }
    let page = app.get_role_page(RoleQueryRequest {
        name: None, code: None, status: None, page: 1, page_size: 2,
    }).await.unwrap();
    assert_eq!(page.list.len(), 2);
    assert_eq!(page.total, 5);
}

#[tokio::test]
async fn query_role_by_name_fuzzy() {
    let (app, _, _) = common::create_role_app();
    app.create_role(CreateRoleCommand {
        name: "系统管理员".into(), code: "sys_admin".into(), sort: 1, remark: None, menu_ids: None,
    }, Some("admin".into())).await.unwrap();
    app.create_role(CreateRoleCommand {
        name: "普通用户".into(), code: "user".into(), sort: 2, remark: None, menu_ids: None,
    }, Some("admin".into())).await.unwrap();

    let page = app.get_role_page(RoleQueryRequest {
        name: Some("管理".into()), code: None, status: None, page: 1, page_size: 10,
    }).await.unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].code, "sys_admin");
}

// ── 菜单分配 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn assign_menus_to_role() {
    let (app, _, _) = common::create_role_app();
    let role = app.create_role(CreateRoleCommand {
        name: "菜单角色".into(), code: "menu_role".into(), sort: 1, remark: None, menu_ids: None,
    }, Some("admin".into())).await.unwrap();

    let result = app.assign_menus(AssignMenusCommand { role_id: role.id, menu_ids: vec![1, 2, 3, 4, 5] }).await.unwrap();
    assert_eq!(result.menu_ids, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn assign_menus_empty_should_clear() {
    let (app, _, _) = common::create_role_app();
    let role = app.create_role(CreateRoleCommand {
        name: "清空菜单".into(), code: "clear_menu".into(), sort: 1,
        remark: None, menu_ids: Some(vec![1, 2]),
    }, Some("admin".into())).await.unwrap();

    let result = app.assign_menus(AssignMenusCommand { role_id: role.id, menu_ids: vec![] }).await.unwrap();
    assert!(result.menu_ids.is_empty());
}
