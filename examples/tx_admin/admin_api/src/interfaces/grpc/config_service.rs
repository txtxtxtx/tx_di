//! 配置管理 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::config::config_service_server::ConfigService;
use admin_proto::admin::config::{
    ConfigResponse, CreateConfigRequest, DeleteConfigRequest, GetByKeysRequest, GetByKeysResponse,
    GetConfigRequest, ListConfigsRequest, ListConfigsResponse, UpdateConfigRequest,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use tx_di_core::App;

use super::auth_interceptor::{self, get_login_id};
use super::err;

#[derive(Clone)]
pub struct ConfigGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl ConfigService for ConfigGrpcService {
    async fn create_config(
        &self,
        request: Request<CreateConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "config:create").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::config::app_service::ConfigAppService> = self.app.inject();
        let r = svc.create_config(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn update_config(
        &self,
        request: Request<UpdateConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "config:update").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::config::app_service::ConfigAppService> = self.app.inject();
        let r = svc.update_config(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn delete_config(
        &self,
        request: Request<DeleteConfigRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "config:delete").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::config::app_service::ConfigAppService> = self.app.inject();
        svc.delete_config(req.config_id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_config(
        &self,
        request: Request<GetConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "config:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::config::app_service::ConfigAppService> = self.app.inject();
        let r = svc.get_config(req.config_id).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn list_configs(
        &self,
        request: Request<ListConfigsRequest>,
    ) -> Result<Response<ListConfigsResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "config:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::config::app_service::ConfigAppService> = self.app.inject();
        let p = svc.get_config_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListConfigsResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }

    async fn get_by_keys(
        &self,
        request: Request<GetByKeysRequest>,
    ) -> Result<Response<GetByKeysResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "config:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::config::app_service::ConfigAppService> = self.app.inject();
        let configs = svc
            .get_by_keys(req.keys)
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(GetByKeysResponse { configs }))
    }
}
