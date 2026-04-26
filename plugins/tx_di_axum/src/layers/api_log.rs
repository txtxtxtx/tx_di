use axum::{
    body::{Body, to_bytes},
    http::{Request, Response, header},
};
use std::future::Future;
use std::pin::Pin;
use tokio::time::Instant;
use tower::{Layer, Service};
use tracing::{info, warn, Span};


/// API 日志中间件 Layer
/// 
/// 用于记录 HTTP 请求和响应的详细信息，包括：
/// - 请求方法、URI、查询参数
/// - 请求体（仅 JSON 和文本类型）
/// - 响应状态码、响应体（仅 JSON 和文本类型）
#[derive(Clone)]
pub struct ApiLogLayer;

impl<S> Layer<S> for ApiLogLayer {
    type Service = ApiLogMiddleware<S>;

    /// 将中间件应用到内部服务上
    fn layer(&self, inner: S) -> Self::Service {
        ApiLogMiddleware { inner }
    }
}

/// API 日志中间件
/// 
/// 包装内部服务，在请求处理前后记录日志信息
#[derive(Clone)]
pub struct ApiLogMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for ApiLogMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    /// 检查服务是否准备好接收请求
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    /// 处理请求并记录日志
    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();

        // 提取请求基本信息
        let method = req.method().clone();
        let uri = req.uri().clone();
        let query = req.uri().query().unwrap_or("").to_string();

        // 提取 Content-Type 头部
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        Box::pin(async move {
            // 记录请求开始时间
            let start_time = Instant::now();

            // 判断是否需要读取请求体内容（仅文本和 JSON 类型）
            let need_log_request_body = should_log_body(&content_type);
            
            let (parts, body) = req.into_parts();
            
            let request_body;
            let rebuilt_req;
            
            if need_log_request_body {
                // 仅在需要记录时才读取并消耗 body
                let body_bytes = to_bytes(body, usize::MAX).await.unwrap_or_default();
                request_body = String::from_utf8_lossy(&body_bytes).to_string();
                // 重建请求，将读取的字节重新放入 body 中
                rebuilt_req = Request::from_parts(parts, Body::from(body_bytes));
            } else {
                // 不需要记录具体内容时，直接使用原始 body，避免不必要的内存消耗
                // 对于流式数据或大文件，这样性能更好
                request_body = "<streaming or binary data>".to_string();
                rebuilt_req = Request::from_parts(parts, body);
            }

            // 获取当前 span 并记录请求日志
            // 如果存在 TraceLayer 创建的 span，日志会自动关联到该 span
            let current_span = Span::current();
            let _guard = current_span.enter();
            info!(
                method = %method,
                uri = %uri,
                query = %query,
                content_type = %content_type,
                body = %request_body,
                "REQ "
            );

            // 调用内部服务处理请求（传递原始或重建的 body）
            let response = inner.call(rebuilt_req).await?;

            // 先分离响应的头部和身体，避免借用冲突
            let (response_parts, response_body) = response.into_parts();
            
            // 从已分离的头部中提取 Content-Type（转换为 owned String 避免借用问题）
            let response_content_type = response_parts
                .headers
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            
            // 判断是否需要读取响应体内容（仅文本和 JSON 类型）
            let need_log_response_body = should_log_body(&response_content_type);
            
            let response_body_str;
            let final_response;
            
            if need_log_response_body {
                // 仅在需要记录时才读取并消耗响应 body
                let response_bytes = to_bytes(response_body, usize::MAX).await.unwrap_or_default();
                response_body_str = String::from_utf8_lossy(&response_bytes).to_string();
                
                // 重建响应
                final_response = Response::from_parts(response_parts, Body::from(response_bytes));
            } else {
                // 不需要记录具体内容时，直接使用原始 body，保持流式特性
                response_body_str = "<streaming or binary data>".to_string();
                final_response = Response::from_parts(response_parts, response_body);
            }

            let status = final_response.status();
            // 计算请求耗时
            let elapsed = start_time.elapsed();
            let elapsed_nanos = elapsed.as_nanos();

            // 根据耗时大小选择合适的单位
            let latency_str = if elapsed_nanos >= 1_000_000 {
                // 大于等于 1,000,000 纳秒，使用毫秒
                format!("{:.2}ms", elapsed_nanos as f64 / 1_000_000.0)
            } else if elapsed_nanos >= 1_000 {
                // 大于等于 1,000 纳秒，使用微秒
                format!("{:.2}μs", elapsed_nanos as f64 / 1_000.0)
            } else {
                // 小于 1,000 纳秒，使用纳秒
                format!("{}ns", elapsed_nanos)
            };
            // 在同一个 span 中记录响应日志
            let current_span = Span::current();
            let _guard = current_span.enter();
            // 根据响应状态码选择日志级别：成功用 info，失败用 warn
            if status.is_success() {
                info!(
                    status = %status,
                    latency = latency_str,
                    content_type = %response_content_type,
                    body = %response_body_str,
                    "RESP"
                );
            } else {
                warn!(
                    status = %status,
                    latency = latency_str,
                    content_type = %response_content_type,
                    body = %response_body_str,
                    "RESP"
                );
            }
            Ok(final_response)
        })
    }
}

/// 判断是否应该记录完整的 body 内容
/// 
/// 仅对以下 Content-Type 记录完整内容：
/// - application/json
/// - text/* (如 text/plain, text/html 等)
/// - application/xml
/// 
/// 其他类型（如图片、二进制文件等）只记录大小和类型，避免日志过大
fn should_log_body(content_type: &str) -> bool {
    content_type.contains("application/json")
        || content_type.contains("text/")
        || content_type.contains("application/xml")
}

