//! 定时任务集成测试
//!
//! 覆盖功能（参照测试说明书 G-1 缺口补齐）:
//!   - Job CRUD（创建/更新/删除/查询/分页）
//!   - Job 状态变更（暂停/运行）
//!   - Job 手动执行 + 日志记录（成功/失败路径）
//!   - Job 日志查询/清理

mod common;
use admin_proto::{CreateJobRequest, ListJobLogsRequest, ListJobsRequest, UpdateJobRequest};
use tx_di_job::ExecutionStatus;

fn create_job_req(name: &str, handler: &str) -> CreateJobRequest {
    CreateJobRequest {
        name: name.into(),
        handler_name: handler.into(),
        handler_param: Some(r#"{"key":"value"}"#.into()),
        cron_expression: "0 * * * * *".into(),
        retry_count: 3,
        retry_interval: 10,
        monitor_timeout: 60,
    }
}

// ── Job CRUD ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_job_success() {
    let (app, _) = common::create_job_app().await;
    let job = app
        .create_job(
            create_job_req("测试任务", "test_handler"),
            Some("admin".into()),
        )
        .await
        .unwrap();

    // 返回值正确性
    assert_eq!(job.name, "测试任务");
    assert_eq!(job.handler_name, "test_handler");
    assert_eq!(job.retry_count, 3);
    assert_eq!(job.retry_interval, 10);
    assert_eq!(job.monitor_timeout, 60);
    assert!(job.id > 0, "雪花 ID 应大于 0");

    // 持久化验证：回查确认落库
    let found = app.get_job(job.id).await.unwrap();
    assert_eq!(found.name, "测试任务");
    assert_eq!(found.handler_name, "test_handler");
    assert_eq!(found.cron_expression, "0 * * * * *");
}

