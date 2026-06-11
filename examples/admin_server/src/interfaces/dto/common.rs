//! 通用 DTO — 仅保留 PageQuery（含 keyword），其余类型复用 tx_common

use serde::Deserialize;

/// 分页查询参数（比 tx_common::Page 多了 keyword 搜索字段）
#[derive(Debug, Deserialize)]
pub struct PageQuery {
    #[serde(default = "default_page")] pub page: i64,
    #[serde(default = "default_size")] pub size: i64,
    #[serde(default)] pub keyword: Option<String>,
}
fn default_page() -> i64 { 1 }
fn default_size() -> i64 { 10 }
