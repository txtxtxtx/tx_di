//! 用户管理 API

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tx_di_core::App;

use crate::application::user::{CreateUserRequest, UpdateUserRequest, UserService};
use crate::domain::user::UserStatus;
use crate::infrastructure::persistence::InMemoryUserRepository;
use crate::interfaces::dto::common::{ApiResponse, PageQuery, PageResponse};

/// 用户路由
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
        .route("/{id}/reset-password", put(reset_password))
        .with_state(app)
}

/// 分页查询用户
async fn list_users(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Json<ApiResponse<PageResponse<crate::domain::user::User>>> {
    let service = app.inject::<UserService>();

    match service
        .list_users(
            "t-default-001",
            query.keyword.as_deref(),
            None,
            query.page,
            query.page_size,
        )
        .await
    {
        Ok((users, total)) => {
            Json(ApiResponse::success(PageResponse::new(users, total, query.page, query.page_size)))
        }
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}

/// 获取用户详情
async fn get_user(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<crate::domain::user::User>> {
    let user_repo = app.inject::<InMemoryUserRepository>();
    match user_repo.find_by_id(&id).await {
        Ok(Some(user)) => Json(ApiResponse::success(user)),
        Ok(None) => Json(ApiResponse::error(404, "用户不存在")),
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}

/// 创建用户
async fn create_user(
    State(app): State<Arc<App>>,
    Json(body): Json<CreateUserRequest>,
) -> Json<ApiResponse<crate::domain::user::User>> {
    let service = app.inject::<UserService>();

    match service.create_user("t-default-001", body).await {
        Ok(user) => Json(ApiResponse::success(user)),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}

/// 更新用户
async fn update_user(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateUserRequest>,
) -> Json<ApiResponse<crate::domain::user::User>> {
    let service = app.inject::<UserService>();

    match service.update_user(&id, body).await {
        Ok(user) => Json(ApiResponse::success(user)),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}

/// 删除用户
async fn delete_user(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let service = app.inject::<UserService>();

    match service.delete_user(&id).await {
        Ok(()) => Json(ApiResponse::ok()),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}

/// 重置密码
async fn reset_password(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<ApiResponse<()>> {
    let service = app.inject::<UserService>();
    let password = body["password"].as_str().unwrap_or("123456");

    match service.reset_password(&id, password).await {
        Ok(()) => Json(ApiResponse::ok()),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}
