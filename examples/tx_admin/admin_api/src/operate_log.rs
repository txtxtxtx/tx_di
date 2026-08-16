//! 操作日志模块：为 HTTP 请求提供操作日志记录能力。
//!
//! 有界 channel 容量，缓冲 4096 条日志条目。
//! 当 channel 满时，新产生的日志会被丢弃并输出 warn 级别告警，
//! 以确保 HTTP 响应不被阻塞。
//!
//! - [`OperateLogEntry`]：HTTP 请求的操作日志条目，由中间件在请求完成后通过有界 channel 发送给消费者。
//!
//!   # Fields
//!
//!   - `method` — HTTP 方法（如 `"GET"`、`"POST"`）
//!   - `uri` — 请求 URI
//!   - `status` — HTTP 响应状态码
//!   - `latency_ms` — 请求耗时（毫秒）
//!   - `user_ip` — 客户端 IP，优先取 `x-forwarded-for`，其次 `x-real-ip`
//!   - `user_agent` — User-Agent 头部值
//!
//! - [`OperateLogLayer`]：操作日志 Layer，将 HTTP 请求元数据通过有界 channel 异步发送给消费者。
//!
//!   包装 `axum::routing::Route`，提取每次请求的方法、URI、状态码、耗时、IP、UA，
//!   请求完成后通过有界 [`mpsc::Sender`] 发送 [`OperateLogEntry`]。
//!   channel 满时丢弃日志并输出 warn，绝不阻塞 HTTP 响应。
//!
//!   通过 [`tx_di_axum::add_layer`] 注册到全局中间件链，建议 sort 值为 15
//!   （紧接 `api_log(10)` 之后、压缩层 `100` 之前）。
//!
//!   # Examples
//!
//!   在 `plugin.rs` 中注册：
//!
//!   ```ignore
//!   let (tx, rx) = mpsc::channel::<OperateLogEntry>(OPERATE_LOG_CHANNEL_CAP);
//!   let layer = OperateLogLayer::new(tx);
//!   add_layer(layer, 15);
//!   ```
//!
//!   # Panics
//!
//!   不会 panic；channel 满时丢弃日志而非阻塞。
//!
//! - [`OperateLogLayer::new`]：创建操作日志 Layer，传入有界 channel 的发送端。不会 panic。
//!
//! - [`OperateLogMiddleware`]：操作日志中间件，在 HTTP 请求完成后提取元数据并通过有界 channel 发送。
//!
//!   内部委托 `axum::routing::Route` 处理请求，响应完成后构造 [`OperateLogEntry`]
//!   并调用 [`mpsc::Sender::try_send`] 发送。发送失败时：
//!
//!   - **Full** — channel 已满，输出 warn 并丢弃该条日志
//!   - **Closed** — 消费者已退出，静默忽略
//!
//!   # Errors
//!
//!   本服务永远不会返回错误（`type Error = Infallible`）。
use admin_domain::shared::model::value_object::SessionEctData;
use axum::{
    body::Body,
    http::{Request, Response, header},
};
use sa_token_core::token::TokenValue;
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tower::{Layer, Service};
use tracing::{debug, warn};
use tx_common::id;
use tx_di_sa_token::StpUtil;

/// 有界 channel 容量：缓冲 65536 条日志。
/// 满时生产者最多等待 `OPERATE_LOG_SEND_TIMEOUT`（见 send 逻辑），
/// 超时是最后兜底（消费者过慢的极端情况），正常情况下不丢日志。
pub const OPERATE_LOG_CHANNEL_CAP: usize = 65536;

/// 操作日志发送最大等待时间：channel 满时在此时间内等待消费者消化
const OPERATE_LOG_SEND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

/// 操作日志条目，在请求完成后通过 channel 发送给消费者
#[derive(Debug, Clone)]
pub struct OperateLogEntry {
    /// 链路追踪 ID（中间件生成）
    pub trace_id: String,
    pub method: String,
    pub uri: String,
    pub status: u16,
    pub latency_ms: f64,
    pub user_ip: String,
    pub user_agent: String,
    /// 登录用户 ID（从 SaToken 会话提取，未登录则为 None）
    pub user_id: Option<u64>,
    /// 登录用户名（从 SaToken extra_data 提取）
    pub user_name: Option<String>,
    /// 租户 ID（从 SaToken extra_data 提取）
    pub tenant_id: Option<u64>,
}

/// 操作日志 Layer
///
/// 包装 axum::routing::Route，提取每次 HTTP 请求的元数据（方法、URI、状态码、
/// 耗时、IP、UA），完成后通过有界 channel 发送。channel 满时丢弃日志并 warn，
/// 绝不阻塞 HTTP 响应。
///
/// 使用 `tx_di_axum::add_layer(self, sort)` 注册到全局中间件链。
#[derive(Clone)]
pub struct OperateLogLayer {
    tx: mpsc::Sender<OperateLogEntry>,
    /// sa-token 的 token 名（用于从请求头/ Cookie 中解析用户）
    token_name: String,
}

impl OperateLogLayer {
    pub fn new(tx: mpsc::Sender<OperateLogEntry>, token_name: impl Into<String>) -> Self {
        Self {
            tx,
            token_name: token_name.into(),
        }
    }
}

