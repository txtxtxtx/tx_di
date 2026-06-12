//! 用户管理 HTTP API

use axum::{Json, Router, routing::{get, post, put, delete}};
use axum::response::IntoResponse;
use tx_di_axum::bound::DiComp;
use admin_app::user::app_service::UserAppService;

use admin_proto::{
    CreateUserRequest, UpdateUserRequest, ChangePasswordRequest,
    AssignRolesRequest, AssignDeptsRequest, ListUsersRequest,
};
use admin_domain::user::model::value_object::{Sex, UserStatus};
use tx_common::{ApiR, ApiRes};

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
}

/// POST /api/user/
async fn create_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::user::dto::CreateUserCommand {
        username: req.username,
        password: req.password,
        nickname: req.nickname,
        email: req.email,
        mobile: req.mobile,
        sex: req.sex.map(Sex::from),
        remark: req.remark,
        role_ids: if req.role_ids.is_empty() { None } else { Some(req.role_ids) },
        dept_ids: if req.dept_ids.is_empty() { None } else { Some(req.dept_ids) },
    };
    match user_svc.create_user(cmd, None).await {
        Ok(r) => ApiR::success(r),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// GET /api/user/{user_id}
async fn get_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match user_svc.get_user(user_id).await {
        Ok(r) => ApiR::success(r),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// PUT /api/user/{user_id}
async fn update_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::user::dto::UpdateUserCommand {
        user_id,
        nickname: req.nickname,
        email: req.email,
        mobile: req.mobile,
        sex: req.sex.map(Sex::from),
        status: req.status.and_then(|s| UserStatus::try_from_i32(s).ok()),
        remark: req.remark,
    };
    match user_svc.update_user(cmd, None).await {
        Ok(r) => ApiR::success(r),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// DELETE /api/user/{user_id}
async fn delete_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match user_svc.delete_user(user_id, None).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

/// POST /api/user/list
async fn list_users(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ListUsersRequest>,
) -> impl IntoResponse {
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
    match user_svc.get_user_page(query).await {
        Ok(page) => ApiR::success(page),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// POST /api/user/change-password
async fn change_password(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::user::dto::ChangePasswordCommand {
        user_id: req.user_id,
        new_password: req.new_password,
    };
    match user_svc.change_password(cmd, None).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

/// POST /api/user/assign-roles
async fn assign_roles(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<AssignRolesRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::user::dto::AssignRolesCommand {
        user_id: req.user_id,
        role_ids: req.role_ids,
    };
    match user_svc.assign_roles(cmd).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

/// POST /api/user/assign-depts
async fn assign_depts(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<AssignDeptsRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::user::dto::AssignDeptsCommand {
        user_id: req.user_id,
        dept_ids: req.dept_ids,
    };
    match user_svc.assign_departments(cmd).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}
