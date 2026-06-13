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

/// WebErr 作为 extractor rejection 类型，手动实现 JsonSchema
/// 实际错误响应不体现在 API 文档中，使用空 schema 占位
#[cfg(feature = "api-doc")]
impl schemars::JsonSchema for WebErr {
    fn schema_name() -> String {
        "WebErr".to_string()
    }
    fn json_schema(_gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Object(Default::default())
    }
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
