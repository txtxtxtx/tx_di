//! 日志管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第9节）:
//!   9.1 操作日志   ✅ (创建/分页/删除/清理)
//!   9.3 登录日志   ✅ (创建/分页/删除/清理)

mod common;
use admin_app::log::dto::*;

// ── 9.1 操作日志 ───────────────────────────────────────────────────────────

#[tokio::test]
async fn create_operate_log() {
    let (app, _, _) = common::create_operate_log_app();
    let log = app.create_log(CreateOperateLogCommand {
        trace_id: "trace-001".into(), user_id: 1, user_type: 1,
        log_type: "操作日志".into(), sub_type: "用户管理".into(),
        biz_id: 100, action: "新增用户".into(), success: 1,
        extra: r#"{"username":"test"}"#.into(),
    }).await.unwrap();
    assert_eq!(log.trace_id, "trace-001");
    assert_eq!(log.user_id, 1);
    assert_eq!(log.log_type, "操作日志");
    assert_eq!(log.sub_type, "用户管理");
    assert_eq!(log.action, "新增用户");
    assert_eq!(log.success, 1);
}

#[tokio::test]
async fn create_operate_log_failure() {
    let (app, _, _) = common::create_operate_log_app();
    let log = app.create_log(CreateOperateLogCommand {
        trace_id: "trace-002".into(), user_id: 2, user_type: 1,
        log_type: "操作日志".into(), sub_type: "角色管理".into(),
        biz_id: 200, action: "删除角色".into(), success: 0,
        extra: r#"{"error":"角色不存在"}"#.into(),
    }).await.unwrap();
    assert_eq!(log.success, 0);
}

#[tokio::test]
async fn paginate_operate_logs() {
    let (app, _, _) = common::create_operate_log_app();
    for i in 1..=5 {
        app.create_log(CreateOperateLogCommand {
            trace_id: format!("trace-{}", i), user_id: i as u64, user_type: 1,
            log_type: "测试".into(), sub_type: "操作".into(),
            biz_id: i as u64, action: "操作".into(), success: 1, extra: "{}".into(),
        }).await.unwrap();
    }
    let page = app.get_log_page(OperateLogQueryRequest {
        user_id: None, log_type: None, sub_type: None, success: None,
        begin_time: None, end_time: None, page: 1, page_size: 2,
    }).await.unwrap();
    assert_eq!(page.list.len(), 2);
    assert_eq!(page.total, 5);
}

#[tokio::test]
async fn query_operate_logs_by_sub_type() {
    let (app, _, _) = common::create_operate_log_app();
    app.create_log(CreateOperateLogCommand {
        trace_id: "t1".into(), user_id: 1, user_type: 1,
        log_type: "操作".into(), sub_type: "用户管理".into(),
        biz_id: 1, action: "创建".into(), success: 1, extra: "".into(),
    }).await.unwrap();
    app.create_log(CreateOperateLogCommand {
        trace_id: "t2".into(), user_id: 1, user_type: 1,
        log_type: "操作".into(), sub_type: "角色管理".into(),
        biz_id: 2, action: "创建".into(), success: 1, extra: "".into(),
    }).await.unwrap();

    let page = app.get_log_page(OperateLogQueryRequest {
        user_id: None, log_type: None, sub_type: Some("用户管理".into()),
        success: None, begin_time: None, end_time: None, page: 1, page_size: 10,
    }).await.unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].sub_type, "用户管理");
}

#[tokio::test]
async fn delete_operate_logs() {
    let (app, _, _) = common::create_operate_log_app();
    app.create_log(CreateOperateLogCommand {
        trace_id: "t".into(), user_id: 1, user_type: 1,
        log_type: "m".into(), sub_type: "m".into(),
        biz_id: 1, action: "op".into(), success: 1, extra: "".into(),
    }).await.unwrap();
    app.delete_logs(&[1]).await.unwrap();
}

