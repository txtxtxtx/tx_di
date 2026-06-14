//! 系统工具 HTTP API（示例 mock 数据）

use tx_di_axum::{R, Router};
use tx_di_axum::aide::axum::routing::get;
use admin_proto::CacheStatsResponse;
use tx_common::ApiR;

pub fn router() -> Router {
    Router::new()
        .api_route("/cache/stats", get(get_cache_stats))
}

/// GET /api/tool/cache/stats - 获取缓存统计
async fn get_cache_stats() -> R<CacheStatsResponse> {
    R(ApiR::success(CacheStatsResponse {
        total_keys: 1024,
        used_memory: 67_108_864,  // 64 MB
        hit_count: 8500,
        miss_count: 1500,
        hit_rate: 85.0,
    }))
}
