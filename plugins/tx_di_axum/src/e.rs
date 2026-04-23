use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use thiserror::Error;
use tx_di_core::{ApiRes, IE};

#[derive(Error, Debug)]
pub enum WebErr{
    /// 系统错误
    #[error("IE: {0}")]
    IE(#[from] IE),
    /// 其他错误
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for WebErr {
    fn into_response(self) -> Response {
        match self {
            Self::IE(e) => {
                tracing::warn!("IE: {:?}", e);
                (StatusCode::OK,  Json(ApiRes::from(e)))
            }
            Self::Other(e) => {
                tracing::error!("internal server error:{e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiRes::fail(e.to_string()))
                )
            }
        }
            .into_response()
    }
}