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
use crate::services;

#[derive(Debug, Default)]
pub struct DictGrpcService;

fn map_type(d: admin_app::dictionary::dto::DictTypeResponse) -> DictTypeResponse {
    DictTypeResponse { id: d.id, name: d.name, dict_type: d.dict_type, status: d.status, remark: d.remark }
}

fn map_data(d: admin_app::dictionary::dto::DictDataResponse) -> DictDataResponse {
    DictDataResponse {
        id: d.id, sort: d.sort, label: d.label, value: d.value, dict_type: d.dict_type,
        status: d.status, color_type: d.color_type, css_class: d.css_class, remark: d.remark,
    }
}

#[tonic::async_trait]
impl DictService for DictGrpcService {
    // ════════════════════ 字典类型 ════════════════════

    async fn create_dict_type(&self, r: Request<CreateDictTypeRequest>) -> Result<Response<DictTypeResponse>, Status> {
        let req = r.into_inner();
        let cmd = admin_app::dictionary::dto::CreateDictTypeCommand { name: req.name, dict_type: req.dict_type, remark: req.remark };
        services::get().dict_type.create_dict_type(cmd, None).await
            .map(|r| Response::new(map_type(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_dict_type(&self, r: Request<UpdateDictTypeRequest>) -> Result<Response<DictTypeResponse>, Status> {
        let req = r.into_inner();
        let cmd = admin_app::dictionary::dto::UpdateDictTypeCommand { id: req.id, name: req.name, dict_type: req.dict_type, remark: req.remark };
        services::get().dict_type.update_dict_type(cmd, None).await
            .map(|r| Response::new(map_type(r)))
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
        let q = admin_app::dictionary::dto::DictTypeQueryRequest { name: None, dict_type: None, status: None, page: 1, size: 100 };
        services::get().dict_type.get_dict_type_page(q).await
            .map(|p| {
                let found = p.list.into_iter().find(|d| d.id == req.id).expect("dict type not found");
                Response::new(map_type(found))
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_dict_types(&self, r: Request<ListDictTypesRequest>) -> Result<Response<ListDictTypesResponse>, Status> {
        let req = r.into_inner();
        let q = admin_app::dictionary::dto::DictTypeQueryRequest {
            name: req.name, dict_type: req.dict_type, status: req.status,
            page: req.page, size: req.page_size,
        };
        services::get().dict_type.get_dict_type_page(q).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size; let total_pages = p.total_pages();
                let items = p.list.into_iter().map(map_type).collect();
                Response::new(ListDictTypesResponse { items, page_info: Some(PageResponse { total, page, size, total_pages }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    // ════════════════════ 字典数据 ════════════════════

    async fn create_dict_data(&self, r: Request<CreateDictDataRequest>) -> Result<Response<DictDataResponse>, Status> {
        let req = r.into_inner();
        let cmd = admin_app::dictionary::dto::CreateDictDataCommand {
            sort: req.sort, label: req.label, value: req.value, dict_type: req.dict_type,
            color_type: req.color_type, css_class: req.css_class, remark: req.remark,
        };
        services::get().dict_data.create_dict_data(cmd, None).await
            .map(|r| Response::new(map_data(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_dict_data(&self, r: Request<UpdateDictDataRequest>) -> Result<Response<DictDataResponse>, Status> {
        let req = r.into_inner();
        let cmd = admin_app::dictionary::dto::UpdateDictDataCommand {
            id: req.id, sort: req.sort, label: req.label, value: req.value, dict_type: req.dict_type,
            color_type: req.color_type, css_class: req.css_class, remark: req.remark,
        };
        services::get().dict_data.update_dict_data(cmd, None).await
            .map(|r| Response::new(map_data(r)))
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
        let q = admin_app::dictionary::dto::DictDataQueryRequest { dict_type: None, label: None, status: None, page: 1, size: 100 };
        services::get().dict_data.get_dict_data_page(q).await
            .map(|p| {
                let found = p.list.into_iter().find(|d| d.id == req.id).expect("dict data not found");
                Response::new(map_data(found))
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_dict_data(&self, r: Request<ListDictDataRequest>) -> Result<Response<ListDictDataResponse>, Status> {
        let req = r.into_inner();
        let q = admin_app::dictionary::dto::DictDataQueryRequest {
            dict_type: req.dict_type, label: req.label, status: req.status,
            page: req.page, size: req.page_size,
        };
        services::get().dict_data.get_dict_data_page(q).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size; let total_pages = p.total_pages();
                let items = p.list.into_iter().map(map_data).collect();
                Response::new(ListDictDataResponse { items, page_info: Some(PageResponse { total, page, size, total_pages }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }
}
