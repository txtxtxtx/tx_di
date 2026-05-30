//! 通用 DTO

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self { Self { code: 200, message: "success".to_string(), data: Some(data) } }
    pub fn ok() -> ApiResponse<()> { ApiResponse { code: 200, message: "success".to_string(), data: Some(()) } }
    pub fn error(code: i32, message: impl Into<String>) -> Self { Self { code, message: message.into(), data: None } }
}

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    #[serde(default = "default_page")] pub page: u64,
    #[serde(default = "default_page_size")] pub page_size: u64,
    #[serde(default)] pub keyword: Option<String>,
}
fn default_page() -> u64 { 1 }
fn default_page_size() -> u64 { 10 }

#[derive(Debug, Serialize)]
pub struct PageResponse<T: Serialize> {
    pub list: Vec<T>, pub total: u64, pub page: u64, pub page_size: u64,
}
impl<T: Serialize> PageResponse<T> {
    pub fn new(list: Vec<T>, total: u64, page: u64, page_size: u64) -> Self { Self { list, total, page, page_size } }
}
