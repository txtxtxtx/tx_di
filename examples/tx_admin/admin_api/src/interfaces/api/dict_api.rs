//! 字典管理 HTTP API

use axum::Json;
use tx_di_axum::Router;
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::dictionary::app_service::{DictTypeAppService, DictDataAppService};
use admin_proto::{CreateDictTypeRequest, UpdateDictTypeRequest, ListDictTypesRequest, DictTypeResponse, CreateDictDataRequest, UpdateDictDataRequest, ListDictDataRequest, DictDataResponse, Empty};
use tx_common::{ApiR, ApiRes, Page};
use crate::auth::ensure_permission;
use crate::error::ApiErr;
use tx_di_sa_token::StpUtil;

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

async fn create_dict_type(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    Json(req): Json<CreateDictTypeRequest>,
) -> Result<ApiR<DictTypeResponse>, ApiErr> {
    ensure_permission("dict:create").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = dict_type.create_dict_type(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn get_dict_type(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<ApiR<DictTypeResponse>, ApiErr> {
    ensure_permission("dict:view").await?;
    let q = ListDictTypesRequest { name: None, dict_type: None, status: None, page: 1, page_size: 100 };
    let page = dict_type.get_dict_type_page(q).await?;
    let r = page.list.into_iter().find(|d| d.id == id)
        .ok_or_else(|| anyhow::anyhow!("not found"))?;
    Ok(ApiR::success(r))
}

async fn update_dict_type(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(req): Json<UpdateDictTypeRequest>,
) -> Result<ApiR<DictTypeResponse>, ApiErr> {
    ensure_permission("dict:update").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let mut req = req;
    req.id = id;
    let r = dict_type.update_dict_type(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn delete_dict_type(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("dict:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    dict_type.delete_dict_type(id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

async fn list_dict_types(
    DiComp(dict_type): DiComp<DictTypeAppService>,
    Json(req): Json<ListDictTypesRequest>,
) -> Result<ApiR<Page<DictTypeResponse>>, ApiErr> {
    ensure_permission("dict:view").await?;
    let page = dict_type.get_dict_type_page(req).await?;
    Ok(ApiR::success(page))
}

async fn create_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    Json(req): Json<CreateDictDataRequest>,
) -> Result<ApiR<DictDataResponse>, ApiErr> {
    ensure_permission("dict:create").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = dict_data.create_dict_data(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn get_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<ApiR<DictDataResponse>, ApiErr> {
    ensure_permission("dict:view").await?;
    let q = ListDictDataRequest { dict_type: None, label: None, status: None, page: 1, page_size: 100 };
    let page = dict_data.get_dict_data_page(q).await?;
    let r = page.list.into_iter().find(|d| d.id == id)
        .ok_or_else(|| anyhow::anyhow!("not found"))?;
    Ok(ApiR::success(r))
}

async fn update_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(req): Json<UpdateDictDataRequest>,
) -> Result<ApiR<DictDataResponse>, ApiErr> {
    ensure_permission("dict:update").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let mut req = req;
    req.id = id;
    let r = dict_data.update_dict_data(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn delete_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("dict:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    dict_data.delete_dict_data(id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

async fn list_dict_data(
    DiComp(dict_data): DiComp<DictDataAppService>,
    Json(req): Json<ListDictDataRequest>,
) -> Result<ApiR<Page<DictDataResponse>>, ApiErr> {
    ensure_permission("dict:view").await?;
    let page = dict_data.get_dict_data_page(req).await?;
    Ok(ApiR::success(page))
}

/// GET /api/dict/data/type/{dict_type}
async fn get_dict_data_by_type(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(dict_type): axum::extract::Path<String>,
) -> Result<ApiR<Vec<DictDataResponse>>, ApiErr> {
    ensure_permission("dict:view").await?;
    let list = dict_data.get_by_dict_type(&dict_type).await?;
    Ok(ApiR::success(list))
}

/// GET /api/dict/data/code/{dict_code}
async fn get_dict_data_by_code(
    DiComp(dict_data): DiComp<DictDataAppService>,
    axum::extract::Path(dict_code): axum::extract::Path<String>,
) -> Result<ApiR<Vec<DictDataResponse>>, ApiErr> {
    ensure_permission("dict:view").await?;
    let list = dict_data.get_by_dict_type(&dict_code).await?;
    Ok(ApiR::success(list))
}
