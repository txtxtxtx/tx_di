//! 日志管理 API
//!
//! 包含登录日志和操作日志的查询接口（只读）。

use axum::{Json, Router, extract::{Query, State}, routing::get};
use std::sync::Arc;
use tx_di_core::App;

use crate::domain::AppError;
use crate::domain::login_log::LoginLogRepository;
use crate::domain::login_log::repo::ToastyLoginLogRepository;
use crate::domain::operate_log::OperateLogRepository;
use crate::domain::operate_log::repo::ToastyOperateLogRepository;
use crate::interfaces::dto::common::{ApiResponse, PageQuery, PageResponse};
use crate::interfaces::dto::log_dto::{LoginLogDto, OperateLogDto};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        // 登录日志
        .route("/login", get(list_login_logs))
        // 操作日志
        .route("/operate", get(list_operate_logs))
        .with_state(app)
}

// ── 登录日志 ─────────────────────────────────────────────

async fn list_login_logs(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiResponse<PageResponse<LoginLogDto>>>, AppError> {
    let repo = app.inject::<ToastyLoginLogRepository>();
    // TODO: tenant_id 应从 sa-token 获取
    let (logs, total) = repo.find_page(1, query.keyword.as_deref(), query.page, query.page_size).await?;
    let dtos: Vec<LoginLogDto> = logs.iter().map(LoginLogDto::from).collect();
    Ok(Json(ApiResponse::success(PageResponse::new(dtos, total, query.page, query.page_size))))
}

// ── 操作日志 ─────────────────────────────────────────────

async fn list_operate_logs(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiResponse<PageResponse<OperateLogDto>>>, AppError> {
    let repo = app.inject::<ToastyOperateLogRepository>();
    // TODO: tenant_id 应从 sa-token 获取
    let (logs, total) = repo.find_page(1, query.page, query.page_size).await?;
    let dtos: Vec<OperateLogDto> = logs.iter().map(OperateLogDto::from).collect();
    Ok(Json(ApiResponse::success(PageResponse::new(dtos, total, query.page, query.page_size))))
}
