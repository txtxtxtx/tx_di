//! 部门管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第5节）:
//!   5.1 部门CRUD     ✅
//!   5.3 层级管理     ✅ (树/父子关系)

mod common;
use admin_app::department::dto::*;

// ── 5.1 部门 CRUD ──────────────────────────────────────────────────────────

#[tokio::test]
async fn create_department_root() {
    let (app, _, _) = common::create_dept_app().await;
    let dept = app.create_dept(CreateDeptCommand {
        name: "总公司".into(), parent_id: 0, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    assert_eq!(dept.name, "总公司");
    assert_eq!(dept.parent_id, 0);
}

#[tokio::test]
async fn create_department_with_leader() {
    let (app, _, _) = common::create_dept_app().await;
    let dept = app.create_dept(CreateDeptCommand {
        name: "技术部".into(), parent_id: 0, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    // leader_user_id 在 update 时设置
    let updated = app.update_dept(UpdateDeptCommand {
        dept_id: dept.id, name: "技术部".into(), parent_id: 0, sort: 1,
        leader_user_id: Some(1), phone: Some("010-12345678".into()), email: Some("tech@example.com".into()),
    }, Some("admin".into())).await.unwrap();
    assert_eq!(updated.name, "技术部");
    assert_eq!(updated.leader_user_id, Some(1));
}

#[tokio::test]
async fn create_dept_hierarchy() {
    let (app, _, _) = common::create_dept_app().await;
    let parent = app.create_dept(CreateDeptCommand {
        name: "总公司".into(), parent_id: 0, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    let child = app.create_dept(CreateDeptCommand {
        name: "技术部".into(), parent_id: parent.id, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    assert_eq!(child.parent_id, parent.id);
}

#[tokio::test]
async fn update_department() {
    let (app, _, _) = common::create_dept_app().await;
    let dept = app.create_dept(CreateDeptCommand {
        name: "旧部门".into(), parent_id: 0, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();

    let updated = app.update_dept(UpdateDeptCommand {
        dept_id: dept.id, name: "新部门".into(), parent_id: 10, sort: 20,
        leader_user_id: Some(5), phone: Some("010-999".into()), email: Some("new@example.com".into()),
    }, Some("admin".into())).await.unwrap();

    assert_eq!(updated.name, "新部门");
    assert_eq!(updated.sort, 20);
    assert_eq!(updated.leader_user_id, Some(5));
}

#[tokio::test]
async fn delete_department() {
    let (app, _, _) = common::create_dept_app().await;
    let dept = app.create_dept(CreateDeptCommand {
        name: "待删除部门".into(), parent_id: 0, sort: 99,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    app.delete_dept(dept.id, Some("admin".into())).await.unwrap();
    assert!(app.get_dept(dept.id).await.is_err());
}

#[tokio::test]
async fn get_department_detail() {
    let (app, _, _) = common::create_dept_app().await;
    let dept = app.create_dept(CreateDeptCommand {
        name: "详情部门".into(), parent_id: 0, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    // 通过 update 设置 leader_user_id
    app.update_dept(UpdateDeptCommand {
        dept_id: dept.id, name: "详情部门".into(), parent_id: 0, sort: 1,
        leader_user_id: Some(10), phone: None, email: None,
    }, Some("admin".into())).await.unwrap();

    let found = app.get_dept(dept.id).await.unwrap();
    assert_eq!(found.name, "详情部门");
    assert_eq!(found.leader_user_id, Some(10));
}

// ── 部门树 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_dept_tree() {
    let (app, _, _) = common::create_dept_app().await;
    let root = app.create_dept(CreateDeptCommand {
        name: "总公司".into(), parent_id: 0, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    app.create_dept(CreateDeptCommand {
        name: "技术部".into(), parent_id: root.id, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    app.create_dept(CreateDeptCommand {
        name: "产品部".into(), parent_id: root.id, sort: 2,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();

    let tree = app.get_dept_tree(DeptQueryRequest { name: None, status: None }).await.unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].children.len(), 2);
}

#[tokio::test]
async fn get_dept_list_flat() {
    let (app, _, _) = common::create_dept_app().await;
    for i in 0..3 {
        app.create_dept(CreateDeptCommand {
            name: format!("部门{}", i), parent_id: 0, sort: i,
            leader_user_id: None, phone: None, email: None,
        }, Some("admin".into())).await.unwrap();
    }
    let list = app.get_dept_list(DeptQueryRequest { name: None, status: None }).await.unwrap();
    assert_eq!(list.len(), 3);
}

#[tokio::test]
async fn get_child_dept_ids_via_tree() {
    let (app, _, _) = common::create_dept_app().await;
    let parent = app.create_dept(CreateDeptCommand {
        name: "总公司".into(), parent_id: 0, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    app.create_dept(CreateDeptCommand {
        name: "技术部".into(), parent_id: parent.id, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    app.create_dept(CreateDeptCommand {
        name: "产品部".into(), parent_id: parent.id, sort: 2,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();

    // 通过树形查询验证子部门
    let tree = app.get_dept_tree(DeptQueryRequest { name: None, status: None }).await.unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].children.len(), 2);
}
