//! 认证 HTTP API

use axum::Json;
use tx_di_axum::{Router, R};
use tx_di_axum::bound::DiComp;
use tx_di_sa_token::{StpUtil, LoginIdExtractor};
use admin_app::auth::app_service::AuthAppService;
use admin_proto::{LoginRequest, GetUserInfoRequest, LogoutRequest, Empty};
use tx_common::{ApiR, ApiRes};

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
) -> R<admin_app::auth::dto::LoginResponse> {
    let cmd = admin_app::auth::dto::LoginCommand {
        username: req.username,
        password: req.password,
        login_ip: req.login_ip,
    };
    match auth.login(cmd).await {
        Ok(r) => {
            // 登录成功，设置 sa-token
            match StpUtil::login(r.user_id.to_string()).await {
                Ok(token) => {
                    // token 附加在 msg 字段返回
                    let mut resp = R(ApiR::success(r));
                    resp.0.msg = token.to_string();
                    resp
                }
                Err(e) => R(ApiRes::fail(format!("token 创建失败: {}", e)).into_typed()),
            }
        }
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// GET /api/auth/user-info
async fn user_info(
    DiComp(auth): DiComp<AuthAppService>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> R<admin_app::auth::dto::UserInfoResponse> {
    let user_id: u64 = login_id.parse().unwrap_or(0);
    match auth.get_user_info(user_id).await {
        Ok(r) => R(ApiR::success(r)),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/auth/logout
async fn logout(
    DiComp(_auth): DiComp<AuthAppService>,
) -> R<Empty> {
    match StpUtil::logout_current().await {
        Ok(_) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::fail(format!("登出失败: {}", e)).into_typed()),
    }
}
