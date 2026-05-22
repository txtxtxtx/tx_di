use std::ops::Deref;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use tx_di_core::{ApiR, ApiRes};


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

impl<T> R<T> {
    /// 成功响应
    pub fn ok(data: T) -> Self {
        R(ApiR::success(data))
    }

    /// 失败响应（code = -1）
    pub fn fail(msg: String) -> Self {
        R(ApiRes::fail(msg).into_typed())
    }

    /// 错误响应（自定义 code）
    pub fn error(code: i32, msg: String) -> Self {
        R(ApiRes::error(code, msg).into_typed())
    }
}

impl<T: serde::Serialize> IntoResponse for R<T> {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, Json(self.0)).into_response()
    }
}