//! 请求体大小限制层（基于 Content-Length 提前拦截）
//!
//! 在 body 被消费**之前**检查 `Content-Length` 请求头，
//! 超限立即返回 413 Payload Too Large，避免接收无用字节。
//!
//! 配合上层的 `DefaultBodyLimit`（流式读 body 检查）形成双层防护：
//! - 本层：零字节开销，提前拒绝明显超大的请求
//! - `DefaultBodyLimit`：兜底，处理无 Content-Length 或谎报大小的请求

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Json;
use axum::response::{IntoResponse, Response};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tx_common::ApiRes;
use tracing::{ warn};

/// 请求体大小限制层
///
/// 构造时指定 `max_bytes`、`error_code`、`error_msg`，
/// 通过 `.route_layer()` 应用到特定路由（如文件上传）。
#[derive(Clone)]
pub struct BodySizeLimitLayer {
    max_bytes: u64,
    error_code: i32,
    error_msg: String,
}

impl BodySizeLimitLayer {
    /// 创建新实例
    ///
    /// `max_bytes == 0` 表示不限制（直接放行）
    pub fn new(max_bytes: u64, error_code: i32, error_msg: impl Into<String>) -> Self {
        Self {
            max_bytes,
            error_code,
            error_msg: error_msg.into(),
        }
    }
}

impl<S> Layer<S> for BodySizeLimitLayer {
    type Service = BodySizeLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BodySizeLimitMiddleware {
            inner,
            max_bytes: self.max_bytes,
            error_code: self.error_code,
            error_msg: self.error_msg.clone(),
        }
    }
}

/// 中间件 Service 实现
#[derive(Clone)]
pub struct BodySizeLimitMiddleware<S> {
    inner: S,
    max_bytes: u64,
    error_code: i32,
    error_msg: String,
}

impl<S> Service<Request<Body>> for BodySizeLimitMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // ── 仅 Content-Length 明确超限时提前拒绝 ──
        if self.max_bytes > 0 {
            if let Some(cl) = req
                .headers()
                .get(header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
            {
                if cl > self.max_bytes {
                    warn!(
                        "BodySizeLimit: 拒绝请求 (Content-Length={cl} > max={})",
                        self.max_bytes
                    );
                    let resp = (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(ApiRes::error(self.error_code, self.error_msg.clone())),
                    )
                        .into_response();
                    return Box::pin(async move { Ok(resp) });
                }
            }
        }

        // ── 放行（无 Content-Length 或未超限） ──
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move { inner.call(req).await })
    }
}
