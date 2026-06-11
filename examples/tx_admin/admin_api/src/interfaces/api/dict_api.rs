//! 字典管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use axum::response::IntoResponse;
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateDictTypeRequest, UpdateDictTypeRequest, ListDictTypesRequest, DictTypeResponse,
    CreateDictDataRequest, UpdateDictDataRequest, ListDictDataRequest, DictDataResponse, Empty,
};
use crate::services;
use tx_common::{ApiR, ApiRes, Page};

pub fn router(app: Arc<App>) -> Router {
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
        .with_state(app)
}

// ── 字典类型 ──

fn map_dict_type(d: admin_app::dictionary::dto::DictTypeResponse) -> DictTypeResponse {
    DictTypeResponse {
        id: d.id, name: d.name, dict_type: d.dict_type,
        status: d.status, remark: d.remark,
    }
}

async fn create_dict_type(Json(req): Json<CreateDictTypeRequest>) -> impl IntoResponse {
    let cmd = admin_app::dictionary::dto::CreateDictTypeCommand {
        name: req.name, dict_type: req.dict_type, remark: req.remark,
    };
    match services::get().dict_type.create_dict_type(cmd, None).await {
        Ok(r) => ApiR::success(map_dict_type(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn get_dict_type(axum::extract::Path(id): axum::extract::Path<u64>) -> impl IntoResponse {
    let q = admin_app::dictionary::dto::DictTypeQueryRequest {
        name: None, dict_type: None, status: None, page: 1, size: 100,
    };
    match services::get().dict_type.get_dict_type_page(q).await {
        Ok(page) => match page.list.into_iter().find(|d| d.id == id) {
            Some(d) => ApiR::success(map_dict_type(d)),
            None => ApiRes::error(404, "字典类型不存在".into()).into_typed(),
        },
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn update_dict_type(axum::extract::Path(id): axum::extract::Path<u64>, Json(req): Json<UpdateDictTypeRequest>) -> impl IntoResponse {
    let cmd = admin_app::dictionary::dto::UpdateDictTypeCommand {
        id, name: req.name, dict_type: req.dict_type, remark: req.remark,
    };
    match services::get().dict_type.update_dict_type(cmd, None).await {
        Ok(r) => ApiR::success(map_dict_type(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn delete_dict_type(axum::extract::Path(id): axum::extract::Path<u64>) -> impl IntoResponse {
    match services::get().dict_type.delete_dict_type(id, None).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

async fn list_dict_types(Json(req): Json<ListDictTypesRequest>) -> impl IntoResponse {
    let q = admin_app::dictionary::dto::DictTypeQueryRequest {
        name: req.name, dict_type: req.dict_type, status: req.status,
        page: req.page, size: req.page_size,
    };
    match services::get().dict_type.get_dict_type_page(q).await {
        Ok(page) => ApiR::success(Page::new(
            page.list.into_iter().map(map_dict_type).collect(),
            page.page, page.size, page.total,
        )),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

// ── 字典数据 ──

fn map_dict_data(d: admin_app::dictionary::dto::DictDataResponse) -> DictDataResponse {
    DictDataResponse {
        id: d.id, sort: d.sort, label: d.label, value: d.value,
        dict_type: d.dict_type, status: d.status,
        color_type: d.color_type, css_class: d.css_class, remark: d.remark,
    }
}

async fn create_dict_data(Json(req): Json<CreateDictDataRequest>) -> impl IntoResponse {
    let cmd = admin_app::dictionary::dto::CreateDictDataCommand {
        sort: req.sort, label: req.label, value: req.value, dict_type: req.dict_type,
        color_type: req.color_type, css_class: req.css_class, remark: req.remark,
    };
    match services::get().dict_data.create_dict_data(cmd, None).await {
        Ok(r) => ApiR::success(map_dict_data(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn get_dict_data(axum::extract::Path(id): axum::extract::Path<u64>) -> impl IntoResponse {
    let q = admin_app::dictionary::dto::DictDataQueryRequest {
        dict_type: None, label: None, status: None, page: 1, size: 100,
    };
    match services::get().dict_data.get_dict_data_page(q).await {
        Ok(page) => match page.list.into_iter().find(|d| d.id == id) {
            Some(d) => ApiR::success(map_dict_data(d)),
            None => ApiRes::error(404, "字典数据不存在".into()).into_typed(),
        },
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn update_dict_data(axum::extract::Path(id): axum::extract::Path<u64>, Json(req): Json<UpdateDictDataRequest>) -> impl IntoResponse {
    let cmd = admin_app::dictionary::dto::UpdateDictDataCommand {
        id, sort: req.sort, label: req.label, value: req.value, dict_type: req.dict_type,
        color_type: req.color_type, css_class: req.css_class, remark: req.remark,
    };
    match services::get().dict_data.update_dict_data(cmd, None).await {
        Ok(r) => ApiR::success(map_dict_data(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn delete_dict_data(axum::extract::Path(id): axum::extract::Path<u64>) -> impl IntoResponse {
    match services::get().dict_data.delete_dict_data(id, None).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

async fn list_dict_data(Json(req): Json<ListDictDataRequest>) -> impl IntoResponse {
    let q = admin_app::dictionary::dto::DictDataQueryRequest {
        dict_type: req.dict_type, label: req.label, status: req.status,
        page: req.page, size: req.page_size,
    };
    match services::get().dict_data.get_dict_data_page(q).await {
        Ok(page) => ApiR::success(Page::new(
            page.list.into_iter().map(map_dict_data).collect(),
            page.page, page.size, page.total,
        )),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}