#[tokio::test]
async fn clean_operate_logs() {
    let (app, _, _) = common::create_operate_log_app();
    for i in 1..=3 {
        app.create_log(CreateOperateLogCommand {
            trace_id: format!("t{}", i), user_id: i as u64, user_type: 1,
            log_type: "m".into(), sub_type: "m".into(),
            biz_id: i as u64, action: "op".into(), success: 1, extra: "".into(),
        }).await.unwrap();
    }
    app.clean_logs().await.unwrap();
    let page = app.get_log_page(OperateLogQueryRequest {
        user_id: None, log_type: None, sub_type: None, success: None,
        begin_time: None, end_time: None, page: 1, page_size: 10,
    }).await.unwrap();
    assert_eq!(page.total, 0);
}

// ── 9.3 登录日志 ───────────────────────────────────────────────────────────

#[tokio::test]
async fn create_login_log_success() {
    let (app, _, _) = common::create_login_log_app();
    let log = app.create_log(CreateLoginLogCommand {
        user_id: 1, user_type: 1, username: "admin".into(),
        login_ip: "10.0.0.1".into(), login_type: "password".into(), result: 1,
    }).await.unwrap();
    assert_eq!(log.username, "admin");
    assert_eq!(log.login_ip, "10.0.0.1");
    assert_eq!(log.result, 1);
}

#[tokio::test]
async fn create_login_log_failure() {
    let (app, _, _) = common::create_login_log_app();
    let log = app.create_log(CreateLoginLogCommand {
        user_id: 0, user_type: 0, username: "hacker".into(),
        login_ip: "1.2.3.4".into(), login_type: "password".into(), result: 0,
    }).await.unwrap();
    assert_eq!(log.result, 0);
    assert_eq!(log.username, "hacker");
}

#[tokio::test]
async fn paginate_login_logs() {
    let (app, _, _) = common::create_login_log_app();
    for i in 1..=5 {
        app.create_log(CreateLoginLogCommand {
            user_id: i as u64, user_type: 1, username: format!("u{}", i),
            login_ip: "127.0.0.1".into(), login_type: "password".into(), result: 1,
        }).await.unwrap();
    }
    let page = app.get_log_page(LoginLogQueryRequest {
        user_id: None, username: None, login_ip: None, login_type: None,
        result: None, begin_time: None, end_time: None, page: 1, page_size: 2,
    }).await.unwrap();
    assert_eq!(page.list.len(), 2);
    assert_eq!(page.total, 5);
}

#[tokio::test]
async fn query_login_logs_by_result() {
    let (app, _, _) = common::create_login_log_app();
    app.create_log(CreateLoginLogCommand {
        user_id: 1, user_type: 1, username: "u1".into(),
        login_ip: "1.1.1.1".into(), login_type: "password".into(), result: 1,
    }).await.unwrap();
    app.create_log(CreateLoginLogCommand {
        user_id: 2, user_type: 1, username: "u2".into(),
        login_ip: "2.2.2.2".into(), login_type: "password".into(), result: 0,
    }).await.unwrap();

    let page = app.get_log_page(LoginLogQueryRequest {
        user_id: None, username: None, login_ip: None, login_type: None,
        result: Some(0), begin_time: None, end_time: None, page: 1, page_size: 10,
    }).await.unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].result, 0);
}

#[tokio::test]
async fn delete_login_logs() {
    let (app, _, _) = common::create_login_log_app();
    app.create_log(CreateLoginLogCommand {
        user_id: 1, user_type: 1, username: "u1".into(),
        login_ip: "127.0.0.1".into(), login_type: "password".into(), result: 1,
    }).await.unwrap();
    app.delete_logs(&[1]).await.unwrap();
}

#[tokio::test]
async fn clean_login_logs() {
    let (app, _, _) = common::create_login_log_app();
    for i in 1..=3 {
        app.create_log(CreateLoginLogCommand {
            user_id: i as u64, user_type: 1, username: format!("u{}", i),
            login_ip: "127.0.0.1".into(), login_type: "password".into(), result: 1,
        }).await.unwrap();
    }
    app.clean_logs().await.unwrap();
    let page = app.get_log_page(LoginLogQueryRequest {
        user_id: None, username: None, login_ip: None, login_type: None,
        result: None, begin_time: None, end_time: None, page: 1, page_size: 10,
    }).await.unwrap();
    assert_eq!(page.total, 0);
}