#[tokio::test]
async fn update_job_success() {
    let (app, _) = common::create_job_app().await;
    let job = app
        .create_job(create_job_req("原始任务", "handler1"), Some("admin".into()))
        .await
        .unwrap();

    let updated = app
        .update_job(
            UpdateJobRequest {
                id: job.id,
                name: "更新任务".into(),
                handler_name: "handler2".into(),
                handler_param: Some(r#"{"k":"v"}"#.into()),
                cron_expression: "*/5 * * * * *".into(),
                retry_count: 5,
                retry_interval: 30,
                monitor_timeout: 120,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    assert_eq!(updated.name, "更新任务");
    assert_eq!(updated.handler_name, "handler2");
    assert_eq!(updated.retry_count, 5);
    assert_eq!(updated.retry_interval, 30);
    assert_eq!(updated.monitor_timeout, 120);

    // 持久化验证
    let found = app.get_job(job.id).await.unwrap();
    assert_eq!(found.name, "更新任务");
    assert_eq!(found.cron_expression, "*/5 * * * * *");
}

#[tokio::test]
async fn delete_job_soft_delete() {
    let (app, _) = common::create_job_app().await;
    let job = app
        .create_job(
            create_job_req("待删除", "del_handler"),
            Some("admin".into()),
        )
        .await
        .unwrap();

    app.delete_job(job.id, Some("admin".into())).await.unwrap();

    // 软删除后查询应失败
    assert!(app.get_job(job.id).await.is_err());
}

#[tokio::test]
async fn get_job_not_found() {
    let (app, _) = common::create_job_app().await;
    assert!(app.get_job(999999).await.is_err());
}

#[tokio::test]
async fn get_job_page_pagination() {
    let (app, _) = common::create_job_app().await;
    for i in 1..=5 {
        app.create_job(
            create_job_req(&format!("任务{}", i), &format!("h{}", i)),
            Some("admin".into()),
        )
        .await
        .unwrap();
    }

    let page = app
        .get_job_page(ListJobsRequest {
            name: None,
            status: None,
            page: 1,
            page_size: 2,
        })
        .await
        .unwrap();

    assert_eq!(page.list.len(), 2);
    assert_eq!(page.total, 5);
}

#[tokio::test]
async fn get_job_page_filter_by_name() {
    let (app, _) = common::create_job_app().await;
    app.create_job(create_job_req("备份任务", "backup"), Some("admin".into()))
        .await
        .unwrap();
    app.create_job(create_job_req("清理任务", "cleanup"), Some("admin".into()))
        .await
        .unwrap();

    let page = app
        .get_job_page(ListJobsRequest {
            name: Some("备份".into()),
            status: None,
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();

    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].name, "备份任务");
}

// ── Job 状态变更 ────────────────────────────────────────────────────────────

#[tokio::test]
async fn change_status_pause_and_resume() {
    let (app, _) = common::create_job_app().await;
    let job = app
        .create_job(
            create_job_req("状态任务", "st_handler"),
            Some("admin".into()),
        )
        .await
        .unwrap();

    // 新建任务默认 Running(1)，暂停后应为 Paused(0)
    assert_eq!(job.status, 1);

    let paused = app
        .change_status(job.id, 0, Some("admin".into()))
        .await
        .unwrap();
    assert_eq!(paused.status, 0);

    // 持久化验证
    let found = app.get_job(job.id).await.unwrap();
    assert_eq!(found.status, 0);

    // 恢复运行
    let resumed = app
        .change_status(job.id, 1, Some("admin".into()))
        .await
        .unwrap();
    assert_eq!(resumed.status, 1);
}

#[tokio::test]
async fn get_job_page_filter_by_status() {
    let (app, _) = common::create_job_app().await;
    let j1 = app
        .create_job(create_job_req("运行中", "h1"), Some("admin".into()))
        .await
        .unwrap();
    let j2 = app
        .create_job(create_job_req("暂停中", "h2"), Some("admin".into()))
        .await
        .unwrap();
    app.change_status(j2.id, 0, Some("admin".into()))
        .await
        .unwrap();

    // 只查暂停的任务 (status=0)
    let page = app
        .get_job_page(ListJobsRequest {
            name: None,
            status: Some(0),
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();

    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].name, "暂停中");

    // 只查运行中的任务 (status=1)
    let page = app
        .get_job_page(ListJobsRequest {
            name: None,
            status: Some(1),
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();

    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].id, j1.id);
}

// ── Job 手动执行 + 日志 ────────────────────────────────────────────────────

#[tokio::test]
async fn run_job_success_logs_ok() {
    let (app, job_plugin) = common::create_job_app().await;

    // 注册一个成功的测试 handler
    job_plugin.register_handler("test_ok", |_param| tx_di_job::JobResult {
        status: ExecutionStatus::Success,
        result: Some("执行成功".to_string()),
        error: None,
    });

    let job = app
        .create_job(create_job_req("成功任务", "test_ok"), Some("admin".into()))
        .await
        .unwrap();

    // 手动执行
    app.run_job(job.id, Some("admin".into())).await.unwrap();

    // 验证执行日志已落库（状态=成功）
    let log_page = app
        .get_job_log_page(ListJobLogsRequest {
            job_id: Some(job.id),
            status: None,
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();

    assert_eq!(log_page.total, 1, "应有一条执行日志");
    let log = &log_page.list[0];
    assert_eq!(log.job_id, job.id);
    assert_eq!(log.status, ExecutionStatus::Success as i32);
    assert!(log.end_time.is_some(), "成功执行应有结束时间");
    assert!(log.result.is_some(), "成功执行应有结果");
}

#[tokio::test]
async fn run_job_handler_not_found_logs_failure() {
    let (app, _) = common::create_job_app().await;

    // 不注册任何 handler，直接执行
    let job = app
        .create_job(
            create_job_req("失败任务", "nonexistent_handler"),
            Some("admin".into()),
        )
        .await
        .unwrap();

    // run_job 内部捕获执行失败，不会返回 Err（失败状态记录在日志中）
    app.run_job(job.id, Some("admin".into())).await.unwrap();

    // 验证执行日志记录了失败
    let log_page = app
        .get_job_log_page(ListJobLogsRequest {
            job_id: Some(job.id),
            status: Some(ExecutionStatus::Failed as i32),
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();

    assert_eq!(log_page.total, 1, "应有一条失败日志");
    let log = &log_page.list[0];
    assert_eq!(log.status, ExecutionStatus::Failed as i32);
    assert!(log.result.is_some(), "失败日志应包含错误信息");
}

#[tokio::test]
async fn get_job_log_by_id() {
    let (app, job_plugin) = common::create_job_app().await;
    job_plugin.register_handler("log_test", |_param| tx_di_job::JobResult {
        status: ExecutionStatus::Success,
        result: Some("ok".to_string()),
        error: None,
    });

    let job = app
        .create_job(create_job_req("日志任务", "log_test"), Some("admin".into()))
        .await
        .unwrap();
    app.run_job(job.id, Some("admin".into())).await.unwrap();

    // 从分页结果拿到 log id，再按 id 查详情
    let page = app
        .get_job_log_page(ListJobLogsRequest {
            job_id: Some(job.id),
            status: None,
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(page.total, 1);

    let log = app.get_job_log(page.list[0].id).await.unwrap();
    assert_eq!(log.job_id, job.id);
    assert_eq!(log.handler_name, "log_test");
}

#[tokio::test]
async fn clean_job_logs_by_job() {
    let (app, job_plugin) = common::create_job_app().await;
    job_plugin.register_handler("clean_test", |_param| tx_di_job::JobResult {
        status: ExecutionStatus::Success,
        result: Some("ok".to_string()),
        error: None,
    });

    let job = app
        .create_job(
            create_job_req("清理日志任务", "clean_test"),
            Some("admin".into()),
        )
        .await
        .unwrap();
    app.run_job(job.id, Some("admin".into())).await.unwrap();

    // 清理前有日志
    let before = app
        .get_job_log_page(ListJobLogsRequest {
            job_id: Some(job.id),
            status: None,
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(before.total, 1);

    // 清理指定任务的日志
    app.clean_job_logs(Some(job.id)).await.unwrap();

    // 清理后无日志
    let after = app
        .get_job_log_page(ListJobLogsRequest {
            job_id: Some(job.id),
            status: None,
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(after.total, 0);
}

#[tokio::test]
async fn clean_all_job_logs() {
    let (app, job_plugin) = common::create_job_app().await;
    job_plugin.register_handler("clean_all", |_param| tx_di_job::JobResult {
        status: ExecutionStatus::Success,
        result: Some("ok".to_string()),
        error: None,
    });

    let j1 = app
        .create_job(create_job_req("任务A", "clean_all"), Some("admin".into()))
        .await
        .unwrap();
    let j2 = app
        .create_job(create_job_req("任务B", "clean_all"), Some("admin".into()))
        .await
        .unwrap();
    app.run_job(j1.id, Some("admin".into())).await.unwrap();
    app.run_job(j2.id, Some("admin".into())).await.unwrap();

    // 清理所有日志（job_id=None）
    app.clean_job_logs(None).await.unwrap();

    let total = app
        .get_job_log_page(ListJobLogsRequest {
            job_id: None,
            status: None,
            page: 1,
            page_size: 100,
        })
        .await
        .unwrap();
    assert_eq!(total.total, 0, "清空后应无任何日志");
}
