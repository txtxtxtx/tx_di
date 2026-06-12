//! 配置管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::config::config_service_server::ConfigService;
use admin_proto::admin::config::{
    CreateConfigRequest, ConfigResponse, UpdateConfigRequest, DeleteConfigRequest,
    GetConfigRequest, ListConfigsRequest, ListConfigsResponse,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use crate::services;

#[derive(Debug, Default)]
pub struct ConfigGrpcService;

fn map_config(c: admin_app::config::dto::ConfigResponse) -> ConfigResponse {
    ConfigResponse {
        id: c.id, category: c.category, config_type: c.config_type,
        name: c.name, config_key: c.config_key, value: c.value,
        visible: c.visible, remark: c.remark,
    }
}

#[tonic::async_trait]
impl ConfigService for ConfigGrpcService {
    async fn create_config(&self, request: Request<CreateConfigRequest>) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::config::dto::CreateConfigCommand {
            category: req.category, config_type: req.config_type,
            name: req.name, config_key: req.config_key,
            value: req.value, remark: req.remark,
        };
        services::get().config.create_config(cmd, None).await
            .map(|r| Response::new(map_config(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_config(&self, request: Request<UpdateConfigRequest>) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::config::dto::UpdateConfigCommand {
            config_id: req.config_id, category: req.category, config_type: req.config_type,
            name: req.name, config_key: req.config_key, value: req.value,
            visible: req.visible, remark: req.remark,
        };
        services::get().config.update_config(cmd, None).await
            .map(|r| Response::new(map_config(r)))
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
            .map(|r| Response::new(map_config(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_configs(&self, request: Request<ListConfigsRequest>) -> Result<Response<ListConfigsResponse>, Status> {
        let req = request.into_inner();
        let query = admin_app::config::dto::ConfigQueryRequest {
            name: req.name, category: req.category,
            config_key: req.config_key, config_type: req.config_type,
            page: req.page, size: req.page_size,
        };
        services::get().config.get_config_page(query).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list.into_iter().map(map_config).collect();
                Response::new(ListConfigsResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }
}
