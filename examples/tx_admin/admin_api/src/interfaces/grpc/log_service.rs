//! 日志管理 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::log::log_service_server::LogService;
use admin_proto::admin::log::{
    CreateLoginLogRequest, CreateOperateLogRequest, ListLoginLogsRequest, ListLoginLogsResponse,
    ListOperateLogsRequest, ListOperateLogsResponse,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use tx_di_core::App;

use super::auth_interceptor::{self, get_login_id};
use super::err;

#[derive(Clone)]
pub struct LogGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl LogService for LogGrpcService {
    async fn create_operate_log(
        &self,
        request: Request<CreateOperateLogRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "log:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::log::app_service::OperateLogAppService> = self.app.inject();
        svc.create_log(req).await.map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn list_operate_logs(
        &self,
        request: Request<ListOperateLogsRequest>,
    ) -> Result<Response<ListOperateLogsResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "log:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::log::app_service::OperateLogAppService> = self.app.inject();
        let p = svc.get_log_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListOperateLogsResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }

    async fn create_login_log(
        &self,
        request: Request<CreateLoginLogRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "log:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::log::app_service::LoginLogAppService> = self.app.inject();
        svc.create_log(req).await.map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn list_login_logs(
        &self,
        request: Request<ListLoginLogsRequest>,
    ) -> Result<Response<ListLoginLogsResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "log:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::log::app_service::LoginLogAppService> = self.app.inject();
        let p = svc.get_log_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListLoginLogsResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }
}
