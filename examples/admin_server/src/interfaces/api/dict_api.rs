//! 字典管理 API
//!
//! 包含字典类型和字典数据的 CRUD 接口。

use axum::{Json, Router, extract::{Path, Query, State}, routing::{delete, get, post, put}};
use std::sync::Arc;
use tx_di_core::App;

use crate::domain::{AppError, AdminErr};
use crate::domain::dict::DictRepository;
use crate::domain::dict::repo::ToastyDictRepository;
use crate::domain::dept::CommonStatus;
use tx_common::{ApiR, ApiRes, Page};
use crate::interfaces::dto::common::PageQuery;
use crate::interfaces::dto::dict_dto::{
    DictTypeDto, CreateDictTypeRequest, UpdateDictTypeRequest,
    DictDataDto, CreateDictDataRequest, UpdateDictDataRequest,
};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        // 字典类型
        .route("/types", get(list_dict_types).post(create_dict_type))
        .route("/types/{id}", get(get_dict_type).put(update_dict_type).delete(delete_dict_type))
        // 字典数据
        .route("/data", get(list_dict_data).post(create_dict_data))
        .route("/data/{id}", get(get_dict_data).put(update_dict_data).delete(delete_dict_data))
        // 按类型获取字典数据列表（不分页，供前端下拉等使用）
        .route("/data/type/{dict_type}", get(list_dict_data_by_type))
        .with_state(app)
}

// ── 字典类型 ─────────────────────────────────────────────

async fn list_dict_types(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiR<Page<DictTypeDto>>>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    let (types, total) = repo.find_type_page(query.keyword.as_deref(), query.page as u64, query.size as u64).await?;
    let dtos: Vec<DictTypeDto> = types.iter().map(DictTypeDto::from).collect();
    Ok(Json(ApiR::success(Page::new(dtos, query.page, query.size, total as i64))))
}

async fn get_dict_type(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiR<DictTypeDto>>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    let dt = repo.find_type_by_id(id).await?.ok_or(AppError::with_context(AdminErr::DictTypeNotFound, id.to_string()))?;
    Ok(Json(ApiR::success(DictTypeDto::from(&dt))))
}

async fn create_dict_type(
    State(app): State<Arc<App>>,
    Json(req): Json<CreateDictTypeRequest>,
) -> Result<Json<ApiR<DictTypeDto>>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    let mut dt = crate::domain::dict::DictType::new(req.name, req.dict_type);
    if let Some(s) = req.status {
        dt.status = match s.as_str() { "enable" | "1" => CommonStatus::Enable, _ => CommonStatus::Disable };
    }
    dt.remark = req.remark;
    repo.save_type(&dt).await?;
    Ok(Json(ApiR::success(DictTypeDto::from(&dt))))
}

async fn update_dict_type(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateDictTypeRequest>,
) -> Result<Json<ApiR<DictTypeDto>>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    let mut dt = repo.find_type_by_id(id).await?.ok_or(AppError::with_context(AdminErr::DictTypeNotFound, id.to_string()))?;
    if let Some(v) = req.name { dt.name = v; }
    if let Some(v) = req.dict_type { dt.dict_type = v; }
    if let Some(v) = req.status {
        dt.status = match v.as_str() { "enable" | "1" => CommonStatus::Enable, _ => CommonStatus::Disable };
    }
    if let Some(v) = req.remark { dt.remark = Some(v); }
    repo.save_type(&dt).await?;
    Ok(Json(ApiR::success(DictTypeDto::from(&dt))))
}

async fn delete_dict_type(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiRes>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    repo.find_type_by_id(id).await?.ok_or(AppError::with_context(AdminErr::DictTypeNotFound, id.to_string()))?;
    repo.delete_type(id).await?;
    Ok(Json(ApiRes::ok()))
}

// ── 字典数据 ─────────────────────────────────────────────

async fn list_dict_data(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiR<Page<DictDataDto>>>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    // 字典数据按类型分页：keyword 作为 dict_type 过滤条件
    let (data, total) = if let Some(ref dict_type) = query.keyword {
        // 简化实现：先获取全部再手动分页
        let all = repo.find_data_by_type(dict_type).await?;
        let total = all.len() as i64;
        let start = ((query.page - 1) * query.size) as usize;
        let page_data: Vec<_> = all.into_iter().skip(start).take(query.size as usize).collect();
        (page_data, total)
    } else {
        // 没有 keyword 时返回空（字典数据通常需要按类型查询）
        (vec![], 0)
    };
    let dtos: Vec<DictDataDto> = data.iter().map(DictDataDto::from).collect();
    Ok(Json(ApiR::success(Page::new(dtos, query.page, query.size, total as i64))))
}

async fn list_dict_data_by_type(
    State(app): State<Arc<App>>,
    Path(dict_type): Path<String>,
) -> Result<Json<ApiR<Vec<DictDataDto>>>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    let data = repo.find_data_by_type(&dict_type).await?;
    let dtos: Vec<DictDataDto> = data.iter().map(DictDataDto::from).collect();
    Ok(Json(ApiR::success(dtos)))
}

async fn get_dict_data(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiR<DictDataDto>>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    let data = repo.find_data_by_id(id).await?.ok_or(AppError::with_context(AdminErr::DictDataNotFound, id.to_string()))?;
    Ok(Json(ApiR::success(DictDataDto::from(&data))))
}

async fn create_dict_data(
    State(app): State<Arc<App>>,
    Json(req): Json<CreateDictDataRequest>,
) -> Result<Json<ApiR<DictDataDto>>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    let mut data = crate::domain::dict::DictData::new(req.dict_type, req.label, req.value, req.sort.unwrap_or(0));
    if let Some(s) = req.status {
        data.status = match s.as_str() { "enable" | "1" => CommonStatus::Enable, _ => CommonStatus::Disable };
    }
    data.color_type = req.color_type;
    data.css_class = req.css_class;
    data.remark = req.remark;
    repo.save_data(&data).await?;
    Ok(Json(ApiR::success(DictDataDto::from(&data))))
}

async fn update_dict_data(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateDictDataRequest>,
) -> Result<Json<ApiR<DictDataDto>>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    let mut data = repo.find_data_by_id(id).await?.ok_or(AppError::with_context(AdminErr::DictDataNotFound, id.to_string()))?;
    if let Some(v) = req.sort { data.sort = v; }
    if let Some(v) = req.label { data.label = v; }
    if let Some(v) = req.value { data.value = v; }
    if let Some(v) = req.dict_type { data.dict_type = v; }
    if let Some(v) = req.status {
        data.status = match v.as_str() { "enable" | "1" => CommonStatus::Enable, _ => CommonStatus::Disable };
    }
    if let Some(v) = req.color_type { data.color_type = Some(v); }
    if let Some(v) = req.css_class { data.css_class = Some(v); }
    if let Some(v) = req.remark { data.remark = Some(v); }
    repo.save_data(&data).await?;
    Ok(Json(ApiR::success(DictDataDto::from(&data))))
}

async fn delete_dict_data(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiRes>, AppError> {
    let repo = app.inject::<ToastyDictRepository>();
    repo.find_data_by_id(id).await?.ok_or(AppError::with_context(AdminErr::DictDataNotFound, id.to_string()))?;
    repo.delete_data(id).await?;
    Ok(Json(ApiRes::ok()))
}
