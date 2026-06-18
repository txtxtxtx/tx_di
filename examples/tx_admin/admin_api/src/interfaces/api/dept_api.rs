//! 部门管理 HTTP API

use admin_app::department::app_service::DepartmentAppService;
use admin_domain::department::model::value_object::DeptTreeNode;
use admin_proto::{CreateDeptRequest, DeptResponse, Empty, ListDeptsRequest, UpdateDeptRequest};
use axum::Json;
use admin_app::empty_string::opt_filter;
use tx_common::{ApiR, ApiRes};
use axum::routing::{delete, get, post, put};
use tx_di_axum::bound::DiComp;
use tx_di_axum::{R, Router};
use crate::auth::ensure_permission;
use crate::error::ApiErr;
use tx_di_sa_token::StpUtil;

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_dept))
        .route("/{dept_id}", get(get_dept))
        .route("/{dept_id}", put(update_dept))
        .route("/{dept_id}", delete(delete_dept))
        .route("/list", post(list_depts))
}

async fn create_dept(
    DiComp(dept): DiComp<DepartmentAppService>,
    Json(mut req): Json<CreateDeptRequest>,
) -> Result<R<DeptResponse>, ApiErr> {
    ensure_permission("dept:create").await?;
    req.phone = opt_filter(req.phone);
    req.email = opt_filter(req.email);
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = dept.create_dept(req, Some(login_id)).await?;
    Ok(R(ApiR::success(r)))
}

async fn get_dept(
    DiComp(dept): DiComp<DepartmentAppService>,
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
) -> Result<R<DeptResponse>, ApiErr> {
    ensure_permission("dept:view").await?;
    let r = dept.get_dept(dept_id).await?;
    Ok(R(ApiR::success(r)))
}

async fn update_dept(
    DiComp(dept): DiComp<DepartmentAppService>,
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateDeptRequest>,
) -> Result<R<DeptResponse>, ApiErr> {
    ensure_permission("dept:update").await?;
    req.dept_id = dept_id;
    req.phone = opt_filter(req.phone);
    req.email = opt_filter(req.email);
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = dept.update_dept(req, Some(login_id)).await?;
    Ok(R(ApiR::success(r)))
}

async fn delete_dept(
    DiComp(dept): DiComp<DepartmentAppService>,
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("dept:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    dept.delete_dept(dept_id, Some(login_id)).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

async fn list_depts(
    DiComp(dept): DiComp<DepartmentAppService>,
    Json(mut req): Json<ListDeptsRequest>,
) -> Result<R<Vec<DeptTreeNode>>, ApiErr> {
    ensure_permission("dept:view").await?;
    req.name = opt_filter(req.name);
    let tree = dept.get_dept_tree(req).await?;
    Ok(R(ApiR::success(tree)))
}
