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
use tx_di_sa_token::sa_check_permission;
use crate::error::ApiErr;

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_dept))
        .route("/{dept_id}", get(get_dept))
        .route("/{dept_id}", put(update_dept))
        .route("/{dept_id}", delete(delete_dept))
        .route("/list", post(list_depts))
}

fn map_dept(d: admin_app::department::dto::DeptResponse) -> DeptResponse {
    DeptResponse {
        id: d.id,
        name: d.name,
        parent_id: d.parent_id,
        sort: d.sort,
        leader_user_id: d.leader_user_id,
        phone: d.phone,
        email: d.email,
        status: d.status,
    }
}

#[sa_check_permission("dept:create")]
async fn create_dept(
    DiComp(dept): DiComp<DepartmentAppService>,
    Json(req): Json<CreateDeptRequest>,
) -> Result<R<DeptResponse>, ApiErr> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::department::dto::CreateDeptCommand {
        name: req.name,
        parent_id: req.parent_id,
        sort: req.sort,
        leader_user_id: req.leader_user_id,
        phone: opt_filter(req.phone),
        email: opt_filter(req.email),
    };
    let r = dept.create_dept(cmd, None).await?;
    Ok(R(ApiR::success(map_dept(r))))
}

#[sa_check_permission("dept:view")]
async fn get_dept(
    DiComp(dept): DiComp<DepartmentAppService>,
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
) -> Result<R<DeptResponse>, ApiErr> {
    let r = dept.get_dept(dept_id).await?;
    Ok(R(ApiR::success(map_dept(r))))
}

#[sa_check_permission("dept:update")]
async fn update_dept(
    DiComp(dept): DiComp<DepartmentAppService>,
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateDeptRequest>,
) -> Result<R<DeptResponse>, ApiErr> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::department::dto::UpdateDeptCommand {
        dept_id,
        name: req.name,
        parent_id: req.parent_id,
        sort: req.sort,
        leader_user_id: req.leader_user_id,
        phone: opt_filter(req.phone),
        email: opt_filter(req.email),
    };
    let r = dept.update_dept(cmd, None).await?;
    Ok(R(ApiR::success(map_dept(r))))
}

#[sa_check_permission("dept:delete")]
async fn delete_dept(
    DiComp(dept): DiComp<DepartmentAppService>,
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    dept.delete_dept(dept_id, None).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

#[sa_check_permission("dept:view")]
async fn list_depts(
    DiComp(dept): DiComp<DepartmentAppService>,
    Json(req): Json<ListDeptsRequest>,
) -> Result<R<Vec<DeptTreeNode>>, ApiErr> {
    let q = admin_app::department::dto::DeptQueryRequest {
        name: opt_filter(req.name),
        status: req.status,
    };
    let tree = dept.get_dept_tree(q).await?;
    Ok(R(ApiR::success(tree)))
}
