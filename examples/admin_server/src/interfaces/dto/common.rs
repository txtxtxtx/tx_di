//! 通用 DTO

use serde::{Deserialize, Serialize};

/// 统一 API 响应格式
///
/// ```json
/// { "code": 200, "message": "success", "data": {} }
/// ```
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 成功响应
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            message: "success".to_string(),
            data: Some(data),
        }
    }

    /// 成功响应（无数据）
    pub fn ok() -> ApiResponse<()> {
        ApiResponse {
            code: 200,
            message: "success".to_string(),
            data: Some(()),
        }
    }

    /// 错误响应
    pub fn error(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

/// 分页查询参数
#[derive(Debug, Deserialize)]
pub struct PageQuery {
    /// 页码（从 1 开始）
    #[serde(default = "default_page")]
    pub page: u64,
    /// 每页条数
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    /// 搜索关键词
    #[serde(default)]
    pub keyword: Option<String>,
}

fn default_page() -> u64 { 1 }
fn default_page_size() -> u64 { 10 }

/// 分页响应
#[derive(Debug, Serialize)]
pub struct PageResponse<T: Serialize> {
    /// 数据列表
    pub list: Vec<T>,
    /// 总记录数
    pub total: u64,
    /// 当前页码
    pub page: u64,
    /// 每页条数
    pub page_size: u64,
}

impl<T: Serialize> PageResponse<T> {
    pub fn new(list: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        Self { list, total, page, page_size }
    }
}
