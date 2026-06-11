//! 字典管理 gRPC 服务实现
//!
//! 包含字典类型和字典数据两部分。

use tonic::{Request, Response, Status};

use admin_proto::admin::dict::dict_service_server::DictService;
use admin_proto::admin::dict::{
    CreateDictTypeRequest, DictTypeResponse, UpdateDictTypeRequest, DeleteDictTypeRequest,
    GetDictTypeRequest, ListDictTypesRequest, ListDictTypesResponse,
    CreateDictDataRequest, DictDataResponse, UpdateDictDataRequest, DeleteDictDataRequest,
    GetDictDataRequest, ListDictDataRequest, ListDictDataResponse,
};
use admin_proto::Empty;

/// 字典 gRPC 服务
#[derive(Debug, Default)]
pub struct DictGrpcService;

#[tonic::async_trait]
impl DictService for DictGrpcService {
    // ══════════════════════════════════════
    // 字典类型
    // ══════════════════════════════════════

    async fn create_dict_type(
        &self,
        request: Request<CreateDictTypeRequest>,
    ) -> Result<Response<DictTypeResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DictAppService::create_type
        let resp = DictTypeResponse {
            id: 1,
            name: req.name.clone(),
            dict_type: req.dict_type.clone(),
            status: 1,
            remark: req.remark.clone(),
        };
        Ok(Response::new(resp))
    }

    async fn update_dict_type(
        &self,
        request: Request<UpdateDictTypeRequest>,
    ) -> Result<Response<DictTypeResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DictAppService::update_type
        let resp = DictTypeResponse {
            id: req.id,
            name: req.name.clone(),
            dict_type: req.dict_type.clone(),
            status: 1,
            remark: req.remark.clone(),
        };
        Ok(Response::new(resp))
    }

    async fn delete_dict_type(
        &self,
        request: Request<DeleteDictTypeRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DictAppService::delete_type
        let _ = req.id;
        Ok(Response::new(Empty {}))
    }

    async fn get_dict_type(
        &self,
        request: Request<GetDictTypeRequest>,
    ) -> Result<Response<DictTypeResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DictAppService::get_type_by_id
        let resp = DictTypeResponse {
            id: req.id,
            name: "placeholder".into(),
            dict_type: "placeholder".into(),
            status: 1,
            remark: None,
        };
        Ok(Response::new(resp))
    }

    async fn list_dict_types(
        &self,
        _request: Request<ListDictTypesRequest>,
    ) -> Result<Response<ListDictTypesResponse>, Status> {
        // TODO: 调用 DictAppService::list_types
        let resp = ListDictTypesResponse {
            items: vec![],
            page_info: None,
        };
        Ok(Response::new(resp))
    }

    // ══════════════════════════════════════
    // 字典数据
    // ══════════════════════════════════════

    async fn create_dict_data(
        &self,
        request: Request<CreateDictDataRequest>,
    ) -> Result<Response<DictDataResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DictAppService::create_data
        let resp = DictDataResponse {
            id: 1,
            sort: req.sort,
            label: req.label.clone(),
            value: req.value.clone(),
            dict_type: req.dict_type.clone(),
            status: 1,
            color_type: req.color_type.clone(),
            css_class: req.css_class.clone(),
            remark: req.remark.clone(),
        };
        Ok(Response::new(resp))
    }

    async fn update_dict_data(
        &self,
        request: Request<UpdateDictDataRequest>,
    ) -> Result<Response<DictDataResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DictAppService::update_data
        let resp = DictDataResponse {
            id: req.id,
            sort: req.sort,
            label: req.label.clone(),
            value: req.value.clone(),
            dict_type: req.dict_type.clone(),
            status: 1,
            color_type: req.color_type.clone(),
            css_class: req.css_class.clone(),
            remark: req.remark.clone(),
        };
        Ok(Response::new(resp))
    }

    async fn delete_dict_data(
        &self,
        request: Request<DeleteDictDataRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DictAppService::delete_data
        let _ = req.id;
        Ok(Response::new(Empty {}))
    }

    async fn get_dict_data(
        &self,
        request: Request<GetDictDataRequest>,
    ) -> Result<Response<DictDataResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DictAppService::get_data_by_id
        let resp = DictDataResponse {
            id: req.id,
            sort: 0,
            label: "placeholder".into(),
            value: "placeholder".into(),
            dict_type: String::new(),
            status: 1,
            color_type: None,
            css_class: None,
            remark: None,
        };
        Ok(Response::new(resp))
    }

    async fn list_dict_data(
        &self,
        _request: Request<ListDictDataRequest>,
    ) -> Result<Response<ListDictDataResponse>, Status> {
        // TODO: 调用 DictAppService::list_data
        let resp = ListDictDataResponse {
            items: vec![],
            page_info: None,
        };
        Ok(Response::new(resp))
    }
}
