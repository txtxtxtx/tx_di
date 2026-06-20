//! 定时任务 HTTP API

use axum::Json;
use tx_di_sa_token::StpUtil;
use tx_di_axum::Router;
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::job::app_service::JobAppService;
use admin_proto::{
    CreateJobRequest, UpdateJobRequest, ListJobsRequest, JobResponse,
    ChangeJobStatusRequest, Empty,
    ListJobLogsRequest, JobLogResponse, CleanJobLogsRequest,
};
use tx_common::{ApiR, ApiRes, Page};
use crate::auth::ensure_permission;
use crate::error::ApiErr;

pub fn router() -> Router {
    let job_routes = Router::new()
        .route("/list", post(list_jobs))
        .route("/{id}", get(get_job))
        .route("/", post(create_job))
        .route("/{id}", put(update_job))
        .route("/{id}", delete(delete_job))
        .route("/{id}/status", put(change_job_status))
        .route("/{id}/run", post(run_job));

    let log_routes = Router::new()
        .route("/list", post(list_job_logs))
        .route("/{id}", get(get_job_log))
        .route("/clean", delete(clean_job_logs));

    job_routes.nest("/log", log_routes)
}

/// POST /api/job — 创建定时任务
async fn create_job(
    DiComp(job_svc): DiComp<JobAppService>,
    Json(req): Json<CreateJobRequest>,
) -> Result<ApiR<JobResponse>, ApiErr> {
    ensure_permission("job:create").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = job_svc.create_job(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

/// GET /api/job/{id} — 获取定时任务详情
async fn get_job(
    DiComp(job_svc): DiComp<JobAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<ApiR<JobResponse>, ApiErr> {
    ensure_permission("job:view").await?;
    let r = job_svc.get_job(id).await?;
    Ok(ApiR::success(r))
}

/// PUT /api/job/{id} — 更新定时任务
async fn update_job(
    DiComp(job_svc): DiComp<JobAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateJobRequest>,
) -> Result<ApiR<JobResponse>, ApiErr> {
    ensure_permission("job:update").await?;
    req.id = id;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = job_svc.update_job(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

/// DELETE /api/job/{id} — 删除定时任务
async fn delete_job(
    DiComp(job_svc): DiComp<JobAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("job:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    job_svc.delete_job(id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

/// POST /api/job/list — 分页查询定时任务列表
async fn list_jobs(
    DiComp(job_svc): DiComp<JobAppService>,
    Json(req): Json<ListJobsRequest>,
) -> Result<ApiR<Page<JobResponse>>, ApiErr> {
    ensure_permission("job:view").await?;
    let page = job_svc.get_job_page(req).await?;
    Ok(ApiR::success(page))
}

/// PUT /api/job/{id}/status — 变更定时任务状态
async fn change_job_status(
    DiComp(job_svc): DiComp<JobAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut req): Json<ChangeJobStatusRequest>,
) -> Result<ApiR<JobResponse>, ApiErr> {
    ensure_permission("job:update").await?;
    req.id = id;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = job_svc.change_status(req.id, req.status, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

/// POST /api/job/{id}/run — 手动执行定时任务（暂仅返回成功，调度逻辑后续实现）
async fn run_job(
    DiComp(_job_svc): DiComp<JobAppService>,
    axum::extract::Path(_id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("job:update").await?;
    // TODO: 接入调度器执行任务
    Ok(ApiRes::ok().into_typed())
}

/// POST /api/job/log/list — 分页查询任务执行日志
async fn list_job_logs(
    DiComp(job_svc): DiComp<JobAppService>,
    Json(req): Json<ListJobLogsRequest>,
) -> Result<ApiR<Page<JobLogResponse>>, ApiErr> {
    ensure_permission("job:view").await?;
    let page = job_svc.get_job_log_page(req).await?;
    Ok(ApiR::success(page))
}

/// GET /api/job/log/{id} — 获取任务执行日志详情
async fn get_job_log(
    DiComp(job_svc): DiComp<JobAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<ApiR<JobLogResponse>, ApiErr> {
    ensure_permission("job:view").await?;
    let r = job_svc.get_job_log(id).await?;
    Ok(ApiR::success(r))
}

/// DELETE /api/job/log/clean — 清空任务执行日志
async fn clean_job_logs(
    DiComp(job_svc): DiComp<JobAppService>,
    Json(req): Json<CleanJobLogsRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("job:delete").await?;
    job_svc.clean_job_logs(req.job_id).await?;
    Ok(ApiRes::ok().into_typed())
}
