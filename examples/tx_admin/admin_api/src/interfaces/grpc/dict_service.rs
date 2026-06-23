//! 字典管理 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::dict::dict_service_server::DictService;
use admin_proto::admin::dict::{
    CreateDictDataRequest, CreateDictTypeRequest, DeleteDictDataRequest, DeleteDictTypeRequest,
    DictDataResponse, DictTypeResponse, GetByDictTypesRequest, GetByDictTypesResponse,
    GetDictDataRequest, GetDictTypeRequest, ListDictDataRequest, ListDictDataResponse,
    ListDictTypesRequest, ListDictTypesResponse, UpdateDictDataRequest, UpdateDictTypeRequest,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use tx_di_core::App;

use super::auth_interceptor::{self, get_login_id};
use super::err;

#[derive(Clone)]
pub struct DictGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl DictService for DictGrpcService {
    // ════════════════════ 字典类型 ════════════════════

    async fn create_dict_type(
        &self,
        r: Request<CreateDictTypeRequest>,
    ) -> Result<Response<DictTypeResponse>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:create").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictTypeAppService> = self.app.inject();
        let r = svc.create_dict_type(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn update_dict_type(
        &self,
        r: Request<UpdateDictTypeRequest>,
    ) -> Result<Response<DictTypeResponse>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:update").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictTypeAppService> = self.app.inject();
        let r = svc.update_dict_type(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn delete_dict_type(
        &self,
        r: Request<DeleteDictTypeRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:delete").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictTypeAppService> = self.app.inject();
        svc.delete_dict_type(req.id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_dict_type(
        &self,
        r: Request<GetDictTypeRequest>,
    ) -> Result<Response<DictTypeResponse>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:view").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictTypeAppService> = self.app.inject();
        let q = ListDictTypesRequest {
            name: None,
            dict_type: None,
            status: None,
            page: 1,
            page_size: 100,
        };
        let p = svc.get_dict_type_page(q).await.map_err(err::to_status)?;
        let found = p
            .list
            .into_iter()
            .find(|d| d.id == req.id)
            .ok_or_else(|| Status::not_found("dict type not found"))?;
        Ok(Response::new(found))
    }

    async fn list_dict_types(
        &self,
        r: Request<ListDictTypesRequest>,
    ) -> Result<Response<ListDictTypesResponse>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:view").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictTypeAppService> = self.app.inject();
        let p = svc.get_dict_type_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListDictTypesResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }

    // ════════════════════ 字典数据 ════════════════════

    async fn create_dict_data(
        &self,
        r: Request<CreateDictDataRequest>,
    ) -> Result<Response<DictDataResponse>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:create").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictDataAppService> = self.app.inject();
        let r = svc.create_dict_data(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn update_dict_data(
        &self,
        r: Request<UpdateDictDataRequest>,
    ) -> Result<Response<DictDataResponse>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:update").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictDataAppService> = self.app.inject();
        let r = svc.update_dict_data(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn delete_dict_data(
        &self,
        r: Request<DeleteDictDataRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:delete").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictDataAppService> = self.app.inject();
        svc.delete_dict_data(req.id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_dict_data(
        &self,
        r: Request<GetDictDataRequest>,
    ) -> Result<Response<DictDataResponse>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:view").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictDataAppService> = self.app.inject();
        let q = ListDictDataRequest {
            dict_type: None,
            label: None,
            status: None,
            page: 1,
            page_size: 100,
        };
        let p = svc.get_dict_data_page(q).await.map_err(err::to_status)?;
        let found = p
            .list
            .into_iter()
            .find(|d| d.id == req.id)
            .ok_or_else(|| Status::not_found("dict data not found"))?;
        Ok(Response::new(found))
    }

    async fn list_dict_data(
        &self,
        r: Request<ListDictDataRequest>,
    ) -> Result<Response<ListDictDataResponse>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:view").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictDataAppService> = self.app.inject();
        let p = svc.get_dict_data_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListDictDataResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }

    async fn get_by_dict_types(
        &self,
        r: Request<GetByDictTypesRequest>,
    ) -> Result<Response<GetByDictTypesResponse>, Status> {
        let login_id = get_login_id(&r)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dict:view").await?;

        let req = r.into_inner();
        let svc: Arc<admin_app::dictionary::app_service::DictDataAppService> = self.app.inject();
        let map = svc
            .get_by_dict_types(req.dict_types)
            .await
            .map_err(err::to_status)?;

        // 转换: HashMap<String, Vec<DictDataResponse>> → HashMap<String, ListDictDataResponse>
        let dict_data = map
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    ListDictDataResponse {
                        items: v,
                        page_info: None,
                    },
                )
            })
            .collect();

        Ok(Response::new(GetByDictTypesResponse { dict_data }))
    }
}
