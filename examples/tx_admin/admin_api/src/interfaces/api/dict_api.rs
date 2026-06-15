//! 字典管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::dictionary::app_service::{DictTypeAppService, DictDataAppService};
use admin_proto::{CreateDictTypeRequest, UpdateDictTypeRequest, ListDictTypesRequest, DictTypeResponse, CreateDictDataRequest, UpdateDictDataRequest, ListDictDataRequest, DictDataResponse, Empty};
use tx_common::{ApiR, ApiRes, Page};
use tx_di_sa_token::sa_check_permission;
use crate::error::ApiErr;

pub fn router() -> Router {
    Router::new()
        .route("/type", post(create_dict_type))
        .route("/type/{id}", get(get_dict_type))
        .route("/type/{id}", put(update_dict_type))
        .route("/type/{id}", delete(delete_dict_type))
        .route("/type/list", post(list_dict_types))
        .route("/data", post(create_dict_data))
        .route("/data/{id}", get(get_dict_data))
        .route("/data/{id}", put(update_dict_data))
        .route("/data/{id}", delete(delete_dict_data))
        .route("/data/list", post(list_dict_data))
        .route("/data/type/{dict_type}", get(get_dict_data_by_type))
        .route("/data/code/{dict_code}", get(get_dict_data_by_code))
}

fn map_type(d: admin_app::dictionary::dto::DictTypeResponse) -> DictTypeResponse { DictTypeResponse { id: d.id, name: d.name, dict_type: d.dict_type, status: d.status, remark: d.remark } }
fn map_data(d: admin_app::dictionary::dto::DictDataResponse) -> DictDataResponse { DictDataResponse { id: d.id, sort: d.sort, label: d.label, value: d.value, dict_type: d.dict_type, status: d.status, color_type: d.color_type, css_class: d.css_class, remark: d.remark } }

#[sa_check_permission("dict:create")]
async fn create_dict_type(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    Json(req): Json<CreateDictTypeRequest>,
) -> Result<R<DictTypeResponse>, ApiErr> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::dictionary::dto::CreateDictTypeCommand { name: req.name, dict_type: req.dict_type, remark: opt_filter(req.remark) };
    let r = dict_type.create_dict_type(cmd, None).await?;
    Ok(R(ApiR::success(map_type(r))))
}

#[sa_check_permission("dict:view")]
async fn get_dict_type(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<R<DictTypeResponse>, ApiErr> {
    let q = admin_app::dictionary::dto::DictTypeQueryRequest { name: None, dict_type: None, status: None, page: 1, size: 100 };
    let page = dict_type.get_dict_type_page(q).await?;
    let r = page.list.into_iter().find(|d| d.id == id)
        .ok_or_else(|| anyhow::anyhow!("not found"))?;
    Ok(R(ApiR::success(map_type(r))))
}

#[sa_check_permission("dict:update")]
async fn update_dict_type(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(req): Json<UpdateDictTypeRequest>,
) -> Result<R<DictTypeResponse>, ApiErr> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::dictionary::dto::UpdateDictTypeCommand { id, name: req.name, dict_type: req.dict_type, remark: opt_filter(req.remark) };
    let r = dict_type.update_dict_type(cmd, None).await?;
    Ok(R(ApiR::success(map_type(r))))
}

#[sa_check_permission("dict:delete")]
async fn delete_dict_type(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    dict_type.delete_dict_type(id, None).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

#[sa_check_permission("dict:view")]
async fn list_dict_types(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    Json(req): Json<ListDictTypesRequest>,
) -> Result<R<Page<DictTypeResponse>>, ApiErr> {
    let query = admin_app::dictionary::dto::DictTypeQueryRequest { name: req.name, dict_type: req.dict_type, status: req.status, page: req.page, size: req.page_size };
    let page = dict_type.get_dict_type_page(query).await?;
    Ok(R(ApiR::success(Page::new(page.list.into_iter().map(map_type).collect(), page.page, page.size, page.total))))
}

#[sa_check_permission("dict:create")]
async fn create_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    Json(req): Json<CreateDictDataRequest>,
) -> Result<R<DictDataResponse>, ApiErr> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::dictionary::dto::CreateDictDataCommand { sort: req.sort, label: req.label, value: req.value, dict_type: req.dict_type, color_type: opt_filter(req.color_type), css_class: opt_filter(req.css_class), remark: opt_filter(req.remark) };
    let r = dict_data.create_dict_data(cmd, None).await?;
    Ok(R(ApiR::success(map_data(r))))
}

#[sa_check_permission("dict:view")]
async fn get_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<R<DictDataResponse>, ApiErr> {
    let q = admin_app::dictionary::dto::DictDataQueryRequest { dict_type: None, label: None, status: None, page: 1, size: 100 };
    let page = dict_data.get_dict_data_page(q).await?;
    let r = page.list.into_iter().find(|d| d.id == id)
        .ok_or_else(|| anyhow::anyhow!("not found"))?;
    Ok(R(ApiR::success(map_data(r))))
}

#[sa_check_permission("dict:update")]
async fn update_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(req): Json<UpdateDictDataRequest>,
) -> Result<R<DictDataResponse>, ApiErr> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::dictionary::dto::UpdateDictDataCommand { id, sort: req.sort, label: req.label, value: req.value, dict_type: req.dict_type, color_type: opt_filter(req.color_type), css_class: opt_filter(req.css_class), remark: opt_filter(req.remark) };
    let r = dict_data.update_dict_data(cmd, None).await?;
    Ok(R(ApiR::success(map_data(r))))
}

#[sa_check_permission("dict:delete")]
async fn delete_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    dict_data.delete_dict_data(id, None).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

#[sa_check_permission("dict:view")]
async fn list_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    Json(req): Json<ListDictDataRequest>,
) -> Result<R<Page<DictDataResponse>>, ApiErr> {
    let query = admin_app::dictionary::dto::DictDataQueryRequest { dict_type: req.dict_type, label: req.label, status: req.status, page: req.page, size: req.page_size };
    let page = dict_data.get_dict_data_page(query).await?;
    Ok(R(ApiR::success(Page::new(page.list.into_iter().map(map_data).collect(), page.page, page.size, page.total))))
}

/// GET /api/dict/data/type/{dict_type}
#[sa_check_permission("dict:view")]
async fn get_dict_data_by_type(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(dict_type): axum::extract::Path<String>,
) -> Result<R<Vec<DictDataResponse>>, ApiErr> {
    let list = dict_data.get_by_dict_type(&dict_type).await?;
    Ok(R(ApiR::success(list.into_iter().map(map_data).collect())))
}

/// GET /api/dict/data/code/{dict_code}
#[sa_check_permission("dict:view")]
async fn get_dict_data_by_code(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(dict_code): axum::extract::Path<String>,
) -> Result<R<Vec<DictDataResponse>>, ApiErr> {
    let list = dict_data.get_by_dict_type(&dict_code).await?;
    Ok(R(ApiR::success(list.into_iter().map(map_data).collect())))
}
