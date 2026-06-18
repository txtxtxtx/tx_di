//! 菜单管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第4节）:
//!   4.1 菜单CRUD     ✅
//!   4.2 菜单类型     ✅ (目录/菜单/按钮)
//!   4.3 菜单属性     ✅ (路径/组件/图标/排序)

mod common;
use admin_proto::{CreateMenuRequest, UpdateMenuRequest, ListMenusRequest};

// ── 4.1 菜单 CRUD ──────────────────────────────────────────────────────────

#[tokio::test]
async fn create_menu_directory() {
    let (app, _, _) = common::create_menu_app().await;
    let menu = app.create_menu(CreateMenuRequest {
        name: "系统管理".into(), permission: "system".into(), types: 0, sort: 1,
        parent_id: 0, path: Some("/system".into()), icon: Some("setting".into()),
        component: None, component_name: Some("System".into()),
    }, Some("admin".into())).await.unwrap();
    assert_eq!(menu.name, "系统管理");
    assert_eq!(menu.types, 0);
    assert_eq!(menu.parent_id, 0);
}

#[tokio::test]
async fn create_menu_page() {
    let (app, _, _) = common::create_menu_app().await;
    let menu = app.create_menu(CreateMenuRequest {
        name: "用户管理".into(), permission: "system:user:list".into(), types: 1, sort: 1,
        parent_id: 1, path: Some("/system/user".into()), icon: Some("user".into()),
        component: Some("system/user/index".into()), component_name: Some("UserManagement".into()),
    }, Some("admin".into())).await.unwrap();
    assert_eq!(menu.types, 1);
    assert_eq!(menu.component, Some("system/user/index".into()));
}

#[tokio::test]
async fn create_menu_button() {
    let (app, _, _) = common::create_menu_app().await;
    let menu = app.create_menu(CreateMenuRequest {
        name: "新增用户".into(), permission: "system:user:create".into(), types: 2, sort: 1,
        parent_id: 2, path: None, icon: None, component: None, component_name: Some("UserCreate".into()),
    }, Some("admin".into())).await.unwrap();
    assert_eq!(menu.types, 2);
    assert_eq!(menu.permission, "system:user:create");
}

#[tokio::test]
async fn create_menu_hierarchy() {
    let (app, _, _) = common::create_menu_app().await;
    let parent = app.create_menu(CreateMenuRequest {
        name: "系统".into(), permission: "sys".into(), types: 0, sort: 1,
        parent_id: 0, path: Some("/sys".into()), icon: Some("gear".into()),
        component: None, component_name: Some("Sys".into()),
    }, Some("admin".into())).await.unwrap();
    let child = app.create_menu(CreateMenuRequest {
        name: "用户".into(), permission: "sys:user".into(), types: 1, sort: 1,
        parent_id: parent.id, path: Some("/sys/user".into()), icon: Some("user".into()),
        component: Some("sys/user/index".into()), component_name: Some("SysUser".into()),
    }, Some("admin".into())).await.unwrap();
    assert_eq!(child.parent_id, parent.id);
}

#[tokio::test]
async fn update_menu() {
    let (app, _, _) = common::create_menu_app().await;
    let menu = app.create_menu(CreateMenuRequest {
        name: "旧名称".into(), permission: "old".into(), types: 1, sort: 1,
        parent_id: 0, path: None, icon: None, component: None, component_name: None,
    }, Some("admin".into())).await.unwrap();

    let updated = app.update_menu(UpdateMenuRequest {
        menu_id: menu.id,
        name: "新名称".into(),
        permission: "new:perm".into(),
        types: 0,
        sort: 99,
        parent_id: 10,
        path: Some("/new".into()),
        icon: Some("star".into()),
        component: Some("new/index".into()),
        component_name: Some("NewComp".into()),
        visible: 1,
        keep_alive: 0,
    }, Some("admin".into())).await.unwrap();

    assert_eq!(updated.name, "新名称");
    assert_eq!(updated.permission, "new:perm");
    assert_eq!(updated.sort, 99);
}

#[tokio::test]
async fn delete_menu() {
    let (app, _, _) = common::create_menu_app().await;
    let menu = app.create_menu(CreateMenuRequest {
        name: "待删除".into(), permission: "del".into(), types: 1, sort: 99,
        parent_id: 0, path: None, icon: None, component: None, component_name: None,
    }, Some("admin".into())).await.unwrap();
    app.delete_menu(menu.id, Some("admin".into())).await.unwrap();
}

// ── 菜单树 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_menu_tree() {
    let (app, _, _) = common::create_menu_app().await;
    let root = app.create_menu(CreateMenuRequest {
        name: "系统管理".into(), permission: "system".into(), types: 0, sort: 1,
        parent_id: 0, path: Some("/system".into()), icon: Some("setting".into()),
        component: None, component_name: Some("System".into()),
    }, Some("admin".into())).await.unwrap();
    app.create_menu(CreateMenuRequest {
        name: "用户管理".into(), permission: "system:user:list".into(), types: 1, sort: 1,
        parent_id: root.id, path: Some("/system/user".into()), icon: Some("user".into()),
        component: Some("system/user/index".into()), component_name: Some("User".into()),
    }, Some("admin".into())).await.unwrap();
    app.create_menu(CreateMenuRequest {
        name: "角色管理".into(), permission: "system:role:list".into(), types: 1, sort: 2,
        parent_id: root.id, path: Some("/system/role".into()), icon: Some("peoples".into()),
        component: Some("system/role/index".into()), component_name: Some("Role".into()),
    }, Some("admin".into())).await.unwrap();

    let tree = app.get_menu_tree(ListMenusRequest { name: None, status: None, types: None }).await.unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].children.len(), 2);
}

#[tokio::test]
async fn get_menu_list_flat() {
    let (app, _, _) = common::create_menu_app().await;
    for i in 0..3 {
        app.create_menu(CreateMenuRequest {
            name: format!("菜单{}", i), permission: format!("m{}", i), types: 1, sort: i,
            parent_id: 0, path: None, icon: None, component: None, component_name: None,
        }, Some("admin".into())).await.unwrap();
    }
    let list = app.get_menu_list(ListMenusRequest { name: None, status: None, types: None }).await.unwrap();
    assert_eq!(list.len(), 3);
}
