//! 日志管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post}};
use axum::response::IntoResponse;
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateOperateLogRequest, ListOperateLogsRequest, OperateLogResponse,
    CreateLoginLogRequest, ListLoginLogsRequest, LoginLogResponse, Empty,
};
use crate::services;
use tx_common::{ApiR, ApiRes, Page};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/operate", post(create_operate_log))
        .route("/operate/list", post(list_operate_logs))
        .route("/login", post(create_login_log))
        .route("/login/list", post(list_login_logs))
        .with_state(app)
}

// ── 操作日志 ──

fn map_oper_log(l: admin_app::log::dto::OperateLogResponse) -> OperateLogResponse {
    OperateLogResponse {
        id: l.id, trace_id: l.trace_id, user_id: l.user_id, user_type: l.user_type,
        log_type: l.log_type, sub_type: l.sub_type, biz_id: l.biz_id,
        action: l.action, success: l.success, extra: l.extra,
        request_method: l.request_method, request_url: l.request_url, user_ip: l.user_ip,
    }
}

async fn create_operate_log(Json(req): Json<CreateOperateLogRequest>) -> impl IntoResponse {
    let cmd = admin_app::log::dto::CreateOperateLogCommand {
        trace_id: req.trace_id, user_id: req.user_id, user_type: req.user_type,
        log_type: req.log_type, sub_type: req.sub_type, biz_id: req.biz_id,
        action: req.action, success: req.success, extra: req.extra,
    };
    match services::get().oper_log.create_log(cmd).await {
        Ok(_) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

async fn list_operate_logs(Json(req): Json<ListOperateLogsRequest>) -> impl IntoResponse {
    let q = admin_app::log::dto::OperateLogQueryRequest {
        user_id: req.user_id, log_type: req.log_type, sub_type: req.sub_type,
        success: req.success, begin_time: req.begin_time, end_time: req.end_time,
        page: req.page, size: req.page_size,
    };
    match services::get().oper_log.get_log_page(q).await {
        Ok(page) => ApiR::success(Page::new(
            page.list.into_iter().map(map_oper_log).collect(),
            page.page, page.size, page.total,
        )),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

// ── 登录日志 ──

fn map_login_log(l: admin_app::log::dto::LoginLogResponse) -> LoginLogResponse {
    LoginLogResponse {
        id: l.id, user_id: l.user_id, user_type: l.user_type,
        username: l.username, login_ip: l.login_ip,
        login_type: l.login_type, result: l.result, msg: l.msg,
    }
}

async fn create_login_log(Json(req): Json<CreateLoginLogRequest>) -> impl IntoResponse {
    let cmd = admin_app::log::dto::CreateLoginLogCommand {
        user_id: req.user_id, user_type: req.user_type,
        username: req.username, login_ip: req.login_ip,
        login_type: req.login_type, result: req.result,
    };
    match services::get().login_log.create_log(cmd).await {
        Ok(_) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

async fn list_login_logs(Json(req): Json<ListLoginLogsRequest>) -> impl IntoResponse {
    let q = admin_app::log::dto::LoginLogQueryRequest {
        user_id: req.user_id, username: req.username, login_ip: req.login_ip,
        login_type: req.login_type, result: req.result,
        begin_time: req.begin_time, end_time: req.end_time,
        page: req.page, size: req.page_size,
    };
    match services::get().login_log.get_log_page(q).await {
        Ok(page) => ApiR::success(Page::new(
            page.list.into_iter().map(map_login_log).collect(),
            page.page, page.size, page.total,
        )),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}
