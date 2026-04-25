//! 请求进入:
//!  1. CORS (跨域)
//!  2. Trace/Logging (请求追踪)
//!  3. Timeout (超时控制)
//!  4. Compression (压缩)
//!  5. Body Limit (请求体大小限制)
//!  6. Authentication/Authorization (认证授权)
//!  7. Rate Limiting (限流)
//!  8. Request ID (请求ID生成)
//!  9. Custom Business Logic (自定义业务逻辑)
//!
//! → Handler
//!
//! 响应相反

use std::sync::{Arc, LazyLock, RwLock};
use axum::body::Body;
use axum::http::{Request, Response};
use axum::Router;
use axum::routing::Route;
use tower::Layer;
use tracing::{error, info};

pub mod api_log;

/// 动态中间件层 trait
///
/// 允许外部 crate 注册自定义中间件，提供类型擦除的统一接口
pub trait DynMiddleware: Send + Sync {
    /// 应用中间件到 Router
    fn apply_to_router(&self, router: Router) -> Router;
}

/// 为任何满足条件的 Layer 自动实现 DynMiddleware
///
/// 支持所有可以应用到 axum::routing::Route 上的 Layer，
/// 包括 tower-http 提供的各种中间件（TraceLayer、CorsLayer、CompressionLayer 等）
impl<L> DynMiddleware for L
where
// 条件1: L 必须是一个能作用于 Route 的 Layer，且可克隆、线程安全、生命周期足够长
    L: Layer<Route> + Clone + Send + Sync + 'static,
// 条件2: Layer 处理后产生的 Service 必须是 tower 的服务
    <L as Layer<Route>>::Service: tower::Service<Request<Body>> + Clone + Send + Sync + 'static,
    <<L as Layer<Route>>::Service as tower::Service<Request<Body>>>::Future: Send + 'static,
    <<L as Layer<Route>>::Service as tower::Service<Request<Body>>>::Response: axum::response::IntoResponse + 'static,
    <<L as Layer<Route>>::Service as tower::Service<Request<Body>>>::Error: Into<std::convert::Infallible> + 'static,
{
    fn apply_to_router(&self, router: Router) -> Router {
        router.layer(self.clone())
    }
}

/// 带排序的中间件层
type SortLayer = (i32, Arc<dyn DynMiddleware>);

/// 中间件层注册表
pub static LAYER_REGISTRY: LazyLock<Arc<RwLock<Vec<SortLayer>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// 添加中间件到全局中间件
pub fn add_layer<M>(middleware: M, sort: i32)
where
    M: DynMiddleware + 'static,
{
    if let Ok(mut layers) = LAYER_REGISTRY.write() {
        let middleware = Arc::new(middleware);
        layers.push((sort, middleware));
        info!("中间件层已注册到全局注册表: sort={}", sort);
    } else {
        error!("无法获取中间件注册表的写锁");
    }
}

pub fn add_arc_layer(middleware: Arc<dyn DynMiddleware>, sort: i32)
{
    if let Ok(mut layers) = LAYER_REGISTRY.write() {
        layers.push((sort, middleware));
        info!("中间件层已注册到全局注册表: sort={}", sort);
    } else {
        error!("无法获取中间件注册表的写锁");
    }
}
pub fn add_layer_by_name(name: impl Into<String>, sort: i32){
    if let Some(middleware) = get_layer_by_name(name) {
        add_arc_layer(middleware, sort);
    }
}
/// 通过名称获取中间件
pub fn get_layer_by_name(name: impl Into<String>) -> Option<Arc<dyn DynMiddleware>> {
    let name = name.into();
    match name.as_str() {
        "api_log" => {
            Some(Arc::new(api_log::ApiLogLayer))
        }
        "cors" => {
            Some(Arc::new(tower_http::cors::CorsLayer::permissive()))
        }
        "trace" => {
            Some(Arc::new(tower_http::trace::TraceLayer::new_for_http()))
        }
        "timeout" => {
            use std::time::Duration;
            Some(Arc::new(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(30))))
        }
        "compression" => {
            Some(Arc::new(tower_http::compression::CompressionLayer::new()))
        }
        _ => {
            error!("无法获取中间件: {}", name);
            None
        }
    }

}