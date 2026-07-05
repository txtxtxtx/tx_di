//! 静态端点注册表
//!
//! 类似于 `ROUTER_REGISTRY` 模式，HTTP/gRPC 插件在初始化时将自己的端点注册进来，
//! `RegistryPlugin` 收集所有端点后统一注册到 Nacos。

use std::sync::{Arc, LazyLock, Mutex};

pub use crate::traits::EndpointProvider;
use crate::model::ServiceEndpoint;

/// 全局端点提供者注册表
static ENDPOINT_PROVIDERS: LazyLock<Mutex<Vec<Arc<dyn EndpointProvider>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// 注册端点提供者
///
/// HTTP/gRPC 插件在 `app_async_init` 中调用此函数。
pub fn register_endpoints(provider: Arc<dyn EndpointProvider>) {
    if let Ok(mut registry) = ENDPOINT_PROVIDERS.lock() {
        registry.push(provider);
    }
}

/// 收集所有已注册的端点
pub fn collect_endpoints() -> Vec<ServiceEndpoint> {
    let mut all = Vec::new();
    if let Ok(registry) = ENDPOINT_PROVIDERS.lock() {
        for provider in registry.iter() {
            all.extend(provider.get_endpoints());
        }
    }
    all
}
