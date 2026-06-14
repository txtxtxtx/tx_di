//! 用户管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use tx_di_axum::aide::axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::user::app_service::UserAppService;

use admin_proto::{
    CreateUserRequest, UpdateUserRequest, ChangePasswordRequest,
    AssignRolesRequest, AssignDeptsRequest, ListUsersRequest,
    ChangeUserStatusRequest, UserResponse, Empty, UserIdRequest,
};
use admin_domain::user::model::value_object::{Sex, UserStatus};
use tx_common::{ApiR, ApiRes, Page};

pub fn router() -> Router {
    Router::new()
        .api_route("/", post(create_user))
        .api_route("/{user_id}", get(get_user))
        .api_route("/{user_id}", put(update_user))
        .api_route("/{user_id}", delete(delete_user))
        .api_route("/list", post(list_users))
        .api_route("/change_password", post(change_password))
        .api_route("/assign_roles", post(assign_roles))
        .api_route("/assign_depts", post(assign_depts))
        .api_route("/change-status", post(change_user_status))
        .api_route("/enable", post(enable_user))
        .api_route("/disable", post(disable_user))
        .api_route("/lock", post(lock_user))
        .api_route("/unlock", post(unlock_user))
}

/// POST /api/user/
async fn create_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<CreateUserRequest>,
) -> R<UserResponse> {
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
    match user_svc.create_user(cmd, None).await {
        Ok(r) => R(ApiR::success(r)),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// GET /api/user/{user_id}
async fn get_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> R<UserResponse> {
    match user_svc.get_user(user_id).await {
        Ok(r) => R(ApiR::success(r)),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// PUT /api/user/{user_id}
async fn update_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateUserRequest>,
) -> R<UserResponse> {
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
    match user_svc.update_user(cmd, None).await {
        Ok(r) => R(ApiR::success(r)),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// DELETE /api/user/{user_id}
async fn delete_user(
    DiComp(user_svc): DiComp<UserAppService>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> R<Empty> {
    match user_svc.delete_user(user_id, None).await {
        Ok(()) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/user/list
async fn list_users(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ListUsersRequest>,
) -> R<Page<UserResponse>> {
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
        Ok(page) => R(ApiR::success(page)),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/user/change-password
async fn change_password(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ChangePasswordRequest>,
) -> R<Empty> {
    let cmd = admin_app::user::dto::ChangePasswordCommand {
        user_id: req.user_id,
        new_password: req.new_password,
    };
    match user_svc.change_password(cmd, None).await {
        Ok(()) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/user/assign-roles
async fn assign_roles(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<AssignRolesRequest>,
) -> R<Empty> {
    let cmd = admin_app::user::dto::AssignRolesCommand {
        user_id: req.user_id,
        role_ids: req.role_ids,
    };
    match user_svc.assign_roles(cmd).await {
        Ok(()) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/user/assign-depts
async fn assign_depts(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<AssignDeptsRequest>,
) -> R<Empty> {
    let cmd = admin_app::user::dto::AssignDeptsCommand {
        user_id: req.user_id,
        dept_ids: req.dept_ids,
    };
    match user_svc.assign_departments(cmd).await {
        Ok(()) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/user/change-status
async fn change_user_status(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<ChangeUserStatusRequest>,
) -> R<UserResponse> {
    let status = match UserStatus::try_from_i32(req.status) {
        Ok(s) => s,
        Err(_) => return R(ApiRes::fail("invalid status".into()).into_typed()),
    };
    match user_svc.change_status(req.user_id, status, None).await {
        Ok(r) => R(ApiR::success(r)),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/user/enable
async fn enable_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> R<Empty> {
    match user_svc.change_status(req.user_id, UserStatus::Active, None).await {
        Ok(_) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/user/disable
async fn disable_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> R<Empty> {
    match user_svc.change_status(req.user_id, UserStatus::Disabled, None).await {
        Ok(_) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/user/lock
async fn lock_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> R<Empty> {
    match user_svc.change_status(req.user_id, UserStatus::Locked, None).await {
        Ok(_) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/user/unlock
async fn unlock_user(
    DiComp(user_svc): DiComp<UserAppService>,
    Json(req): Json<UserIdRequest>,
) -> R<Empty> {
    match user_svc.change_status(req.user_id, UserStatus::Active, None).await {
        Ok(_) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}
