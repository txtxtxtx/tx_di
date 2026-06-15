//! 接口统一错误类型
//!
//! 同时支持业务错误（WebErr）和认证错误（SaTokenError），
//! 供 sa-token 宏注解的 handler 使用。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
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
    SaToken(String),
}

impl From<WebErr> for ApiErr {
    fn from(e: WebErr) -> Self {
        Self::Web(e)
    }
}

impl From<sa_token_core::error::SaTokenError> for ApiErr {
    fn from(e: sa_token_core::error::SaTokenError) -> Self {
        Self::SaToken(e.to_string())
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
            Self::SaToken(msg) => {
                tracing::warn!("认证失败: {}", msg);
                (StatusCode::UNAUTHORIZED, Json(ApiRes::error(401, msg))).into_response()
            }
        }
    }
}
