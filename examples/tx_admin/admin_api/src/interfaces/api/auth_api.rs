//! 认证 HTTP API

use axum::Json;
use tx_di_axum::{Router, R};
use tx_di_axum::bound::DiComp;
use tx_di_sa_token::{StpUtil, LoginIdExtractor, sa_check_permission};
use admin_app::auth::app_service::AuthAppService;
use admin_proto::{LoginRequest, Empty};
use tx_common::{ApiR, ApiRes};
use crate::error::ApiErr;

/// 公开路由（无需认证）
pub fn open_router() -> Router {
    use axum::routing::post;
    Router::new()
        .route("/api/auth/login", post(login))
}

/// 受保护路由（需要认证）
pub fn router() -> Router {
    use axum::routing::{get, post};
    Router::new()
        .route("/api/auth/user-info", get(user_info))
        .route("/api/auth/logout", post(logout))
}

/// POST /api/auth/login
async fn login(
    DiComp(auth): DiComp<AuthAppService>,
    Json(req): Json<LoginRequest>,
) -> Result<R<admin_app::auth::dto::LoginResponse>, ApiErr> {
    let cmd = admin_app::auth::dto::LoginCommand {
        username: req.username,
        password: req.password,
        login_ip: req.login_ip,
    };
    let r = auth.login(cmd).await?;
    let token = StpUtil::login(r.user_id.to_string()).await?;

    // 根据角色设置权限
    // TODO: 当前使用硬编码逻辑判断管理员（role_ids.contains(&1)），应改为：
    //   1. 从 RoleRepository 查询用户所有角色，使用角色的真实名称（而非硬编码 "admin"/"user"）
    //   2. 判断管理员应基于角色编码（如 role.code == "super_admin"）而非 ID == 1
    //   3. StpUtil::set_permissions/set_roles 的错误不应被 `let _` 静默丢弃
    let user_id_str = r.user_id.to_string();
    let is_admin = r.role_ids.contains(&1);
    if is_admin {
        let _ = StpUtil::set_permissions(&user_id_str, vec!["*".to_string()]).await;
        let _ = StpUtil::set_roles(&user_id_str, vec!["admin".to_string()]).await;
    } else {
        let _ = StpUtil::set_permissions(&user_id_str, r.permissions.clone()).await;
        let _ = StpUtil::set_roles(&user_id_str, vec!["user".to_string()]).await;
    }

    let mut resp = R(ApiR::success(r));
    resp.0.msg = token.to_string();
    Ok(resp)
}

/// GET /api/auth/user-info
#[sa_check_permission("auth:info")]
async fn user_info(
    DiComp(auth): DiComp<AuthAppService>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> Result<R<admin_app::auth::dto::UserInfoResponse>, ApiErr> {
    let user_id: u64 = login_id.parse().unwrap_or(0);
    let r = auth.get_user_info(user_id).await?;
    Ok(R(ApiR::success(r)))
}

/// POST /api/auth/logout
#[sa_check_permission("auth:logout")]
async fn logout() -> Result<R<Empty>, ApiErr> {
    StpUtil::logout_current().await?;
    Ok(R(ApiRes::ok().into_typed()))
}
