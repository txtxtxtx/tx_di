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
                // 不暴露原始错误链给前端，使用统一内部错误码
                let app_err = AppError::from_anyhow(e);
                (StatusCode::OK, Json(ApiRes::error(app_err.code(), app_err.message().to_string())))
            }
        }.into_response()
    }
}
