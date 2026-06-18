//! 字典管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::dict::dict_service_server::DictService;
use admin_proto::admin::dict::{
    CreateDictTypeRequest, DictTypeResponse, UpdateDictTypeRequest, DeleteDictTypeRequest,
    GetDictTypeRequest, ListDictTypesRequest, ListDictTypesResponse,
    CreateDictDataRequest, DictDataResponse, UpdateDictDataRequest, DeleteDictDataRequest,
    GetDictDataRequest, ListDictDataRequest, ListDictDataResponse,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;

#[derive(Debug, Default)]
pub struct DictGrpcService;

#[tonic::async_trait]
impl DictService for DictGrpcService {
    // ════════════════════ 字典类型 ════════════════════

    async fn create_dict_type(&self, r: Request<CreateDictTypeRequest>) -> Result<Response<DictTypeResponse>, Status> {
        let req = r.into_inner();
        services::get().dict_type.create_dict_type(req, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_dict_type(&self, r: Request<UpdateDictTypeRequest>) -> Result<Response<DictTypeResponse>, Status> {
        let req = r.into_inner();
        services::get().dict_type.update_dict_type(req, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn delete_dict_type(&self, r: Request<DeleteDictTypeRequest>) -> Result<Response<Empty>, Status> {
        let req = r.into_inner();
        services::get().dict_type.delete_dict_type(req.id, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_dict_type(&self, r: Request<GetDictTypeRequest>) -> Result<Response<DictTypeResponse>, Status> {
        let req = r.into_inner();
        let q = ListDictTypesRequest { name: None, dict_type: None, status: None, page: 1, page_size: 100 };
        services::get().dict_type.get_dict_type_page(q).await
            .map(|p| {
                let found = p.list.into_iter().find(|d| d.id == req.id).expect("dict type not found");
                Response::new(found)
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_dict_types(&self, r: Request<ListDictTypesRequest>) -> Result<Response<ListDictTypesResponse>, Status> {
        let req = r.into_inner();
        services::get().dict_type.get_dict_type_page(req).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list;
                Response::new(ListDictTypesResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    // ════════════════════ 字典数据 ════════════════════

    async fn create_dict_data(&self, r: Request<CreateDictDataRequest>) -> Result<Response<DictDataResponse>, Status> {
        let req = r.into_inner();
        services::get().dict_data.create_dict_data(req, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_dict_data(&self, r: Request<UpdateDictDataRequest>) -> Result<Response<DictDataResponse>, Status> {
        let req = r.into_inner();
        services::get().dict_data.update_dict_data(req, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn delete_dict_data(&self, r: Request<DeleteDictDataRequest>) -> Result<Response<Empty>, Status> {
        let req = r.into_inner();
        services::get().dict_data.delete_dict_data(req.id, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_dict_data(&self, r: Request<GetDictDataRequest>) -> Result<Response<DictDataResponse>, Status> {
        let req = r.into_inner();
        let q = ListDictDataRequest { dict_type: None, label: None, status: None, page: 1, page_size: 100 };
        services::get().dict_data.get_dict_data_page(q).await
            .map(|p| {
                let found = p.list.into_iter().find(|d| d.id == req.id).expect("dict data not found");
                Response::new(found)
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_dict_data(&self, r: Request<ListDictDataRequest>) -> Result<Response<ListDictDataResponse>, Status> {
        let req = r.into_inner();
        services::get().dict_data.get_dict_data_page(req).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list;
                Response::new(ListDictDataResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }
}
