//! 日志管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::log::log_service_server::LogService;
use admin_proto::admin::log::{
    CreateOperateLogRequest, ListOperateLogsRequest, ListOperateLogsResponse,
    CreateLoginLogRequest, ListLoginLogsRequest, ListLoginLogsResponse,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use crate::services;

#[derive(Debug, Default)]
pub struct LogGrpcService;

fn map_oper_log(l: admin_app::log::dto::OperateLogResponse) -> admin_proto::OperateLogResponse {
    admin_proto::OperateLogResponse {
        id: l.id, trace_id: l.trace_id, user_id: l.user_id, user_type: l.user_type,
        log_type: l.log_type, sub_type: l.sub_type, biz_id: l.biz_id,
        action: l.action, success: l.success, extra: l.extra,
        request_method: l.request_method, request_url: l.request_url, user_ip: l.user_ip,
    }
}

fn map_login_log(l: admin_app::log::dto::LoginLogResponse) -> admin_proto::LoginLogResponse {
    admin_proto::LoginLogResponse {
        id: l.id, user_id: l.user_id, user_type: l.user_type,
        username: l.username, login_ip: l.login_ip,
        login_type: l.login_type, result: l.result, msg: l.msg,
    }
}

#[tonic::async_trait]
impl LogService for LogGrpcService {
    async fn create_operate_log(&self, request: Request<CreateOperateLogRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::log::dto::CreateOperateLogCommand {
            trace_id: req.trace_id, user_id: req.user_id, user_type: req.user_type,
            log_type: req.log_type, sub_type: req.sub_type, biz_id: req.biz_id,
            action: req.action, success: req.success, extra: req.extra,
        };
        services::get().oper_log.create_log(cmd).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_operate_logs(&self, request: Request<ListOperateLogsRequest>) -> Result<Response<ListOperateLogsResponse>, Status> {
        let req = request.into_inner();
        let q = admin_app::log::dto::OperateLogQueryRequest {
            user_id: req.user_id, log_type: req.log_type, sub_type: req.sub_type,
            success: req.success, begin_time: req.begin_time, end_time: req.end_time,
            page: req.page, size: req.page_size,
        };
        services::get().oper_log.get_log_page(q).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list.into_iter().map(map_oper_log).collect();
                Response::new(ListOperateLogsResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn create_login_log(&self, request: Request<CreateLoginLogRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::log::dto::CreateLoginLogCommand {
            user_id: req.user_id, user_type: req.user_type,
            username: req.username, login_ip: req.login_ip,
            login_type: req.login_type, result: req.result,
        };
        services::get().login_log.create_log(cmd).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_login_logs(&self, request: Request<ListLoginLogsRequest>) -> Result<Response<ListLoginLogsResponse>, Status> {
        let req = request.into_inner();
        let q = admin_app::log::dto::LoginLogQueryRequest {
            user_id: req.user_id, username: req.username, login_ip: req.login_ip,
            login_type: req.login_type, result: req.result,
            begin_time: req.begin_time, end_time: req.end_time,
            page: req.page, size: req.page_size,
        };
        services::get().login_log.get_log_page(q).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list.into_iter().map(map_login_log).collect();
                Response::new(ListLoginLogsResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }
}
