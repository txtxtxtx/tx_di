//! 日志管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::log::log_service_server::LogService;
use admin_proto::admin::log::{
    CreateOperateLogRequest, ListOperateLogsRequest, ListOperateLogsResponse,
    CreateLoginLogRequest, ListLoginLogsRequest, ListLoginLogsResponse,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;

#[derive(Debug, Default)]
pub struct LogGrpcService;

#[tonic::async_trait]
impl LogService for LogGrpcService {
    async fn create_operate_log(&self, request: Request<CreateOperateLogRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().oper_log.create_log(req).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_operate_logs(&self, request: Request<ListOperateLogsRequest>) -> Result<Response<ListOperateLogsResponse>, Status> {
        let req = request.into_inner();
        services::get().oper_log.get_log_page(req).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list;
                Response::new(ListOperateLogsResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn create_login_log(&self, request: Request<CreateLoginLogRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().login_log.create_log(req).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_login_logs(&self, request: Request<ListLoginLogsRequest>) -> Result<Response<ListLoginLogsResponse>, Status> {
        let req = request.into_inner();
        services::get().login_log.get_log_page(req).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list;
                Response::new(ListLoginLogsResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }
}
