//! OpenTelemetry 链路追踪初始化（阶段 E-3）
//!
//! 在 `[log_config].otlp_endpoint` 配置后启用：构建 OTLP HTTP 导出器（Protobuf），
//! 挂载为 tracing subscriber 的 OpenTelemetryLayer，实现全链路 Trace 导出
//! （W3C TraceContext 传播，可与 Jaeger / OTel Collector / 云厂商对接）。

use std::sync::OnceLock;

use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::{SdkTracerProvider, Tracer};
use tx_di_core::RIE;

/// OTel TracerProvider 全局持有（避免 drop 时立即关闭导出器）
static OTEL_PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

/// 是否已初始化
pub fn is_initialized() -> bool {
    OTEL_PROVIDER.get().is_some()
}

/// 初始化 OTel 链路追踪，返回 `opentelemetry_sdk::trace::Tracer`
///
/// 调用方通过 `tracing_opentelemetry::layer().with_tracer(tracer)` 挂载到 tracing。
///
/// # Arguments
/// * `endpoint` - OTLP HTTP 端点（如 `http://127.0.0.1:4318`）
/// * `service_name` - 上报的服务名
pub fn init_tracer(endpoint: &str, service_name: &str) -> RIE<Tracer> {
    use opentelemetry_otlp::WithExportConfig as _;

    // 1. 设置全局 TraceContext 传播器（W3C 标准，跨服务透传 traceparent）
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    // 2. 构建 OTLP SpanExporter（HTTP + Protobuf，blocking 客户端后台导出）
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| anyhow::anyhow!("OTel SpanExporter 构建失败: {e}"))?;

    // 3. 构建 TracerProvider（batch exporter：后台批量导出，性能友好）
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();

    // 4. 进程级持有 provider，防止 drop 后导出器被关闭
    let _ = OTEL_PROVIDER.set(provider.clone());

    let tracer = provider.tracer(service_name.to_string());
    tracing::info!(
        endpoint = %endpoint,
        service = %service_name,
        "OpenTelemetry 链路追踪已启用 (OTLP HTTP)"
    );

    Ok(tracer)
}
