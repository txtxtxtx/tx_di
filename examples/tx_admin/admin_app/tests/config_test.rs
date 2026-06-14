//! 配置管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第7节）:
//!   7.1 配置CRUD     ✅
//!   7.2 配置属性     ✅ (Key唯一性/分组/类型)
//!   7.3 配置操作     ✅ (按Key查询)

mod common;
use admin_app::config::dto::*;

// ── 7.1 配置 CRUD ──────────────────────────────────────────────────────────

#[tokio::test]
async fn create_config_success() {
    let (app, _, _) = common::create_config_app().await;
    let cmd = CreateConfigCommand {
        category: "system".into(),
        config_type: 0,
        name: "系统名称".into(),
        config_key: "sys.name".into(),
        value: "Admin System".into(),
        remark: Some("系统显示名称".into()),
    };
    let config = app.create_config(cmd, Some("admin".into())).await.unwrap();
    assert_eq!(config.name, "系统名称");
    assert_eq!(config.config_key, "sys.name");
    assert_eq!(config.value, "Admin System");
    assert_eq!(config.category, "system");
}

#[tokio::test]
async fn create_duplicate_key_should_fail() {
    let (app, _, _) = common::create_config_app().await;
    let cmd = |key: &str, val: &str| CreateConfigCommand {
        category: "sys".into(), config_type: 0,
        name: "名称".into(), config_key: key.into(), value: val.into(), remark: None,
    };
    app.create_config(cmd("dup.key", "v1"), Some("admin".into())).await.unwrap();
    assert!(app.create_config(cmd("dup.key", "v2"), Some("admin".into())).await.is_err());
}

#[tokio::test]
async fn update_config() {
    let (app, _, _) = common::create_config_app().await;
    let cfg = app.create_config(CreateConfigCommand {
        category: "email".into(), config_type: 0,
        name: "SMTP".into(), config_key: "email.smtp".into(),
        value: "smtp.old.com".into(), remark: None,
    }, Some("admin".into())).await.unwrap();

    let updated = app.update_config(UpdateConfigCommand {
        config_id: cfg.id,
        category: "email".into(),
        config_type: 0,
        name: "SMTP服务器".into(),
        config_key: "email.smtp".into(),
        value: "smtp.new.com".into(),
        visible: 1,
        remark: Some("已更新".into()),
    }, Some("admin".into())).await.unwrap();

    assert_eq!(updated.name, "SMTP服务器");
    assert_eq!(updated.value, "smtp.new.com");
}

#[tokio::test]
async fn delete_config() {
    let (app, _, _) = common::create_config_app().await;
    let cfg = app.create_config(CreateConfigCommand {
        category: "test".into(), config_type: 0,
        name: "待删除".into(), config_key: "test.del".into(),
        value: "x".into(), remark: None,
    }, Some("admin".into())).await.unwrap();

    app.delete_config(cfg.id, Some("admin".into())).await.unwrap();
    assert!(app.get_config(cfg.id).await.is_err());
}

#[tokio::test]
async fn paginate_configs() {
    let (app, _, _) = common::create_config_app().await;
    for i in 0..5 {
        app.create_config(CreateConfigCommand {
            category: "test".into(), config_type: 0,
            name: format!("配置{}", i), config_key: format!("test.k{}", i),
            value: format!("v{}", i), remark: None,
        }, Some("admin".into())).await.unwrap();
    }
    let page = app.get_config_page(ConfigQueryRequest {
        category: None, config_key: None, name: None, config_type: None, page: 1, size: 2,
    }).await.unwrap();
    assert_eq!(page.list.len(), 2);
    assert_eq!(page.total, 5);
}

// ── 7.3 按 Key 查询 ───────────────────────────────────────────────────────

#[tokio::test]
async fn get_config_by_key() {
    let (app, _, _) = common::create_config_app().await;
    app.create_config(CreateConfigCommand {
        category: "system".into(), config_type: 0,
        name: "系统名称".into(), config_key: "sys.name".into(),
        value: "MyApp".into(), remark: None,
    }, Some("admin".into())).await.unwrap();

    let cfg = app.get_by_key("sys.name").await.unwrap();
    assert_eq!(cfg.value, "MyApp");
}

#[tokio::test]
async fn get_config_by_key_not_found() {
    let (app, _, _) = common::create_config_app().await;
    assert!(app.get_by_key("nonexistent.key").await.is_err());
}

#[tokio::test]
async fn query_config_by_category() {
    let (app, _, _) = common::create_config_app().await;
    app.create_config(CreateConfigCommand {
        category: "email".into(), config_type: 0, name: "SMTP".into(),
        config_key: "email.smtp".into(), value: "smtp.com".into(), remark: None,
    }, Some("admin".into())).await.unwrap();
    app.create_config(CreateConfigCommand {
        category: "system".into(), config_type: 0, name: "名称".into(),
        config_key: "sys.appname".into(), value: "App".into(), remark: None,
    }, Some("admin".into())).await.unwrap();

    let page = app.get_config_page(ConfigQueryRequest {
        category: Some("email".into()), config_key: None, name: None,
        config_type: None, page: 1, size: 10,
    }).await.unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].config_key, "email.smtp");
}
