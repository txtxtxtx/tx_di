//! 用户管理 HTTP API

use axum::Json;
use tx_di_sa_token::StpUtil;
use tx_di_axum::{R, Router};
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::user::app_service::UserAppService;
use crate::auth::ensure_permission;

use admin_proto::{
    CreateUserRequest, UpdateUserRequest, ChangePasswordRequest,
    AssignRolesRequest, AssignDeptsRequest, ListUsersRequest,
    ChangeUserStatusRequest, UserResponse, Empty, UserIdRequest,
};
use admin_domain::user::model::value_object::{Sex, UserStatus};
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
        .route("/change-status", post(change_user_status))
        .route("/enable", post(enable_user))
        .route("/disable", post(disable_user))
        .route("/lock", post(lock_user))
        .route("/unlock", post(unlock_user))
}

/// POST /api/user/
async fn create_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<CreateUserRequest>,
) -> Result<R<UserResponse>, ApiErr> {
    ensure_permission("user:create").await?;
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::user::dto::CreateUserCommand {
        username: req.username,
        password: req.password,
        nickname: req.nickname,
        email: opt_filter(req.email),
        mobile: opt_filter(req.mobile),
        sex: req.sex.map(Sex::from),
        remark: opt_filter(req.remark),
        role_ids: if req.role_ids.is_empty() { None } else { Some(req.role_ids) },
        dept_ids: if req.dept_ids.is_empty() { None } else { Some(req.dept_ids) },
    };
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = user_svc.create_user(cmd, Some(login_id)).await?;
    Ok(R(ApiR::success(r)))
}

/// GET /api/user/{user_id}
async fn get_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> Result<R<UserResponse>, ApiErr> {
    ensure_permission("user:view").await?;
    let r = user_svc.get_user(user_id).await?;
    Ok(R(ApiR::success(r)))
}

/// PUT /api/user/{user_id}
async fn update_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<R<UserResponse>, ApiErr> {
    ensure_permission("user:update").await?;
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::user::dto::UpdateUserCommand {
        user_id,
        nickname: opt_filter(req.nickname),
        email: opt_filter(req.email),
        mobile: opt_filter(req.mobile),
        sex: req.sex.map(Sex::from),
        status: req.status.and_then(|s| UserStatus::try_from_i32(s).ok()),
        remark: opt_filter(req.remark),
    };
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = user_svc.update_user(cmd, Some(login_id)).await?;
    Ok(R(ApiR::success(r)))
}

/// DELETE /api/user/{user_id}
async fn delete_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("user:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.delete_user(user_id, Some(login_id)).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// POST /api/user/list
async fn list_users(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ListUsersRequest>,
) -> Result<R<Page<UserResponse>>, ApiErr> {
    ensure_permission("user:view").await?;
    let status = match req.status {
        Some(s) => UserStatus::try_from_i32(s).ok(),
        None => None,
    };
    let page_info = req.page_info.unwrap_or_default();
    let query = admin_app::user::dto::UserQueryRequest {
        username: req.username,
        nickname: req.nickname,
        mobile: req.mobile,
        status,
        dept_id: req.dept_id,
        page: page_info.page,
        size: page_info.size,
    };
    let page = user_svc.get_user_page(query).await?;
    Ok(R(ApiR::success(page)))
}

/// POST /api/user/change-password
async fn change_password(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("user:password").await?;
    let cmd = admin_app::user::dto::ChangePasswordCommand {
        user_id: req.user_id,
        new_password: req.new_password,
    };
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.change_password(cmd, Some(login_id)).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// POST /api/user/assign-roles
async fn assign_roles(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<AssignRolesRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("user:assign_role").await?;
    let cmd = admin_app::user::dto::AssignRolesCommand {
        user_id: req.user_id,
        role_ids: req.role_ids,
    };
    user_svc.assign_roles(cmd).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// POST /api/user/assign-depts
async fn assign_depts(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<AssignDeptsRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("user:assign_dept").await?;
    let cmd = admin_app::user::dto::AssignDeptsCommand {
        user_id: req.user_id,
        dept_ids: req.dept_ids,
    };
    user_svc.assign_departments(cmd).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// POST /api/user/change-status
async fn change_user_status(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ChangeUserStatusRequest>,
) -> Result<R<UserResponse>, ApiErr> {
    ensure_permission("user:status").await?;
    let status = UserStatus::try_from_i32(req.status)
        .map_err(|_| anyhow::anyhow!("invalid status"))?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = user_svc.change_status(req.user_id, status, Some(login_id)).await?;
    Ok(R(ApiR::success(r)))
}

/// POST /api/user/enable
async fn enable_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("user:status").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.change_status(req.user_id, UserStatus::Active, Some(login_id)).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// POST /api/user/disable
async fn disable_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("user:status").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.change_status(req.user_id, UserStatus::Disabled, Some(login_id)).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// POST /api/user/lock
async fn lock_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("user:status").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.change_status(req.user_id, UserStatus::Locked, Some(login_id)).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// POST /api/user/unlock
async fn unlock_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("user:status").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    user_svc.change_status(req.user_id, UserStatus::Active, Some(login_id)).await?;
    Ok(R(ApiRes::ok().into_typed()))
}
