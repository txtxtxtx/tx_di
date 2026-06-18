//! 用户管理 HTTP API

use axum::Json;
use tx_di_sa_token::StpUtil;
use tx_di_axum::Router;
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::user::app_service::UserAppService;
use crate::auth::ensure_permission;

use admin_proto::{
    CreateUserRequest, UpdateUserRequest, ChangePasswordRequest,
    AssignRolesRequest, AssignDeptsRequest, ListUsersRequest,
    ChangeUserStatusRequest, UserResponse, Empty, UserIdRequest,
};
use admin_domain::user::model::value_object::UserStatus;
use tx_common::{ApiR, ApiRes, Page};
use crate::error::ApiErr;

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_user))
        .route("/{user_id}", get(get_user))
        .route("/{user_id}", put(update_user))
        .route("/{user_id}", delete(delete_user))
        .route("/list", post(list_users))
        .route("/change_password", post(change_password))
        .route("/assign_roles", post(assign_roles))
        .route("/assign_depts", post(assign_depts))
        .route("/change_status", post(change_user_status))
        .route("/enable", post(enable_user))
        .route("/disable", post(disable_user))
        .route("/lock", post(lock_user))
        .route("/unlock", post(unlock_user))
}

/// POST /api/user/
async fn create_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<CreateUserRequest>,
) -> Result<ApiR<UserResponse>, ApiErr> {
    ensure_permission("user:create").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = user_svc.create_user(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

/// GET /api/user/{user_id}
async fn get_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> Result<ApiR<UserResponse>, ApiErr> {
    ensure_permission("user:view").await?;
    let r = user_svc.get_user(user_id).await?;
    Ok(ApiR::success(r))
}

/// PUT /api/user/{user_id}
async fn update_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateUserRequest>,
) -> Result<ApiR<UserResponse>, ApiErr> {
    let login_id = StpUtil::get_login_id_as_string().await?;
    if login_id != user_id.to_string() {
        ensure_permission("user:update").await?;
    }
    req.user_id = user_id;

    let r = user_svc.update_user(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

/// DELETE /api/user/{user_id}
async fn delete_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("user:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.delete_user(user_id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

/// POST /api/user/list
async fn list_users(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ListUsersRequest>,
) -> Result<ApiR<Page<UserResponse>>, ApiErr> {
    ensure_permission("user:view").await?;
    let page = user_svc.get_user_page(req).await?;
    Ok(ApiR::success(page))
}

/// POST /api/user/change-password
async fn change_password(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    let login_id = StpUtil::get_login_id_as_string().await?;
    if req.user_id.to_string() != login_id {
        ensure_permission("user:password").await?;
    }
    user_svc.change_password(req, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

/// POST /api/user/assign-roles
async fn assign_roles(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<AssignRolesRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("user:assign_role").await?;
    user_svc.assign_roles(req).await?;
    Ok(ApiRes::ok().into_typed())
}

/// POST /api/user/assign-depts
async fn assign_depts(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<AssignDeptsRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("user:assign_dept").await?;
    user_svc.assign_departments(req).await?;
    Ok(ApiRes::ok().into_typed())
}

/// POST /api/user/change-status
async fn change_user_status(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ChangeUserStatusRequest>,
) -> Result<ApiR<UserResponse>, ApiErr> {
    ensure_permission("user:status").await?;
    let status = UserStatus::try_from_i32(req.status)
        .map_err(|_| anyhow::anyhow!("invalid status"))?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = user_svc.change_status(req.user_id, status, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

/// POST /api/user/enable
async fn enable_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("user:status").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.change_status(req.user_id, UserStatus::Active, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

/// POST /api/user/disable
async fn disable_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("user:status").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.change_status(req.user_id, UserStatus::Disabled, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

/// POST /api/user/lock
async fn lock_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("user:status").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.change_status(req.user_id, UserStatus::Locked, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

/// POST /api/user/unlock
async fn unlock_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("user:status").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.change_status(req.user_id, UserStatus::Active, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}
