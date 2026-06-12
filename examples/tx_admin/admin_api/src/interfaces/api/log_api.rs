//! 日志管理 HTTP API

use axum::{Json, Router, routing::{get, post}};
use axum::response::IntoResponse;
use tx_di_axum::bound::DiComp;
use admin_app::log::app_service::{OperateLogAppService, LoginLogAppService};
use admin_proto::{CreateOperateLogRequest, ListOperateLogsRequest, OperateLogResponse, CreateLoginLogRequest, ListLoginLogsRequest, LoginLogResponse};
use tx_common::{ApiR, ApiRes, Page};

pub fn router() -> Router {
    Router::new()
        .route("/operate", post(create_operate_log))
        .route("/operate/list", post(list_operate_logs))
        .route("/login", post(create_login_log))
        .route("/login/list", post(list_login_logs))
}

fn map_oper_log(l: admin_app::log::dto::OperateLogResponse) -> OperateLogResponse { OperateLogResponse { id: l.id, trace_id: l.trace_id, user_id: l.user_id, user_type: l.user_type, log_type: l.log_type, sub_type: l.sub_type, biz_id: l.biz_id, action: l.action, success: l.success, extra: l.extra, request_method: l.request_method, request_url: l.request_url, user_ip: l.user_ip } }
fn map_login_log(l: admin_app::log::dto::LoginLogResponse) -> LoginLogResponse { LoginLogResponse { id: l.id, user_id: l.user_id, user_type: l.user_type, username: l.username, login_ip: l.login_ip, login_type: l.login_type, result: l.result, msg: l.msg } }

async fn create_operate_log(DiComp(oper_log): DiComp<OperateLogAppService>, Json(req): Json<CreateOperateLogRequest>) -> impl IntoResponse {
    let cmd = admin_app::log::dto::CreateOperateLogCommand { trace_id: req.trace_id, user_id: req.user_id, user_type: req.user_type, log_type: req.log_type, sub_type: req.sub_type, biz_id: req.biz_id, action: req.action, success: req.success, extra: req.extra };
    match oper_log.create_log(cmd).await { Ok(_) => ApiRes::ok(), Err(e) => ApiRes::from(e) }
}

async fn list_operate_logs(DiComp(oper_log): DiComp<OperateLogAppService>, Json(req): Json<ListOperateLogsRequest>) -> impl IntoResponse {
    let query = admin_app::log::dto::OperateLogQueryRequest { user_id: req.user_id, log_type: req.log_type, sub_type: req.sub_type, success: req.success, begin_time: req.begin_time, end_time: req.end_time, page: req.page, size: req.page_size };
    match oper_log.get_log_page(query).await { Ok(page) => ApiR::success(Page::new(page.list.into_iter().map(map_oper_log).collect(), page.page, page.size, page.total)), Err(e) => ApiRes::from(e).into_typed() }
}

async fn create_login_log(DiComp(login_log): DiComp<LoginLogAppService>, Json(req): Json<CreateLoginLogRequest>) -> impl IntoResponse {
    let cmd = admin_app::log::dto::CreateLoginLogCommand { user_id: req.user_id, user_type: req.user_type, username: req.username, login_ip: req.login_ip, login_type: req.login_type, result: req.result };
    match login_log.create_log(cmd).await { Ok(_) => ApiRes::ok(), Err(e) => ApiRes::from(e) }
}

async fn list_login_logs(DiComp(login_log): DiComp<LoginLogAppService>, Json(req): Json<ListLoginLogsRequest>) -> impl IntoResponse {
    let query = admin_app::log::dto::LoginLogQueryRequest { user_id: req.user_id, username: req.username, login_ip: req.login_ip, login_type: req.login_type, result: req.result, begin_time: req.begin_time, end_time: req.end_time, page: req.page, size: req.page_size };
    match login_log.get_log_page(query).await { Ok(page) => ApiR::success(Page::new(page.list.into_iter().map(map_login_log).collect(), page.page, page.size, page.total)), Err(e) => ApiRes::from(e).into_typed() }
}
