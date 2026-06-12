//! 部门管理 HTTP API

use axum::{Json, Router, routing::{get, post, put, delete}};
use axum::response::IntoResponse;
use tx_di_axum::bound::DiComp;
use admin_app::department::app_service::DepartmentAppService;
use admin_proto::{CreateDeptRequest, UpdateDeptRequest, ListDeptsRequest, DeptResponse};
use tx_common::{ApiR, ApiRes};

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_dept))
        .route("/{dept_id}", get(get_dept))
        .route("/{dept_id}", put(update_dept))
        .route("/{dept_id}", delete(delete_dept))
        .route("/list", post(list_depts))
}

fn map_dept(d: admin_app::department::dto::DeptResponse) -> DeptResponse { DeptResponse { id: d.id, name: d.name, parent_id: d.parent_id, sort: d.sort, leader_user_id: d.leader_user_id, phone: d.phone, email: d.email, status: d.status } }

async fn create_dept(DiComp(dept): DiComp<DepartmentAppService>, Json(req): Json<CreateDeptRequest>) -> impl IntoResponse {
    let cmd = admin_app::department::dto::CreateDeptCommand { name: req.name, parent_id: req.parent_id, sort: req.sort, leader_user_id: req.leader_user_id, phone: req.phone, email: req.email };
    match dept.create_dept(cmd, None).await { Ok(r) => ApiR::success(map_dept(r)), Err(e) => ApiRes::from(e).into_typed() }
}

async fn get_dept(DiComp(dept): DiComp<DepartmentAppService>, axum::extract::Path(dept_id): axum::extract::Path<u64>) -> impl IntoResponse {
    match dept.get_dept(dept_id).await { Ok(r) => ApiR::success(map_dept(r)), Err(e) => ApiRes::from(e).into_typed() }
}

async fn update_dept(DiComp(dept): DiComp<DepartmentAppService>, axum::extract::Path(dept_id): axum::extract::Path<u64>, Json(req): Json<UpdateDeptRequest>) -> impl IntoResponse {
    let cmd = admin_app::department::dto::UpdateDeptCommand { dept_id, name: req.name, parent_id: req.parent_id, sort: req.sort, leader_user_id: req.leader_user_id, phone: req.phone, email: req.email };
    match dept.update_dept(cmd, None).await { Ok(r) => ApiR::success(map_dept(r)), Err(e) => ApiRes::from(e).into_typed() }
}

async fn delete_dept(DiComp(dept): DiComp<DepartmentAppService>, axum::extract::Path(dept_id): axum::extract::Path<u64>) -> impl IntoResponse {
    match dept.delete_dept(dept_id, None).await { Ok(()) => ApiRes::ok(), Err(e) => ApiRes::from(e) }
}

async fn list_depts(DiComp(dept): DiComp<DepartmentAppService>, Json(req): Json<ListDeptsRequest>) -> impl IntoResponse {
    let q = admin_app::department::dto::DeptQueryRequest { name: None, status: None };
    match dept.get_dept_tree(q).await { Ok(tree) => ApiR::success(tree), Err(e) => ApiRes::from(e).into_typed() }
}
