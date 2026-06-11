//! 日志管理 gRPC 服务实现
//!
//! 包含操作日志和登录日志两部分。

use tonic::{Request, Response, Status};

use admin_proto::admin::log::log_service_server::LogService;
use admin_proto::admin::log::{
    CreateOperateLogRequest, ListOperateLogsRequest, ListOperateLogsResponse,
    CreateLoginLogRequest, ListLoginLogsRequest, ListLoginLogsResponse,
};
use admin_proto::Empty;

/// 日志 gRPC 服务
#[derive(Debug, Default)]
pub struct LogGrpcService;

#[tonic::async_trait]
impl LogService for LogGrpcService {
    // ══════════════════════════════════════
    // 操作日志
    // ══════════════════════════════════════

    async fn create_operate_log(
        &self,
        _request: Request<CreateOperateLogRequest>,
    ) -> Result<Response<Empty>, Status> {
        // TODO: 调用 LogAppService::create_operate_log
        Ok(Response::new(Empty {}))
    }

    async fn list_operate_logs(
        &self,
        _request: Request<ListOperateLogsRequest>,
    ) -> Result<Response<ListOperateLogsResponse>, Status> {
        // TODO: 调用 LogAppService::list_operate_logs
        let resp = ListOperateLogsResponse {
            items: vec![],
            page_info: None,
        };
        Ok(Response::new(resp))
    }

    // ══════════════════════════════════════
    // 登录日志
    // ══════════════════════════════════════

    async fn create_login_log(
        &self,
        _request: Request<CreateLoginLogRequest>,
    ) -> Result<Response<Empty>, Status> {
        // TODO: 调用 LogAppService::create_login_log
        Ok(Response::new(Empty {}))
    }

    async fn list_login_logs(
        &self,
        _request: Request<ListLoginLogsRequest>,
    ) -> Result<Response<ListLoginLogsResponse>, Status> {
        // TODO: 调用 LogAppService::list_login_logs
        let resp = ListLoginLogsResponse {
            items: vec![],
            page_info: None,
        };
        Ok(Response::new(resp))
    }
}
