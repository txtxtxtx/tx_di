//! 日志管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use axum::routing::{ post, delete};
use tx_di_axum::bound::DiComp;
use admin_app::log::app_service::{OperateLogAppService, LoginLogAppService};
use admin_proto::{CreateOperateLogRequest, ListOperateLogsRequest, OperateLogResponse, CreateLoginLogRequest, ListLoginLogsRequest, LoginLogResponse, DeleteLogsRequest, Empty};

use tx_common::{ApiR, ApiRes, Page};
use crate::auth::ensure_permission;
use crate::error::ApiErr;

pub fn router() -> Router {
    Router::new()
        .route("/operate", post(create_operate_log))
        .route("/operate/list", post(list_operate_logs))
        .route("/login", post(create_login_log))
        .route("/login/list", post(list_login_logs))
        .route("/operate/delete", post(delete_operate_logs))
        .route("/operate/clean", delete(clean_operate_logs))
        .route("/login/delete", post(delete_login_logs))
        .route("/login/clean", delete(clean_login_logs))
}

fn map_oper_log(l: admin_app::log::dto::OperateLogResponse) -> OperateLogResponse { OperateLogResponse { id: l.id, trace_id: l.trace_id, user_id: l.user_id, user_type: l.user_type, log_type: l.log_type, sub_type: l.sub_type, biz_id: l.biz_id, action: l.action, success: l.success, extra: l.extra, request_method: l.request_method, request_url: l.request_url, user_ip: l.user_ip } }
fn map_login_log(l: admin_app::log::dto::LoginLogResponse) -> LoginLogResponse { LoginLogResponse { id: l.id, user_id: l.user_id, user_type: l.user_type, username: l.username, login_ip: l.login_ip, login_type: l.login_type, result: l.result, msg: l.msg } }

async fn create_operate_log(
    DiComp(oper_log): DiComp<OperateLogAppService>,
    Json(req): Json<CreateOperateLogRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("log:view").await?;
    let cmd = admin_app::log::dto::CreateOperateLogCommand { trace_id: req.trace_id, user_id: req.user_id, user_type: req.user_type, log_type: req.log_type, sub_type: req.sub_type, biz_id: req.biz_id, action: req.action, success: req.success, extra: req.extra };
    oper_log.create_log(cmd).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

async fn list_operate_logs(
    DiComp(oper_log): DiComp<OperateLogAppService>,
    Json(req): Json<ListOperateLogsRequest>,
) -> Result<R<Page<OperateLogResponse>>, ApiErr> {
    ensure_permission("log:view").await?;
    let query = admin_app::log::dto::OperateLogQueryRequest { user_id: req.user_id, log_type: req.log_type, sub_type: req.sub_type, success: req.success, begin_time: req.begin_time, end_time: req.end_time, page: req.page, size: req.page_size };
    let page = oper_log.get_log_page(query).await?;
    Ok(R(ApiR::success(Page::new(page.list.into_iter().map(map_oper_log).collect(), page.page, page.size, page.total))))
}

async fn create_login_log(
    DiComp(login_log): DiComp<LoginLogAppService>,
    Json(req): Json<CreateLoginLogRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("log:view").await?;
    let cmd = admin_app::log::dto::CreateLoginLogCommand { user_id: req.user_id, user_type: req.user_type, username: req.username, login_ip: req.login_ip, login_type: req.login_type, result: req.result };
    login_log.create_log(cmd).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

async fn list_login_logs(
    DiComp(login_log): DiComp<LoginLogAppService>,
    Json(req): Json<ListLoginLogsRequest>,
) -> Result<R<Page<LoginLogResponse>>, ApiErr> {
    ensure_permission("log:view").await?;
    let query = admin_app::log::dto::LoginLogQueryRequest { user_id: req.user_id, username: req.username, login_ip: req.login_ip, login_type: req.login_type, result: req.result, begin_time: req.begin_time, end_time: req.end_time, page: req.page, size: req.page_size };
    let page = login_log.get_log_page(query).await?;
    Ok(R(ApiR::success(Page::new(page.list.into_iter().map(map_login_log).collect(), page.page, page.size, page.total))))
}

/// DELETE /api/log/operate
async fn delete_operate_logs(
    DiComp(oper_log): DiComp<OperateLogAppService>,
    Json(req): Json<DeleteLogsRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("log:delete").await?;
    oper_log.delete_logs(&req.ids).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// DELETE /api/log/operate/clean
async fn clean_operate_logs(
    DiComp(oper_log): DiComp<OperateLogAppService>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("log:clean").await?;
    oper_log.clean_logs().await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// DELETE /api/log/login
async fn delete_login_logs(
    DiComp(login_log): DiComp<LoginLogAppService>,
    Json(req): Json<DeleteLogsRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("log:delete").await?;
    login_log.delete_logs(&req.ids).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// DELETE /api/log/login/clean
async fn clean_login_logs(
    DiComp(login_log): DiComp<LoginLogAppService>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("log:clean").await?;
    login_log.clean_logs().await?;
    Ok(R(ApiRes::ok().into_typed()))
}
