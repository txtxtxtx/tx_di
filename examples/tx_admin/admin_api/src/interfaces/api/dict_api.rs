//! 字典管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use tx_di_axum::aide::axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::dictionary::app_service::{DictTypeAppService, DictDataAppService};
use admin_proto::{CreateDictTypeRequest, UpdateDictTypeRequest, ListDictTypesRequest, DictTypeResponse, CreateDictDataRequest, UpdateDictDataRequest, ListDictDataRequest, DictDataResponse, Empty};
use tx_common::{ApiR, ApiRes, Page};

pub fn router() -> Router {
    Router::new()
        .api_route("/type", post(create_dict_type))
        .api_route("/type/{id}", get(get_dict_type))
        .api_route("/type/{id}", put(update_dict_type))
        .api_route("/type/{id}", delete(delete_dict_type))
        .api_route("/type/list", post(list_dict_types))
        .api_route("/data", post(create_dict_data))
        .api_route("/data/{id}", get(get_dict_data))
        .api_route("/data/{id}", put(update_dict_data))
        .api_route("/data/{id}", delete(delete_dict_data))
        .api_route("/data/list", post(list_dict_data))
        .api_route("/data/type/{dict_type}", get(get_dict_data_by_type))
        .api_route("/data/code/{dict_code}", get(get_dict_data_by_code))
}

fn map_type(d: admin_app::dictionary::dto::DictTypeResponse) -> DictTypeResponse { DictTypeResponse { id: d.id, name: d.name, dict_type: d.dict_type, status: d.status, remark: d.remark } }
fn map_data(d: admin_app::dictionary::dto::DictDataResponse) -> DictDataResponse { DictDataResponse { id: d.id, sort: d.sort, label: d.label, value: d.value, dict_type: d.dict_type, status: d.status, color_type: d.color_type, css_class: d.css_class, remark: d.remark } }

async fn create_dict_type(DiComp(dict_type): DiComp<DictTypeAppService>, Json(req): Json<CreateDictTypeRequest>) -> R<DictTypeResponse> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::dictionary::dto::CreateDictTypeCommand { name: req.name, dict_type: req.dict_type, remark: opt_filter(req.remark) };
    match dict_type.create_dict_type(cmd, None).await { Ok(r) => R(ApiR::success(map_type(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn get_dict_type(DiComp(dict_type): DiComp<DictTypeAppService>, axum::extract::Path(id): axum::extract::Path<u64>) -> R<DictTypeResponse> {
    let q = admin_app::dictionary::dto::DictTypeQueryRequest { name: None, dict_type: None, status: None, page: 1, size: 100 };
    let page = match dict_type.get_dict_type_page(q).await {
        Ok(p) => p,
        Err(e) => return R(ApiRes::from(e).into_typed()),
    };
    let Some(r) = page.list.into_iter().find(|d| d.id == id) else {
        return R(ApiRes::fail("not found".into()).into_typed());
    };
    R(ApiR::success(map_type(r)))
}

async fn update_dict_type(DiComp(dict_type): DiComp<DictTypeAppService>, axum::extract::Path(id): axum::extract::Path<u64>, Json(req): Json<UpdateDictTypeRequest>) -> R<DictTypeResponse> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::dictionary::dto::UpdateDictTypeCommand { id, name: req.name, dict_type: req.dict_type, remark: opt_filter(req.remark) };
    match dict_type.update_dict_type(cmd, None).await { Ok(r) => R(ApiR::success(map_type(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn delete_dict_type(DiComp(dict_type): DiComp<DictTypeAppService>, axum::extract::Path(id): axum::extract::Path<u64>) -> R<Empty> {
    match dict_type.delete_dict_type(id, None).await { Ok(()) => R(ApiRes::ok().into_typed()), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn list_dict_types(DiComp(dict_type): DiComp<DictTypeAppService>, Json(req): Json<ListDictTypesRequest>) -> R<Page<DictTypeResponse>> {
    let query = admin_app::dictionary::dto::DictTypeQueryRequest { name: req.name, dict_type: req.dict_type, status: req.status, page: req.page, size: req.page_size };
    match dict_type.get_dict_type_page(query).await { Ok(page) => R(ApiR::success(Page::new(page.list.into_iter().map(map_type).collect(), page.page, page.size, page.total))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn create_dict_data(DiComp(dict_data): DiComp<DictDataAppService>, Json(req): Json<CreateDictDataRequest>) -> R<DictDataResponse> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::dictionary::dto::CreateDictDataCommand { sort: req.sort, label: req.label, value: req.value, dict_type: req.dict_type, color_type: opt_filter(req.color_type), css_class: opt_filter(req.css_class), remark: opt_filter(req.remark) };
    match dict_data.create_dict_data(cmd, None).await { Ok(r) => R(ApiR::success(map_data(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn get_dict_data(DiComp(dict_data): DiComp<DictDataAppService>, axum::extract::Path(id): axum::extract::Path<u64>) -> R<DictDataResponse> {
    let q = admin_app::dictionary::dto::DictDataQueryRequest { dict_type: None, label: None, status: None, page: 1, size: 100 };
    let page = match dict_data.get_dict_data_page(q).await {
        Ok(p) => p,
        Err(e) => return R(ApiRes::from(e).into_typed()),
    };
    let Some(r) = page.list.into_iter().find(|d| d.id == id) else {
        return R(ApiRes::fail("not found".into()).into_typed());
    };
    R(ApiR::success(map_data(r)))
}

async fn update_dict_data(DiComp(dict_data): DiComp<DictDataAppService>, axum::extract::Path(id): axum::extract::Path<u64>, Json(req): Json<UpdateDictDataRequest>) -> R<DictDataResponse> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::dictionary::dto::UpdateDictDataCommand { id, sort: req.sort, label: req.label, value: req.value, dict_type: req.dict_type, color_type: opt_filter(req.color_type), css_class: opt_filter(req.css_class), remark: opt_filter(req.remark) };
    match dict_data.update_dict_data(cmd, None).await { Ok(r) => R(ApiR::success(map_data(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn delete_dict_data(DiComp(dict_data): DiComp<DictDataAppService>, axum::extract::Path(id): axum::extract::Path<u64>) -> R<Empty> {
    match dict_data.delete_dict_data(id, None).await { Ok(()) => R(ApiRes::ok().into_typed()), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn list_dict_data(DiComp(dict_data): DiComp<DictDataAppService>, Json(req): Json<ListDictDataRequest>) -> R<Page<DictDataResponse>> {
    let query = admin_app::dictionary::dto::DictDataQueryRequest { dict_type: req.dict_type, label: req.label, status: req.status, page: req.page, size: req.page_size };
    match dict_data.get_dict_data_page(query).await { Ok(page) => R(ApiR::success(Page::new(page.list.into_iter().map(map_data).collect(), page.page, page.size, page.total))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

/// GET /api/dict/data/type/{dict_type}
async fn get_dict_data_by_type(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(dict_type): axum::extract::Path<String>,
) -> R<Vec<DictDataResponse>> {
    match dict_data.get_by_dict_type(&dict_type).await {
        Ok(list) => R(ApiR::success(list.into_iter().map(map_data).collect())),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// GET /api/dict/data/code/{dict_code}
async fn get_dict_data_by_code(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(dict_code): axum::extract::Path<String>,
) -> R<Vec<DictDataResponse>> {
    match dict_data.get_by_dict_type(&dict_code).await {
        Ok(list) => R(ApiR::success(list.into_iter().map(map_data).collect())),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}
