use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use thiserror::Error;
use tx_di_core::{ApiRes, AppError};

#[derive(Error, Debug)]
pub enum WebErr {
    #[error("AppError: {0}")]
    AppError(#[from] AppError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for WebErr {
    fn into_response(self) -> Response {
        match self {
            Self::AppError(e) => {
                tracing::warn!("AppError: {:?}", e);
                (StatusCode::OK, Json(ApiRes::from(e)))
            }
            Self::Other(e) => {
                tracing::error!("internal server error:{e:?}");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiRes::fail(e.to_string())))
            }
        }.into_response()
    }
}
