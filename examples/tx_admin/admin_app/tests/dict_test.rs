//! 字典管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第8节）:
//!   8.1 字典类型 CRUD  ✅
//!   8.2 字典项 CRUD    ✅
//!   8.4 字典操作       ✅ (按编码查询)

mod common;
use admin_proto::{
    CreateDictTypeRequest, UpdateDictTypeRequest, ListDictTypesRequest,
    CreateDictDataRequest, UpdateDictDataRequest, ListDictDataRequest,
};

// ── 8.1 字典类型 CRUD ─────────────────────────────────────────────────────

#[tokio::test]
async fn create_dict_type_success() {
    let (app, _, _) = common::create_dict_type_app().await;
    let req = CreateDictTypeRequest {
        name: "用户性别".into(),
        dict_type: "sys_user_sex".into(),
        remark: None,
    };
    let dt = app.create_dict_type(req, Some("admin".into())).await.unwrap();
    assert_eq!(dt.name, "用户性别");
    assert_eq!(dt.dict_type, "sys_user_sex");
    // remark 在 create 时为 None，通过 update 设置
}

#[tokio::test]
async fn create_duplicate_dict_type_should_fail() {
    let (app, _, _) = common::create_dict_type_app().await;
    let req = |t: &str| CreateDictTypeRequest {
        name: "名称".into(), dict_type: t.into(), remark: None,
    };
    app.create_dict_type(req("dup_type"), Some("admin".into())).await.unwrap();
    assert!(app.create_dict_type(req("dup_type"), Some("admin".into())).await.is_err());
}

#[tokio::test]
async fn update_dict_type() {
    let (app, _, _) = common::create_dict_type_app().await;
    let dt = app.create_dict_type(CreateDictTypeRequest {
        name: "旧名称".into(), dict_type: "old_type".into(), remark: None,
    }, Some("admin".into())).await.unwrap();

    let updated = app.update_dict_type(UpdateDictTypeRequest {
        id: dt.id,
        name: "新名称".into(),
        dict_type: "old_type".into(),
        remark: Some("更新备注".into()),
    }, Some("admin".into())).await.unwrap();

    assert_eq!(updated.name, "新名称");
    assert_eq!(updated.remark, Some("更新备注".into()));
}

#[tokio::test]
async fn delete_dict_type() {
    let (app, _, _) = common::create_dict_type_app().await;
    let dt = app.create_dict_type(CreateDictTypeRequest {
        name: "待删除".into(), dict_type: "to_del".into(), remark: None,
    }, Some("admin".into())).await.unwrap();

    app.delete_dict_type(dt.id, Some("admin".into())).await.unwrap();
}

#[tokio::test]
async fn paginate_dict_types() {
    let (app, _, _) = common::create_dict_type_app().await;
    for i in 0..4 {
        app.create_dict_type(CreateDictTypeRequest {
            name: format!("类型{}", i), dict_type: format!("type_{}", i), remark: None,
        }, Some("admin".into())).await.unwrap();
    }
    let page = app.get_dict_type_page(ListDictTypesRequest {
        name: None, dict_type: None, status: None, page: 1, page_size: 2,
    }).await.unwrap();
    assert_eq!(page.list.len(), 2);
    assert_eq!(page.total, 4);
}

#[tokio::test]
async fn get_all_dict_types() {
    let (app, _, _) = common::create_dict_type_app().await;
    app.create_dict_type(CreateDictTypeRequest {
        name: "性别".into(), dict_type: "sex".into(), remark: None,
    }, Some("admin".into())).await.unwrap();
    app.create_dict_type(CreateDictTypeRequest {
        name: "状态".into(), dict_type: "status".into(), remark: None,
    }, Some("admin".into())).await.unwrap();

    let all = app.get_all_dict_types().await.unwrap();
    assert_eq!(all.len(), 2);
}

// ── 8.2 字典项 CRUD ───────────────────────────────────────────────────────

#[tokio::test]
async fn create_dict_data_success() {
    let (app, _, _) = common::create_dict_data_app().await;
    let req = CreateDictDataRequest {
        sort: 1,
        label: "男".into(),
        value: "0".into(),
        dict_type: "sys_user_sex".into(),
        color_type: Some("primary".into()),
        css_class: Some("tag-male".into()),
        remark: None,
    };
    let dd = app.create_dict_data(req, Some("admin".into())).await.unwrap();
    assert_eq!(dd.label, "男");
    assert_eq!(dd.value, "0");
    assert_eq!(dd.dict_type, "sys_user_sex");
    assert_eq!(dd.sort, 1);
}

#[tokio::test]
async fn update_dict_data() {
    let (app, _, _) = common::create_dict_data_app().await;
    let dd = app.create_dict_data(CreateDictDataRequest {
        sort: 1, label: "旧标签".into(), value: "old".into(),
        dict_type: "test".into(), color_type: None, css_class: None, remark: None,
    }, Some("admin".into())).await.unwrap();

    let updated = app.update_dict_data(UpdateDictDataRequest {
        id: dd.id,
        sort: 10,
        label: "新标签".into(),
        value: "new".into(),
        dict_type: "test".into(),
        color_type: Some("success".into()),
        css_class: Some("tag-new".into()),
        remark: Some("更新".into()),
    }, Some("admin".into())).await.unwrap();

    assert_eq!(updated.label, "新标签");
    assert_eq!(updated.sort, 10);
    assert_eq!(updated.color_type, Some("success".into()));
}

#[tokio::test]
async fn delete_dict_data() {
    let (app, _, _) = common::create_dict_data_app().await;
    let dd = app.create_dict_data(CreateDictDataRequest {
        sort: 1, label: "待删除".into(), value: "del".into(),
        dict_type: "test".into(), color_type: None, css_class: None, remark: None,
    }, Some("admin".into())).await.unwrap();

    app.delete_dict_data(dd.id, Some("admin".into())).await.unwrap();
}

#[tokio::test]
async fn get_dict_data_by_type() {
    let (app, _, _) = common::create_dict_data_app().await;
    for (i, label) in ["男", "女", "未知"].iter().enumerate() {
        app.create_dict_data(CreateDictDataRequest {
            sort: i as i32, label: label.to_string(), value: i.to_string(),
            dict_type: "sex".into(), color_type: None, css_class: None, remark: None,
        }, Some("admin".into())).await.unwrap();
    }
    let list = app.get_by_dict_type("sex").await.unwrap();
    assert_eq!(list.len(), 3);
    assert!(list.iter().any(|d| d.label == "男"));
    assert!(list.iter().any(|d| d.label == "女"));
}

#[tokio::test]
async fn paginate_dict_data() {
    let (app, _, _) = common::create_dict_data_app().await;
    for i in 0..5 {
        app.create_dict_data(CreateDictDataRequest {
            sort: i, label: format!("项{}", i), value: i.to_string(),
            dict_type: "test".into(), color_type: None, css_class: None, remark: None,
        }, Some("admin".into())).await.unwrap();
    }
    let page = app.get_dict_data_page(ListDictDataRequest {
        dict_type: Some("test".into()), label: None, status: None, page: 1, page_size: 2,
    }).await.unwrap();
    assert_eq!(page.list.len(), 2);
    assert_eq!(page.total, 5);
}
