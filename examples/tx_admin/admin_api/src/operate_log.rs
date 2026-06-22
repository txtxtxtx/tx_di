/// 有界 channel 容量，缓冲 4096 条日志条目。
///
/// 当 channel 满时，新产生的日志会被丢弃并输出 warn 级别告警，
/// 以确保 HTTP 响应不被阻塞。

/// HTTP 请求的操作日志条目，由中间件在请求完成后通过有界 channel 发送给消费者。
///
/// # Fields
///
/// - `method` — HTTP 方法（如 `"GET"`、`"POST"`）
/// - `uri` — 请求 URI
/// - `status` — HTTP 响应状态码
/// - `latency_ms` — 请求耗时（毫秒）
/// - `user_ip` — 客户端 IP，优先取 `x-forwarded-for`，其次 `x-real-ip`
/// - `user_agent` — User-Agent 头部值

/// 操作日志 Layer，将 HTTP 请求元数据通过有界 channel 异步发送给消费者。
///
/// 包装 `axum::routing::Route`，提取每次请求的方法、URI、状态码、耗时、IP、UA，
/// 请求完成后通过有界 [`mpsc::Sender`] 发送 [`OperateLogEntry`]。
/// channel 满时丢弃日志并输出 warn，绝不阻塞 HTTP 响应。
///
/// 通过 [`tx_di_axum::add_layer`] 注册到全局中间件链，建议 sort 值为 15
/// （紧接 `api_log(10)` 之后、压缩层 `100` 之前）。
///
/// # Examples
///
/// 在 `plugin.rs` 中注册：
///
/// ```ignore
/// let (tx, rx) = mpsc::channel::<OperateLogEntry>(OPERATE_LOG_CHANNEL_CAP);
/// let layer = OperateLogLayer::new(tx);
/// add_layer(layer, 15);
/// ```
///
/// # Panics
///
/// 不会 panic；channel 满时丢弃日志而非阻塞。

/// 创建操作日志 Layer，传入有界 channel 的发送端。
///
/// # Panics
///
/// 不会 panic。

/// 操作日志中间件，在 HTTP 请求完成后提取元数据并通过有界 channel 发送。
///
/// 内部委托 `axum::routing::Route` 处理请求，响应完成后构造 [`OperateLogEntry`]
/// 并调用 [`mpsc::Sender::try_send`] 发送。发送失败时：
///
/// - **Full** — channel 已满，输出 warn 并丢弃该条日志
/// - **Closed** — 消费者已退出，静默忽略
///
/// # Errors
///
/// 本服务永远不会返回错误（`type Error = Infallible`）。
use admin_domain::shared::model::value_object::SessionEctData;
use axum::{
    body::Body,
    http::{Request, Response, header},
};
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
use tx_di_sa_token::StpUtil;

/// 有界 channel 容量：缓冲 4096 条日志，超出则丢弃（避免阻塞 HTTP 响应）
pub const OPERATE_LOG_CHANNEL_CAP: usize = 4096;

/// 操作日志条目，在请求完成后通过 channel 发送给消费者
#[derive(Debug, Clone)]
pub struct OperateLogEntry {
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
}

impl OperateLogLayer {
    pub fn new(tx: mpsc::Sender<OperateLogEntry>) -> Self {
        Self { tx }
    }
}

impl Layer<axum::routing::Route> for OperateLogLayer {
    type Service = OperateLogMiddleware;

    fn layer(&self, inner: axum::routing::Route) -> Self::Service {
        OperateLogMiddleware {
            inner,
            tx: self.tx.clone(),
        }
    }
}

/// 操作日志中间件
#[derive(Clone)]
pub struct OperateLogMiddleware {
    inner: axum::routing::Route,
    tx: mpsc::Sender<OperateLogEntry>,
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
        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let user_ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .or_else(|| {
                req.headers()
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
            })
            .unwrap_or("")
            .to_string();
        let user_agent = req
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let mut inner = self.inner.clone();
        let tx = self.tx.clone();

        Box::pin(async move {
            let start = Instant::now();
            let response = inner.call(req).await?;
            let status = response.status().as_u16();
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

            // 提取当前登录用户信息（仅已认证路由有效）
            let (user_id, user_name, tenant_id) =
                extract_user_info().await;

            match tx.try_send(OperateLogEntry {
                method,
                uri,
                status,
                latency_ms,
                user_ip,
                user_agent,
                user_id,
                user_name,
                tenant_id,
            }) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    warn!("操作日志 channel 已满 (cap={})，丢弃日志", OPERATE_LOG_CHANNEL_CAP);
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    // channel 已关闭，消费者已退出，静默忽略
                }
            }

            Ok(response)
        })
    }
}

/// 从 SaToken 会话中提取当前登录用户信息。
///
/// 仅在已通过 SaTokenLayer 认证的路由上有效；公开路由（如 /api/auth/login）
/// 未通过认证层，`get_login_id_as_string()` 会返回错误，此时返回全 `None`。
async fn extract_user_info() -> (Option<u64>, Option<String>, Option<u64>) {
    // 1. 获取当前 login_id
    let login_id = match StpUtil::get_login_id_as_string().await {
        Ok(id) => id,
        Err(_) => {
            debug!("未登录请求，跳过用户信息提取");
            return (None, None, None);
        }
    };

    // 2. 解析 user_id
    let user_id: Option<u64> = login_id.parse().ok();

    // 3. 通过 login_id 获取 token 值
    let token = match StpUtil::get_token_by_login_id(&login_id).await {
        Ok(t) => t,
        Err(_) => {
            debug!("无法获取 token (login_id={})", login_id);
            return (user_id, None, None);
        }
    };

    // 4. 获取 token 信息，读取 extra_data
    let token_info = match StpUtil::get_token_info(&token).await {
        Ok(info) => info,
        Err(e) => {
            debug!("无法获取 token_info: {:?}", e);
            return (user_id, None, None);
        }
    };

    // 5. 反序列化 extra_data → SessionEctData
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
