//! 部门管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use axum::response::IntoResponse;
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateDeptRequest, UpdateDeptRequest, ListDeptsRequest, DeptResponse, Empty,
};
use crate::services;
use tx_common::{ApiR, ApiRes};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", post(create_dept))
        .route("/{dept_id}", get(get_dept))
        .route("/{dept_id}", put(update_dept))
        .route("/{dept_id}", delete(delete_dept))
        .route("/list", post(list_depts))
        .with_state(app)
}

fn map_dept(d: admin_app::department::dto::DeptResponse) -> DeptResponse {
    DeptResponse {
        id: d.id, name: d.name, parent_id: d.parent_id, sort: d.sort,
        leader_user_id: d.leader_user_id, phone: d.phone, email: d.email,
        status: d.status,
    }
}

async fn create_dept(
    Json(req): Json<CreateDeptRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::department::dto::CreateDeptCommand {
        name: req.name, parent_id: req.parent_id, sort: req.sort,
        leader_user_id: req.leader_user_id, phone: req.phone, email: req.email,
    };
    match services::get().dept.create_dept(cmd, None).await {
        Ok(r) => ApiR::success(map_dept(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn get_dept(
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let query = admin_app::department::dto::DeptQueryRequest { name: None, status: None };
    match services::get().dept.get_dept_list(query).await {
        Ok(list) => {
            match list.into_iter().find(|d| d.id == dept_id) {
                Some(d) => ApiR::success(map_dept(d)),
                None => ApiRes::error(404, "部门不存在".into()).into_typed(),
            }
        }
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn update_dept(
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateDeptRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::department::dto::UpdateDeptCommand {
        dept_id, name: req.name, parent_id: req.parent_id, sort: req.sort,
        leader_user_id: req.leader_user_id, phone: req.phone, email: req.email,
    };
    match services::get().dept.update_dept(cmd, None).await {
        Ok(r) => ApiR::success(map_dept(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn delete_dept(
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match services::get().dept.delete_dept(dept_id, None).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

async fn list_depts(
    Json(req): Json<ListDeptsRequest>,
) -> impl IntoResponse {
    let query = admin_app::department::dto::DeptQueryRequest {
        name: req.name, status: req.status,
    };
    match services::get().dept.get_dept_list(query).await {
        Ok(list) => ApiR::success(list.into_iter().map(map_dept).collect::<Vec<_>>()),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}
