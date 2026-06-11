//! 日志管理 HTTP API
//!
//! 包含操作日志和登录日志两部分。

use axum::{Json, Router, extract::State, routing::{get, post}};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateOperateLogRequest, ListOperateLogsRequest, OperateLogResponse,
    CreateLoginLogRequest, ListLoginLogsRequest, LoginLogResponse, Empty,
};
use crate::interfaces::dto::{ApiResponse, PageResponse};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        // ── 操作日志 ──
        .route("/operate", post(create_operate_log))
        .route("/operate/{id}", get(get_operate_log))
        .route("/operate/list", post(list_operate_logs))
        // ── 登录日志 ──
        .route("/login", post(create_login_log))
        .route("/login/{id}", get(get_login_log))
        .route("/login/list", post(list_login_logs))
        .with_state(app)
}

// ══════════════════════════════════════════════════════════════
// 操作日志
// ══════════════════════════════════════════════════════════════

/// POST /api/log/operate
async fn create_operate_log(
    State(_app): State<Arc<App>>,
    Json(_req): Json<CreateOperateLogRequest>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 LogAppService::create_operate_log
    Ok(Json(ApiResponse::success(Empty {})))
}

/// GET /api/log/operate/{id}
async fn get_operate_log(
    State(_app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<OperateLogResponse>>, tx_error::AppError> {
    // TODO: 调用 LogAppService::get_operate_log
    let resp = OperateLogResponse {
        id,
        trace_id: String::new(),
        user_id: 0,
        user_type: 0,
        log_type: String::new(),
        sub_type: String::new(),
        biz_id: 0,
        action: String::new(),
        success: 0,
        extra: String::new(),
        request_method: None,
        request_url: None,
        user_ip: None,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// POST /api/log/operate/list
async fn list_operate_logs(
    State(_app): State<Arc<App>>,
    Json(req): Json<ListOperateLogsRequest>,
) -> Result<Json<ApiResponse<PageResponse<OperateLogResponse>>>, tx_error::AppError> {
    // TODO: 调用 LogAppService::list_operate_logs
    let page = PageResponse {
        list: vec![],
        total: 0,
        page: req.page,
        size: req.page_size,
    };
    Ok(Json(ApiResponse::success(page)))
}

// ══════════════════════════════════════════════════════════════
// 登录日志
// ══════════════════════════════════════════════════════════════

/// POST /api/log/login
async fn create_login_log(
    State(_app): State<Arc<App>>,
    Json(_req): Json<CreateLoginLogRequest>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 LogAppService::create_login_log
    Ok(Json(ApiResponse::success(Empty {})))
}

/// GET /api/log/login/{id}
async fn get_login_log(
    State(_app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<LoginLogResponse>>, tx_error::AppError> {
    // TODO: 调用 LogAppService::get_login_log
    let resp = LoginLogResponse {
        id,
        user_id: 0,
        user_type: 0,
        username: String::new(),
        login_ip: String::new(),
        login_type: String::new(),
        result: 0,
        msg: None,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// POST /api/log/login/list
async fn list_login_logs(
    State(_app): State<Arc<App>>,
    Json(req): Json<ListLoginLogsRequest>,
) -> Result<Json<ApiResponse<PageResponse<LoginLogResponse>>>, tx_error::AppError> {
    // TODO: 调用 LogAppService::list_login_logs
    let page = PageResponse {
        list: vec![],
        total: 0,
        page: req.page,
        size: req.page_size,
    };
    Ok(Json(ApiResponse::success(page)))
}
