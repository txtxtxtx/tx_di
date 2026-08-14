//! Prometheus 指标端点（阶段 E-3：可观测性）
//!
//! 提供 `/metrics` 端点（Prometheus 文本格式）与全局 HTTP 请求指标采集：
//! - `http_requests_total{method,path,status}`：请求计数
//! - `http_request_duration_seconds{method,path}`：请求耗时直方图
//! - `process_uptime_seconds`：进程运行时长
//!
//! 通过 `MetricsLayer` 挂到 axum 路由（如 admin_api 的 `app_async_init` 中
//! `tx_di_axum::add_layer(MetricsLayer, 5)`），并由 `metrics_router()` 暴露端点。

use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use axum::http::Request;
use axum::response::IntoResponse;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};

/// 全局 Prometheus 注册表
static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

/// HTTP 请求计数：method + 归一化路由路径
static HTTP_REQUESTS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    IntCounterVec::new(
        Opts::new("http_requests_total", "Total HTTP requests processed").namespace("tx_di"),
        &["method", "path"],
    )
    .expect("http_requests_total metric created")
});

/// HTTP 请求耗时直方图
static HTTP_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    HistogramVec::new(
        HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
        )
        .namespace("tx_di")
        .buckets(vec![
            0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ]),
        &["method", "path"],
    )
    .expect("http_request_duration_seconds metric created")
});

/// 进程启动时间（unix epoch seconds）
static PROCESS_START: AtomicU64 = AtomicU64::new(0);

/// 启动时记录进程启动时间（幂等）
pub fn init() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    PROCESS_START.store(now, Ordering::SeqCst);

    // 注册指标到全局注册表（幂等：已注册则跳过）
    for collector in [
        Box::new(HTTP_REQUESTS.clone()) as Box<dyn prometheus::core::Collector>,
        Box::new(HTTP_DURATION.clone()),
    ] {
        let _ = REGISTRY.register(collector);
    }
}

/// 获取当前进程 uptime（秒）
pub fn uptime_secs() -> u64 {
    let start = PROCESS_START.load(Ordering::SeqCst);
    if start == 0 {
        return 0;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(start)
}

/// 请求归一化路径：将 `/api/user/123` → `/api/user/{id}` 避免高基数
fn normalize_path(path: &str) -> String {
    // 简单规则：路径中连续数字段替换为 {id}
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        if !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()) {
            parts.push("{id}");
        } else {
            parts.push(seg);
        }
    }
    parts.join("/")
}

/// Prometheus 指标采集层
///
/// 挂到 axum 路由后，为每个请求记录计数与耗时。
/// 用法：`tx_di_axum::add_layer(tx_di_axum::MetricsLayer, 5)`。
#[derive(Debug, Clone, Default)]
pub struct MetricsLayer;

impl<S> tower::Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        MetricsService { inner }
    }
}

/// 指标采集服务
#[derive(Debug, Clone)]
pub struct MetricsService<S> {
    inner: S,
}

impl<S, B> tower::Service<Request<B>> for MetricsService<S>
where
    S: tower::Service<Request<B>> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: Send,
    S::Error: std::fmt::Display,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let method = req.method().as_str().to_string();
        let path = normalize_path(req.uri().path());
        let start = Instant::now();

        // 直接调用 inner（不借用 &mut self 跨 await），future 满足 Send + 'static
        let fut = self.inner.call(req);

        Box::pin(async move {
            let result = fut.await;
            HTTP_REQUESTS.with_label_values(&[&method, &path]).inc();
            HTTP_DURATION
                .with_label_values(&[&method, &path])
                .observe(start.elapsed().as_secs_f64());
            result
        })
    }
}

/// 构建 `/metrics` 端点路由（挂在 WebPlugin 内置路由）
pub fn metrics_router() -> axum::Router {
    use axum::routing::get;
    axum::Router::new().route("/metrics", get(metrics_handler))
}

/// GET /metrics — Prometheus 文本格式指标
async fn metrics_handler() -> impl IntoResponse {
    // 动态添加 uptime 指标（每次采集时计算）
    let uptime = prometheus::IntGauge::with_opts(Opts::new(
        "process_uptime_seconds",
        "Process uptime in seconds",
    ))
    .expect("uptime metric created");
    uptime.set(uptime_secs() as i64);
    let _ = REGISTRY.register(Box::new(uptime));

    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(()) => {
            let body = String::from_utf8_lossy(&buffer).to_string();
            ([("content-type", "text/plain; version=0.0.4")], body).into_response()
        }
        Err(e) => {
            tracing::error!("Prometheus 指标编码失败: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "metrics encoding error",
            )
                .into_response()
        }
    }
}

/// 供其他 crate 使用：注册额外指标（预留）
pub fn register_collector<C: prometheus::core::Collector + 'static>(collector: C) {
    let _ = REGISTRY.register(Box::new(collector));
}

/// 便捷：确认 metrics 模块就绪（在 app_async_init 调用）
pub fn ensure_initialized() {
    init();
}
