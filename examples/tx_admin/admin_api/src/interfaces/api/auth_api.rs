//! 认证 HTTP API

use admin_domain::shared::model::value_object::{SessionEctData};
use axum::Json;
use tx_di_axum::Router;
use tx_di_axum::bound::DiComp;
use tx_di_sa_token::{StpUtil, LoginIdExtractor, sa_check_permission};
use admin_app::auth::app_service::AuthAppService;
use admin_proto::{LoginRequest, LoginResponse, UserInfoResponse, LogoutRequest, Empty};
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
) -> Result<ApiR<LoginResponse>, ApiErr> {
    let login_ip = req.login_ip.clone();
    let mut r = auth.login(req).await?;
    let token = StpUtil::login(r.user_id.to_string()).await?;
    r.token = token.to_string();
    // 将登录 IP 存入 token 的 extra_data，供在线用户查询使用
    let extra = serde_json::json!(SessionEctData{
        login_ip,
        tenant_id: r.tenant_id.into(),
        role_ids: r.role_ids.clone(),
        dept_ids: r.dept_ids.clone()
    });
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
    let mut resp = ApiR::success(r);
    resp.msg = token.to_string();
    Ok(resp)
}

/// GET /api/auth/user-info
#[sa_check_permission("auth:info")]
async fn user_info(
    DiComp(auth): DiComp<AuthAppService>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> Result<ApiR<UserInfoResponse>, ApiErr> {
    let user_id: u64 = login_id.parse().unwrap_or(0);
    let r = auth.get_user_info(user_id).await?;
    Ok(ApiR::success(r))
}

/// POST /api/auth/logout
#[sa_check_permission("auth:logout")]
async fn logout(
    DiComp(auth): DiComp<AuthAppService>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> Result<ApiR<Empty>, ApiErr> {
    let user_id: u64 = login_id.parse().unwrap_or(0);
    // 1. 使 sa-token 会话失效
    StpUtil::logout_current().await?;
    // 2. 清除 sa-token 中该用户的权限和角色缓存
    StpUtil::clear_permissions(&login_id).await?;
    StpUtil::clear_roles(&login_id).await?;
    // 3. 记录登出日志
    let _ = auth.logout(LogoutRequest { user_id }).await;
    Ok(ApiRes::ok().into_typed())
}
