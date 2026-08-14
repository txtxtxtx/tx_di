//! 健康检查 API（负载均衡 / 容器编排 / 注册中心探活端点）
//!
//! - `GET /health/live` — 存活探针：进程存活即 200
//! - `GET /health/ready` — 就绪探针：校验数据库连通，异常返回 503
//! - `GET /health` — 综合信息（状态 + DB + 版本）
//!
//! 全部为公开路由（无需认证），供 LB / K8s / r-nacos 等外部探活使用。

use axum::Json;
use axum::http::StatusCode;
use serde_json::{Value, json};
use tx_di_axum::Router;
use tx_di_axum::bound::DiComp;
use tx_di_toasty::ToastyPlugin;

use admin_infra::user::model::SysUser;

/// 公开路由（无需认证）
pub fn open_router() -> Router {
    use axum::routing::get;
    Router::new()
        .route("/health", get(health))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
}

/// 对数据库执行一次轻量查询以验证连通性
async fn db_healthy(toasty: &ToastyPlugin) -> bool {
    match toasty.try_db() {
        Some(db) => {
            let mut db = db.clone();
            SysUser::all().limit(1).exec(&mut db).await.is_ok()
        }
        None => false,
    }
}

/// GET /health/live — 存活探针
async fn health_live() -> Json<Value> {
    Json(json!({ "status": "alive" }))
}

/// GET /health/ready — 就绪探针（校验数据库）
async fn health_ready(DiComp(toasty): DiComp<ToastyPlugin>) -> (StatusCode, Json<Value>) {
    if db_healthy(&toasty).await {
        (
            StatusCode::OK,
            Json(json!({ "status": "ready", "db": "ok" })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "unavailable", "db": "error" })),
        )
    }
}

/// GET /health — 综合健康信息
async fn health(DiComp(toasty): DiComp<ToastyPlugin>) -> Json<Value> {
    let db_ok = db_healthy(&toasty).await;
    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "db": if db_ok { "ok" } else { "error" },
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
