//! HTTP 协议通用 DTO / 响应包装
//!
//! gRPC 使用 tonic::Status 表达错误，而 HTTP 需要统一的 JSON 包装。

use serde::{Deserialize, Serialize};

/// HTTP 统一响应包装
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { code: 200, msg: "success".into(), data: Some(data) }
    }

    pub fn error(code: i32, msg: impl Into<String>) -> Self {
        Self { code, msg: msg.into(), data: None }
    }
}

/// 分页响应包装
#[derive(Debug, Serialize, Deserialize)]
pub struct PageResponse<T: Serialize> {
    pub list: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}