impl Layer<axum::routing::Route> for OperateLogLayer {
    type Service = OperateLogMiddleware;

    fn layer(&self, inner: axum::routing::Route) -> Self::Service {
        OperateLogMiddleware {
            inner,
            tx: self.tx.clone(),
            token_name: self.token_name.clone(),
        }
    }
}

/// 操作日志中间件
#[derive(Clone)]
pub struct OperateLogMiddleware {
    inner: axum::routing::Route,
    tx: mpsc::Sender<OperateLogEntry>,
    token_name: String,
}

impl Service<Request<Body>> for OperateLogMiddleware {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <axum::routing::Route as Service<Request<Body>>>::poll_ready(&mut self.inner, cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let trace_id = id::next_id().to_string();
        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let user_ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .or_else(|| req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()))
            .unwrap_or("")
            .to_string();
        let user_agent = req
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        // 在 req 移入 inner 之前，先从请求头/ Cookie 中提取 token，
        // 用于在响应后显式查询用户信息（不依赖 SaToken task-local context）。
        let token = extract_token(&req, &self.token_name);

        let mut inner = self.inner.clone();
        let tx = self.tx.clone();

        Box::pin(async move {
            let start = Instant::now();
            let response = inner.call(req).await?;
            let status = response.status().as_u16();
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

            // 提取当前登录用户信息（仅已认证路由有效）
            let (user_id, user_name, tenant_id) = match token.as_deref() {
                Some(t) => extract_user_info_from_token(t).await,
                None => (None, None, None),
            };

            let entry = OperateLogEntry {
                trace_id,
                method,
                uri,
                status,
                latency_ms,
                user_ip,
                user_agent,
                user_id,
                user_name,
                tenant_id,
            };
            // 审计日志不静默丢弃：channel 满时最多等待 OPERATE_LOG_SEND_TIMEOUT，
            // 超时或消费者退出才放弃（并输出告警便于排查）。
            match tokio::time::timeout(OPERATE_LOG_SEND_TIMEOUT, tx.send(entry)).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    // channel 已关闭，消费者已退出，静默忽略
                }
                Err(_) => {
                    warn!(
                        "操作日志 channel 积压超过 {OPERATE_LOG_SEND_TIMEOUT:?}，丢弃 1 条日志（消费者处理过慢）"
                    );
                }
            }

            Ok(response)
        })
    }
}

/// 从请求头 / Cookie 中提取 sa-token token。
///
/// 提取顺序（与 sa-token 一致）：
/// 1. Header `token_name`（Bearer 语义）
/// 2. `Authorization` header（若 `token_name` 不是 Authorization）
/// 3. Cookie `token_name`
fn extract_token(req: &Request<Body>, token_name: &str) -> Option<String> {
    // 1. Header token_name（支持 "Bearer xxx"）
    if let Some(v) = req.headers().get(token_name) {
        let s = v.to_str().ok()?.trim();
        let s = s.strip_prefix("Bearer ").unwrap_or(s);
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    // 2. Authorization header（若 token_name 不是 Authorization）
    if !token_name.eq_ignore_ascii_case("authorization")
        && let Some(v) = req.headers().get("authorization")
    {
        let s = v.to_str().ok()?.trim();
        let s = s.strip_prefix("Bearer ").unwrap_or(s);
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    // 3. Cookie token_name
    if let Some(cookie) = req.headers().get(header::COOKIE) {
        let cookie = cookie.to_str().ok()?;
        for pair in cookie.split(';') {
            let mut kv = pair.trim().splitn(2, '=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                if k.trim() == token_name && !v.trim().is_empty() {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

/// 通过显式传入的 token 提取当前登录用户信息。
///
/// 不使用 `StpUtil::get_login_id_as_string()`（它依赖 SaToken task-local context，
/// 在操作日志中间件这类外层中间件中已失效），而是直接以 token 查询会话存储。
async fn extract_user_info_from_token(token_str: &str) -> (Option<u64>, Option<String>, Option<u64>) {
    // 1. 用 token 获取 login_id
    let token = TokenValue::new(token_str.to_string());
    let login_id = match StpUtil::get_login_id(&token).await {
        Ok(id) => id,
        Err(_) => {
            debug!("token 无效，跳过用户信息提取");
            return (None, None, None);
        }
    };

    // 2. 解析 user_id
    let user_id: Option<u64> = login_id.parse().ok();

    // 3. 获取 token 信息，读取 extra_data
    let token_info = match StpUtil::get_token_info(&token).await {
        Ok(info) => info,
        Err(e) => {
            debug!("无法获取 token_info: {:?}", e);
            return (user_id, None, None);
        }
    };

    // 4. 反序列化 extra_data → SessionEctData
    let extra = match token_info.extra_data {
        Some(ref data) => match serde_json::from_value::<SessionEctData>(data.clone()) {
            Ok(d) => Some(d),
            Err(e) => {
                debug!("extra_data 反序列化失败: {:?}", e);
                None
            }
        },
        None => None,
    };

    let user_name = extra.as_ref().map(|e| e.username.clone());
    let tenant_id = extra.map(|e| e.tenant_id.into_inner());

    (user_id, user_name, tenant_id)
}
