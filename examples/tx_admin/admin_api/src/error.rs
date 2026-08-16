//! 接口统一错误类型
//!
//! 同时支持业务错误（WebErr）和认证错误（SaTokenError），
//! 供 sa-token 宏注解的 handler 使用。

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sa_token_core::error::SaTokenError;
use tx_common::ApiRes;
use tx_di_axum::e::WebErr;

/// 接口错误类型
///
/// handler 返回 `Result<R<T>, ApiErr>`，sa-token 宏和业务逻辑都能通过 `?` 传播错误。
#[derive(Debug)]
pub enum ApiErr {
    /// 业务错误
    Web(WebErr),
    /// sa-token 认证/权限错误
    SaToken(SaTokenError),
}

impl From<WebErr> for ApiErr {
    fn from(e: WebErr) -> Self {
        Self::Web(e)
    }
}

impl From<sa_token_core::error::SaTokenError> for ApiErr {
    fn from(e: sa_token_core::error::SaTokenError) -> Self {
        Self::SaToken(e)
    }
}

impl From<tx_error::AppError> for ApiErr {
    fn from(e: tx_error::AppError) -> Self {
        Self::Web(WebErr::AppError(e))
    }
}

impl From<anyhow::Error> for ApiErr {
    fn from(e: anyhow::Error) -> Self {
        Self::Web(WebErr::Other(e))
    }
}

impl IntoResponse for ApiErr {
    fn into_response(self) -> Response {
        match self {
            Self::Web(e) => e.into_response(),
            Self::SaToken(e) => {
                // 授权错误（权限/角色不足）→ HTTP 403，业务码 403
                // 认证错误（未登录/Token 无效/过期）→ HTTP 401，业务码 401
                match &e {
                    SaTokenError::PermissionDenied
                    | SaTokenError::PermissionDeniedDetail(_)
                    | SaTokenError::RoleDenied(_) => {
                        let msg = match &e {
                            SaTokenError::PermissionDeniedDetail(perm) => {
                                format!("没有操作权限: {perm}")
                            }
                            SaTokenError::RoleDenied(role) => {
                                format!("没有角色权限: {role}")
                            }
                            _ => "没有操作权限".to_string(),
                        };
                        tracing::warn!("权限不足: {e}");
                        (StatusCode::FORBIDDEN, Json(ApiRes::error(403, msg))).into_response()
                    }
                    _ => {
                        tracing::warn!("认证失败: {e}");
                        (StatusCode::UNAUTHORIZED, Json(ApiRes::error(401, e.to_string())))
                            .into_response()
                    }
                }
            }
        }
    }
}
