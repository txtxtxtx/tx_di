use std::ops::Deref;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use tx_di_core::ApiR;


/// A wrapper around [`ApiR`]
pub struct R<T>(pub ApiR<T>);

impl<T> Deref for R<T> {
    type Target = ApiR<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<ApiR<T>> for R<T> {
    fn from(value: ApiR<T>) -> Self {
        R(value)
    }
}

impl<T: serde::Serialize> IntoResponse for R<T> {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, Json(self.0)).into_response()
    }
}