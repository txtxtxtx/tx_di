//! 全流程集成测试 — 验证跨模块协作
//!
//! 覆盖场景:
//!   - 部门 → 角色 → 用户 → 菜单 → 角色菜单 → 配置 → 字典 → 文件 → 日志 的完整链路

mod common;
use admin_app::auth::dto::*;
use admin_app::config::dto::*;
use admin_app::department::dto::*;
use admin_app::dictionary::dto::*;
use admin_app::file::dto::*;
use admin_app::log::dto::*;
use admin_app::menu::dto::*;
use admin_app::role::dto::*;
use admin_app::user::dto::*;
use admin_domain::user::model::value_object::Sex;

#[tokio::test]
async fn full_crud_workflow() {
    // 1. 创建部门层级
    let (dept_app, _, _) = common::create_dept_app().await;
    let root_dept = dept_app.create_dept(CreateDeptCommand {
        name: "总公司".into(), parent_id: 0, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();
    let tech_dept = dept_app.create_dept(CreateDeptCommand {
        name: "技术部".into(), parent_id: root_dept.id, sort: 1,
        leader_user_id: None, phone: None, email: None,
    }, Some("admin".into())).await.unwrap();

    // 2. 创建角色
    let (role_app, _, _) = common::create_role_app().await;
    let admin_role = role_app.create_role(CreateRoleCommand {
        name: "管理员".into(), code: "admin".into(), sort: 1,
        remark: None, menu_ids: None,
    }, Some("admin".into())).await.unwrap();
    let user_role = role_app.create_role(CreateRoleCommand {
        name: "普通用户".into(), code: "user".into(), sort: 2,
        remark: None, menu_ids: None,
    }, Some("admin".into())).await.unwrap();

    // 3. 创建用户（含角色和部门）
    let (user_app, _, _) = common::create_user_app().await;
    let user = user_app.create_user(CreateUserCommand {
        username: "zhangsan".into(), password: "pwd123".into(), nickname: "张三".into(),
        email: Some("zs@example.com".into()), mobile: Some("13800000001".into()),
        sex: Some(Sex::Male), remark: None,
        role_ids: Some(vec![admin_role.id, user_role.id]),
        dept_ids: Some(vec![tech_dept.id]),
    }, Some("admin".into())).await.unwrap();
    assert_eq!(user.username, "zhangsan");
    assert!(user.role_ids.contains(&admin_role.id));
    assert!(user.dept_ids.contains(&tech_dept.id));

    // 4. 创建菜单层级并分配给角色
    let (menu_app, _, _) = common::create_menu_app().await;
    let sys_menu = menu_app.create_menu(CreateMenuCommand {
        name: "系统管理".into(), permission: "system".into(), types: 0, sort: 1,
        parent_id: 0, path: Some("/system".into()), icon: Some("setting".into()),
        component: None, component_name: Some("System".into()),
    }, Some("admin".into())).await.unwrap();
    let user_menu = menu_app.create_menu(CreateMenuCommand {
        name: "用户管理".into(), permission: "system:user:list".into(), types: 1, sort: 1,
        parent_id: sys_menu.id, path: Some("/system/user".into()), icon: Some("user".into()),
        component: Some("system/user/index".into()), component_name: Some("UserMgmt".into()),
    }, Some("admin".into())).await.unwrap();
    role_app.assign_menus(AssignMenusCommand {
        role_id: admin_role.id, menu_ids: vec![sys_menu.id, user_menu.id],
    }).await.unwrap();

    // 5. 创建系统配置
    let (config_app, _, _) = common::create_config_app().await;
    config_app.create_config(CreateConfigCommand {
        category: "system".into(), config_type: 0, name: "系统名称".into(),
        config_key: "sys.name".into(), value: "Admin System".into(), remark: None,
    }, Some("admin".into())).await.unwrap();
    let cfg = config_app.get_by_key("sys.name").await.unwrap();
    assert_eq!(cfg.value, "Admin System");

    // 6. 创建字典
    let (dict_type_app, _, _) = common::create_dict_type_app().await;
    dict_type_app.create_dict_type(CreateDictTypeCommand {
        name: "用户性别".into(), dict_type: "sys_user_sex".into(), remark: None,
    }, Some("admin".into())).await.unwrap();
    let (dict_data_app, _, _) = common::create_dict_data_app().await;
    for (i, label) in ["男", "女"].iter().enumerate() {
        dict_data_app.create_dict_data(CreateDictDataCommand {
            sort: i as i32, label: label.to_string(), value: i.to_string(),
            dict_type: "sys_user_sex".into(), color_type: None, css_class: None, remark: None,
        }, Some("admin".into())).await.unwrap();
    }
    let items = dict_data_app.get_by_dict_type("sys_user_sex").await.unwrap();
    assert_eq!(items.len(), 2);

    // 7. 文件上传
    let (file_app, _, _) = common::create_file_app().await;
    let uploaded = file_app.upload_file(UploadFileCommand {
        name: "avatar.png".into(), path: "/uploads/avatar.png".into(),
        url: "https://cdn.example.com/uploads/avatar.png".into(),
        file_type: Some("image/png".into()), size: 2048, config_id: None,
    }, Some("admin".into())).await.unwrap();
    assert_eq!(uploaded.name, "avatar.png");

    // 8. 日志记录
    let (log_app, _, _) = common::create_operate_log_app().await;
    let log = log_app.create_log(CreateOperateLogCommand {
        trace_id: "workflow-trace".into(), user_id: user.id, user_type: 1,
        log_type: "操作日志".into(), sub_type: "用户管理".into(),
        biz_id: user.id, action: "创建用户".into(), success: 1,
        extra: format!(r#"{{"username":"{}"}}"#, user.username),
    }).await.unwrap();
    assert_eq!(log.user_id, user.id);

    // 9. 验证配置管理分页
    let cfg_page = config_app.get_config_page(ConfigQueryRequest {
        category: None, config_key: None, name: None, config_type: None, page: 1, size: 10,
    }).await.unwrap();
    assert!(cfg_page.total >= 1);

    println!("Full CRUD workflow test completed!");
}
