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
    let login_ip = req.login_ip.clone();
    let cmd = admin_app::auth::dto::LoginCommand {
        username: req.username,
        password: req.password,
        login_ip: req.login_ip,
    };
    let r = auth.login(cmd).await?;
    let token = StpUtil::login(r.user_id.to_string()).await?;

    // 将登录 IP 存入 token 的 extra_data，供在线用户查询使用
    let extra = serde_json::json!({ "login_ip": login_ip });
    let _ = StpUtil::set_extra_data(&token, extra).await;

    // 根据角色设置 sa-token 权限和角色
    let user_id_str = r.user_id.to_string();
    let is_admin = r.role_codes.iter().any(|c| c == "super_admin" || c == "admin");
    if is_admin {
        StpUtil::set_permissions(&user_id_str, vec!["*".to_string()]).await?;
    } else {
        StpUtil::set_permissions(&user_id_str, r.permissions.clone()).await?;
    }
    StpUtil::set_roles(&user_id_str, r.role_codes.clone()).await?;

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
