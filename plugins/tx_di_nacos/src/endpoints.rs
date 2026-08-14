//! 静态端点注册表
//!
//! HTTP/gRPC 插件在 `app_async_init` 中将端点注册进来，
//! `NacosClient::register_service` 通过 `take_endpoints` 取走并注册到 Nacos。

use std::sync::{Arc, LazyLock, RwLock};

use crate::model::ServiceEndpoint;
pub use crate::traits::EndpointProvider;

/// 全局端点提供者注册表
///
/// 读多写少（写仅在启动注册一次，读在收集端点时频繁），用 `std::sync::RwLock`：
/// - 注册/收集均为同步调用，临界区极短，无需 tokio 的 async 锁
/// - `RwLock` 允许多读者并发，优于 `Mutex`
static ENDPOINT_PROVIDERS: LazyLock<RwLock<Vec<Arc<dyn EndpointProvider>>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// 注册端点提供者
///
/// HTTP/gRPC 插件在 `app_async_init` 中调用此函数。
pub fn register_endpoints(provider: Arc<dyn EndpointProvider>) {
    if let Ok(mut registry) = ENDPOINT_PROVIDERS.write() {
        registry.push(provider);
    }
}

/// 取走并清空所有已注册的端点
///
/// 每次 App 启动时由 `NacosClient::register_service` 调用一次，
/// 取走后清空，避免跨重启累积重复端点。
pub fn take_endpoints() -> Vec<ServiceEndpoint> {
    let mut all = Vec::new();
    if let Ok(mut registry) = ENDPOINT_PROVIDERS.write() {
        for provider in registry.iter() {
            all.extend(provider.get_endpoints());
        }
        registry.clear();
    }
    all
}
