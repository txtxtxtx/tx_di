//! 配置管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::config::config_service_server::ConfigService;
use admin_proto::admin::config::{
    CreateConfigRequest, ConfigResponse, UpdateConfigRequest, DeleteConfigRequest,
    GetConfigRequest, ListConfigsRequest, ListConfigsResponse,
};
use admin_proto::Empty;

/// 配置 gRPC 服务
#[derive(Debug, Default)]
pub struct ConfigGrpcService;

#[tonic::async_trait]
impl ConfigService for ConfigGrpcService {
    async fn create_config(
        &self,
        request: Request<CreateConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 ConfigAppService::create
        let resp = ConfigResponse {
            id: 1,
            category: req.category.clone(),
            config_type: req.config_type,
            name: req.name.clone(),
            config_key: req.config_key.clone(),
            value: req.value.clone(),
            visible: 0,
            remark: req.remark.clone(),
        };
        Ok(Response::new(resp))
    }

    async fn update_config(
        &self,
        request: Request<UpdateConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 ConfigAppService::update
        let resp = ConfigResponse {
            id: req.config_id,
            category: req.category.clone(),
            config_type: req.config_type,
            name: req.name.clone(),
            config_key: req.config_key.clone(),
            value: req.value.clone(),
            visible: req.visible,
            remark: req.remark.clone(),
        };
        Ok(Response::new(resp))
    }

    async fn delete_config(
        &self,
        request: Request<DeleteConfigRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        // TODO: 调用 ConfigAppService::delete
        let _ = req.config_id;
        Ok(Response::new(Empty {}))
    }

    async fn get_config(
        &self,
        request: Request<GetConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 ConfigAppService::get_by_id
        let resp = ConfigResponse {
            id: req.config_id,
            category: String::new(),
            config_type: 0,
            name: "placeholder".into(),
            config_key: String::new(),
            value: String::new(),
            visible: 0,
            remark: None,
        };
        Ok(Response::new(resp))
    }

    async fn list_configs(
        &self,
        _request: Request<ListConfigsRequest>,
    ) -> Result<Response<ListConfigsResponse>, Status> {
        // TODO: 调用 ConfigAppService::list
        let resp = ListConfigsResponse {
            items: vec![],
            page_info: None,
        };
        Ok(Response::new(resp))
    }
}
