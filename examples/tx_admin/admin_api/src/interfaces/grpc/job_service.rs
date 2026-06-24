//! 定时任务 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::job::job_service_server::JobService;
use admin_proto::admin::job::job_log_service_server::JobLogService;
use admin_proto::admin::job::{
    ChangeJobStatusRequest, CleanJobLogsRequest, CreateJobRequest, DeleteJobRequest,
    GetJobLogRequest, GetJobRequest, JobLogResponse, JobResponse, ListJobLogsRequest,
    ListJobLogsResponse, ListJobsRequest, ListJobsResponse, RunJobRequest, UpdateJobRequest,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use tx_di_core::App;

use super::auth_interceptor::{self, get_login_id};
use super::err;

// ════════════════════ JobService ════════════════════

#[derive(Clone)]
pub struct JobGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl JobService for JobGrpcService {
    async fn create_job(
        &self,
        request: Request<CreateJobRequest>,
    ) -> Result<Response<JobResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:create").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        let r = svc.create_job(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn update_job(
        &self,
        request: Request<UpdateJobRequest>,
    ) -> Result<Response<JobResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:update").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        let r = svc.update_job(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn delete_job(
        &self,
        request: Request<DeleteJobRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:delete").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        svc.delete_job(req.id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_job(
        &self,
        request: Request<GetJobRequest>,
    ) -> Result<Response<JobResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        let r = svc.get_job(req.id).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn list_jobs(
        &self,
        request: Request<ListJobsRequest>,
    ) -> Result<Response<ListJobsResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        let p = svc.get_job_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListJobsResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }

    async fn change_job_status(
        &self,
        request: Request<ChangeJobStatusRequest>,
    ) -> Result<Response<JobResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:status").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        let r = svc
            .change_status(req.id, req.status, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn run_job(
        &self,
        request: Request<RunJobRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:execute").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        svc.run_job(req.id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }
}

// ════════════════════ JobLogService ════════════════════

#[derive(Clone)]
pub struct JobLogGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl JobLogService for JobLogGrpcService {
    async fn list_job_logs(
        &self,
        request: Request<ListJobLogsRequest>,
    ) -> Result<Response<ListJobLogsResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        let p = svc.get_job_log_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListJobLogsResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }

    async fn get_job_log(
        &self,
        request: Request<GetJobLogRequest>,
    ) -> Result<Response<JobLogResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        let r = svc.get_job_log(req.id).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn clean_job_logs(
        &self,
        request: Request<CleanJobLogsRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "job:delete").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::job::app_service::JobAppService> = self.app.inject();
        svc.clean_job_logs(req.job_id)
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }
}
