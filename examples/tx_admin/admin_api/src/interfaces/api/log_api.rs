//! 日志管理 HTTP API

use axum::Json;
use tx_di_axum::Router;
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

async fn create_operate_log(
    DiComp(oper_log): DiComp<OperateLogAppService>,
    Json(req): Json<CreateOperateLogRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("log:view").await?;
    oper_log.create_log(req).await?;
    Ok(ApiRes::ok().into_typed())
}

async fn list_operate_logs(
    DiComp(oper_log): DiComp<OperateLogAppService>,
    Json(req): Json<ListOperateLogsRequest>,
) -> Result<ApiR<Page<OperateLogResponse>>, ApiErr> {
    ensure_permission("log:view").await?;
    let page = oper_log.get_log_page(req).await?;
    Ok(ApiR::success(page))
}

async fn create_login_log(
    DiComp(login_log): DiComp<LoginLogAppService>,
    Json(req): Json<CreateLoginLogRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("log:view").await?;
    login_log.create_log(req).await?;
    Ok(ApiRes::ok().into_typed())
}

async fn list_login_logs(
    DiComp(login_log): DiComp<LoginLogAppService>,
    Json(req): Json<ListLoginLogsRequest>,
) -> Result<ApiR<Page<LoginLogResponse>>, ApiErr> {
    ensure_permission("log:view").await?;
    let page = login_log.get_log_page(req).await?;
    Ok(ApiR::success(page))
}

/// DELETE /api/log/operate
async fn delete_operate_logs(
    DiComp(oper_log): DiComp<OperateLogAppService>,
    Json(req): Json<DeleteLogsRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("log:delete").await?;
    oper_log.delete_logs(&req.ids).await?;
    Ok(ApiRes::ok().into_typed())
}

/// DELETE /api/log/operate/clean
async fn clean_operate_logs(
    DiComp(oper_log): DiComp<OperateLogAppService>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("log:clean").await?;
    oper_log.clean_logs().await?;
    Ok(ApiRes::ok().into_typed())
}

/// DELETE /api/log/login
async fn delete_login_logs(
    DiComp(login_log): DiComp<LoginLogAppService>,
    Json(req): Json<DeleteLogsRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("log:delete").await?;
    login_log.delete_logs(&req.ids).await?;
    Ok(ApiRes::ok().into_typed())
}

/// DELETE /api/log/login/clean
async fn clean_login_logs(
    DiComp(login_log): DiComp<LoginLogAppService>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("log:clean").await?;
    login_log.clean_logs().await?;
    Ok(ApiRes::ok().into_typed())
}
