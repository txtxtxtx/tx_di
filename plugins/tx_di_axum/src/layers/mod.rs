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

use std::sync::{Arc, LazyLock, OnceLock, RwLock};
use axum::body::Body;
use axum::http::{Request};
use axum::Router;
use axum::routing::Route;
use tower::Layer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{debug, error, Level};

pub mod api_log;

/// 静态文件路径前缀列表（用于日志过滤）
///
/// 包含所有需要跳过日志记录的路径前缀，例如：
/// - "/static" - 传统静态文件目录
/// - "/app1", "/app2" - SPA 应用路径
static INIT_PATH_PREFIXES: LazyLock<Arc<RwLock<Vec<String>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec!["/static".to_string()])));

/// 添加静态文件路径前缀到过滤列表
pub fn add_static_path_prefix(prefix: impl Into<String>) {
    if let Ok(mut prefixes) = INIT_PATH_PREFIXES.write() {
        let prefix = prefix.into();
        if !prefixes.contains(&prefix) {
            prefixes.push(prefix.clone());
            debug!("已添加静态文件路径前缀: {}", prefix);
        }
    } else {
        error!("无法获取静态文件路径前缀列表的写锁");
    }
}

/// 批量添加静态文件路径前缀
#[allow(unused)]
pub fn add_static_path_prefixes(prefixes: Vec<String>) {
    if let Ok(mut path_list) = INIT_PATH_PREFIXES.write() {
        for prefix in prefixes {
            if !path_list.contains(&prefix) {
                path_list.push(prefix);
            }
        }
        debug!("已批量添加静态文件路径前缀，当前总数: {}", path_list.len());
    } else {
        error!("无法获取静态文件路径前缀列表的写锁");
    }
}

/// 检查路径是否应该被日志过滤
///
/// 如果路径以任何一个静态文件路径前缀开头，则返回 true
#[inline]
pub fn should_filter_path(path: &str) -> bool {
    if let Some(prefixes) = STATIC_PATH_PREFIXES.get() {
        for prefix in prefixes.iter() {
            if path.starts_with(prefix.as_str()) {
                return true;
            }
        }
    }
    false
}


/// 静态文件路径前缀列表（用于日志过滤）
///
/// 使用 OnceLock 实现零成本抽象：
/// - 初始化阶段：通过 RwLock 收集所有路径前缀
/// - 运行阶段：冻结为不可变的 Vec，无锁访问
static STATIC_PATH_PREFIXES: OnceLock<Vec<String>> = OnceLock::new();


/// 获取所有静态文件路径前缀（用于调试）
#[allow(unused)]
pub fn get_static_path_prefixes() -> Vec<String> {
    INIT_PATH_PREFIXES.read().map(|p| p.clone()).unwrap_or_default()
}


/// 冻结静态文件路径前缀
pub fn freeze_static_path_prefixes() {
    if let Ok(prefixes) = INIT_PATH_PREFIXES.read() {
        let frozen = prefixes.clone();
        // 尝试设置，如果已经设置过则忽略
        let _ = STATIC_PATH_PREFIXES.set(frozen);
        debug!("静态文件路径前缀列表已冻结，共 {} 个前缀",
              STATIC_PATH_PREFIXES.get().map(|v| v.len()).unwrap_or(0));
    }
}

/// 超时配置（秒）
static TIMEOUT_SECS: LazyLock<Arc<RwLock<u64>>> =
    LazyLock::new(|| Arc::new(RwLock::new(30)));

/// 设置超时时间（秒）
pub fn set_timeout_secs(secs: u64) {
    if let Ok(mut timeout) = TIMEOUT_SECS.write() {
        *timeout = secs;
        debug!("超时时间已设置为: {} 秒", secs);
    }
}

/// 获取超时时间（秒）
fn get_timeout_secs() -> u64 {
    TIMEOUT_SECS.read().map(|t| *t).unwrap_or(30)
}

/// 动态中间件层 trait
///
/// 允许外部 crate 注册自定义中间件，提供类型擦除的统一接口
pub trait DynMiddleware: Send + Sync {
    /// 应用中间件到 Router
    fn apply_to_router(&self, router: Router) -> Router;
    
    fn name(&self) -> &str;
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
    fn name(&self) -> &str {
        std::any::type_name::<L>()
    }
}

/// 带排序的中间件层
type SortLayer = (i32, Arc<dyn DynMiddleware>);

/// 中间件层注册表
pub static LAYER_REGISTRY: LazyLock<Arc<RwLock<Vec<SortLayer>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// 添加中间件到全局中间件
///
/// layer 实现请参照 [ApiLogLayer](crate::layers::api_log::ApiLogLayer)
#[allow(unused)]
pub fn add_layer<M>(middleware: M, sort: i32)
where
    M: DynMiddleware + 'static,
{
    if let Ok(mut layers) = LAYER_REGISTRY.write() {
        let middleware = Arc::new(middleware);
        layers.push((sort, middleware.clone()));
        debug!("axum 中间件已注册到全局注册表: sort={},name={}", sort, middleware.name());
    } else {
        error!("无法获取中间件注册表的写锁");
    }
}

/// 添加 Arc<dyn DynMiddleware> 到全局中间件,可添加自定义中间件
///
/// layer 实现请参照 [ApiLogLayer](crate::layers::api_log::ApiLogLayer)
pub fn add_arc_layer(middleware: Arc<dyn DynMiddleware>, sort: i32)
{
    if let Ok(mut layers) = LAYER_REGISTRY.write() {
        layers.push((sort, middleware));
        debug!("中间件层已注册到全局注册表: sort={}", sort);
    } else {
        error!("无法获取中间件注册表的写锁");
    }
}

/// 通过名称添加中间件
pub fn add_layer_by_name(name: impl Into<String>, sort: i32){
    if let Some(middleware) = get_layer_by_name(name) {
        add_arc_layer(middleware, sort);
    }
}
/// 通过名称获取中间件
fn get_layer_by_name(name: impl Into<String>) -> Option<Arc<dyn DynMiddleware>> {
    let name = name.into();
    match name.as_str() {
        "api_log" => {
            Some(Arc::new(api_log::ApiLogLayer))
        }
        "cors" => {
            Some(Arc::new(tower_http::cors::CorsLayer::permissive()))
        }
        "trace" => {
            // 创建自定义的 TraceLayer，配置日志输出格式
            let trace_layer = TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(true)
                )
                .on_request(
                    DefaultOnRequest::new().level(Level::INFO)
                )
                .on_response(
                    DefaultOnResponse::new().level(Level::INFO)
                );

            Some(Arc::new(trace_layer))
        }
        "timeout" => {
            use std::time::Duration;
            let timeout_secs = get_timeout_secs();
            Some(Arc::new(tower_http::timeout::TimeoutLayer::with_status_code(
                http::StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(timeout_secs),
            )))        }
        "compression" => {
            Some(Arc::new(tower_http::compression::CompressionLayer::new()))
        }
        _ => {
            error!("无法获取中间件: {}", name);
            None
        }
    }

}