//! axum IntoResponse 实现

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::AppError;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorBody {
            code: i32,
            message: String,
        }

        let body = ErrorBody {
            code: self.code(),
            message: self.full_message(),
        };

        (StatusCode::OK, Json(body)).into_response()
    }
}
