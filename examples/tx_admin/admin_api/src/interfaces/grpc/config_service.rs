//! 配置管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::config::config_service_server::ConfigService;
use admin_proto::admin::config::{
    CreateConfigRequest, ConfigResponse, UpdateConfigRequest, DeleteConfigRequest,
    GetConfigRequest, ListConfigsRequest, ListConfigsResponse,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;

#[derive(Debug, Default)]
pub struct ConfigGrpcService;

#[tonic::async_trait]
impl ConfigService for ConfigGrpcService {
    async fn create_config(&self, request: Request<CreateConfigRequest>) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        services::get().config.create_config(req, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_config(&self, request: Request<UpdateConfigRequest>) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        services::get().config.update_config(req, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn delete_config(&self, request: Request<DeleteConfigRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().config.delete_config(req.config_id, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_config(&self, request: Request<GetConfigRequest>) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        services::get().config.get_config(req.config_id).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_configs(&self, request: Request<ListConfigsRequest>) -> Result<Response<ListConfigsResponse>, Status> {
        let req = request.into_inner();
        services::get().config.get_config_page(req).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list;
                Response::new(ListConfigsResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }
}
