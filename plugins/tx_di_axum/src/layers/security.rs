//! 安全响应头中间件
//!
//! 为所有 HTTP 响应附加基础安全响应头，缓解常见 Web 攻击面：
//! - `X-Content-Type-Options: nosniff` — 禁止 MIME 嗅探
//! - `X-Frame-Options: DENY` — 禁止页面被 iframe 嵌入（防点击劫持）
//! - `X-XSS-Protection: 0` — 关闭浏览器旧式 XSS 过滤器（现代浏览器已用 CSP）
//! - `Referrer-Policy: no-referrer` — 不泄漏来源
//! - `Content-Security-Policy` — 默认严格 CSP（可被后续更精细策略覆盖）
//!
//! 通过 `tx_di_axum::add_layer(SecurityHeadersLayer, 优先级)` 或
//! 在 `web_config.layers` 中配置 `["security"]` 启用。

use axum::{
    body::Body,
    http::{Request, Response},
};
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// 安全响应头 Layer
#[derive(Debug, Clone, Default)]
pub struct SecurityHeadersLayer;

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersMiddleware { inner }
    }
}

/// 安全响应头中间件
#[derive(Debug, Clone)]
pub struct SecurityHeadersMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for SecurityHeadersMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let mut response = inner.call(req).await?;
            apply_security_headers(response.headers_mut());
            Ok(response)
        })
    }
}

/// 为响应头应用基础安全策略
pub fn apply_security_headers(headers: &mut axum::http::HeaderMap) {
    insert_if_absent(
        headers,
        axum::http::HeaderName::from_static("x-content-type-options"),
        "nosniff",
    );
    insert_if_absent(
        headers,
        axum::http::HeaderName::from_static("x-frame-options"),
        "DENY",
    );
    insert_if_absent(
        headers,
        axum::http::HeaderName::from_static("x-xss-protection"),
        "0",
    );
    insert_if_absent(
        headers,
        axum::http::HeaderName::from_static("referrer-policy"),
        "no-referrer",
    );
    insert_if_absent(
        headers,
        axum::http::HeaderName::from_static("content-security-policy"),
        "default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; script-src 'self'",
    );
}

/// 仅在响应头不存在时插入，避免覆盖业务自定义值
fn insert_if_absent(
    headers: &mut axum::http::HeaderMap,
    name: axum::http::HeaderName,
    value: &'static str,
) {
    if !headers.contains_key(&name)
        && let Ok(v) = axum::http::HeaderValue::from_str(value)
    {
        headers.insert(name, v);
    }
}
