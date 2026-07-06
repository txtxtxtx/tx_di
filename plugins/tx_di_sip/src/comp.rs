//! SIP 插件主组件
//!
//! [`SipPlugin`] 是 tx_di_sip 插件的核心，负责：
//! 1. 根据 [`SipConfig`] 创建 UDP/TCP 传输层
//! 2. 构建 rsipstack `Endpoint` 并启动服务
//! 3. 将收到的 SIP 事务经 [`SipRouter`]（含中间件链）分发
//! 4. 暴露 [`SipSender`] 供应用层发送 SIP 消息
//!
//! ## 性能特性
//!
//! - **Semaphore 背压**：限制并发 handler 数量，防止消息风暴 OOM
//! - **Bounded Channel**：配置化队列上限，超限触发背压等待
//! - **O(1) Handler 查找**：DashMap 索引，精确匹配无锁
//!
//! ## 生命周期
//!
//! 使用 `#[derive(Component)] #[component(app_async_init, shutdown, ...)]`：
//! - `app_async_init`：拿到 `app`（含 `cancel_token` 与 `store`），
//!   不再需要「self-inject」；在此收集中间件并注入 router、启动分发循环。
//! - `shutdown`：取消 cancel_token，分发循环据此优雅退出。

use crate::config::SipConfig;
use crate::err::SipErr;
use crate::handler::SipRouter;
use crate::middleware::SipMiddleware;
use crate::sender::SipSender;
use anyhow::anyhow;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use rsipstack::transaction::transaction::Transaction;
use rsipstack::transport::SipAddr;
use rsipstack::transport::TransportLayer;
use rsipstack::transport::tcp_listener::TcpListenerConnection;
use rsipstack::transport::udp::UdpConnection;
use rsipstack::{EndpointBuilder, transaction::Endpoint};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Instant;
use tokio::sync::{Mutex, Semaphore, mpsc};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tx_di_core::{App, Component, DepsTuple, RIE, inject_all_traits_from_store};

/// SIP 插件组件
///
/// # 配置示例
///
/// ```toml
/// [sip_config]
/// host     = "::"          # IPv6 双栈（同时监听 IPv4）
/// port     = 5060
/// transport = "both"       # UDP + TCP
/// user_agent = "MyApp/1.0"
/// enabled = true           # 是否启用 SIP 服务
/// ```
///
/// # 使用示例
///
/// ```rust,ignore
/// use tx_di_sip::{SipPlugin, SipRouter};
/// use tx_di_core::BuildContext;
///
/// // 1. 注册消息处理器（启动前）
/// SipRouter::new().add_handler(
///     Some("REGISTER"),
///     0,
///     |tx| async move {
///         println!("收到 REGISTER: {}", tx.request().method);
///         tx.reply(rsipstack::sip::StatusCode::OK).await?;
///         Ok(())
///     }
/// );
///
/// // 2. 启动 DI 框架
/// let mut ctx = BuildContext::new(Some("config.toml"));
/// ctx.build_and_run().await.unwrap();
/// ```
#[derive(Component)]
#[component(app_async_init, shutdown, init_sort = 10000)]
pub struct SipPlugin {
    /// SIP 配置（注入自 SipConfig）
    pub config: Arc<SipConfig>,

    /// 路由器（DI 注入；持有方法→handler 映射与中间件链）
    pub sip_router: Arc<SipRouter>,

    /// 端点（在 app_async_init 时构建）
    #[tx_cst(OnceLock::new())]
    pub endpoint: OnceLock<Endpoint>,

    /// 优雅关闭令牌
    #[tx_cst(OnceLock::new())]
    pub cancel_token: OnceLock<CancellationToken>,

    /// 后台任务集合（Arc<Mutex> 以便 shutdown 可 await join）
    #[tx_cst(Arc::new(Mutex::new(JoinSet::new())))]
    tasks: Arc<Mutex<JoinSet<()>>>,

    /// 启动时刻（用于指标 uptime）
    #[tx_cst(OnceLock::new())]
    start_at: OnceLock<Instant>,
}

/// 应用异步初始化（替代原 CompInit self-inject 反模式）
async fn app_async_init(comp: Arc<SipPlugin>, app: Arc<App>) -> RIE<()> {
    // 直接从 app 取 cancel_token，无需 self-inject（修 P4）
    let token = app.shutdown_token.clone();
    comp.set_cancel_token(token.clone())?;

    if !comp.config.enabled {
        info!("SIP 插件已禁用（enabled=false），跳过启动");
        return Ok(());
    }

    // 构建传输层 + Endpoint
    let transport_layer = build_transport_layer(&comp.config, token.clone()).await?;
    let endpoint = EndpointBuilder::new()
        .with_cancel_token(token.clone())
        .with_transport_layer(transport_layer)
        .with_user_agent(&comp.config.user_agent)
        .build();
    comp.set_end_point(endpoint)?;

    // 收集中间件并注入 router（DI 收集，替代全局 REGISTRY，修 P1/P2）
    let mws = inject_all_traits_from_store::<dyn SipMiddleware>(&app.store);
    comp.sip_router.set_middlewares(mws);

    // 启动入站分发循环（生产者 + 消费者）
    comp.start_dispatch_loop().await?;

    comp.start_at.set(Instant::now()).ok();
    info!("SIP 插件启动成功，监听 {}", comp.config.bind_addr());
    Ok(())
}

/// 优雅关闭：取消令牌，分发循环据此退出（修 P5）
///
/// 模块级自由函数，由宏生成的 `fn shutdown(&self)` 覆写通过 `self::shutdown(self)` 调用。
fn shutdown(_comp: &SipPlugin) {
    if let Some(t) = _comp.cancel_token.get() {
        info!("优雅关闭 SIP 服务...");
        t.cancel();
    }
    // 生产者/消费者循环在 token.cancelled() 后自行 break 并收尾；
    // JoinSet 随 SipPlugin 析构一并回收。trait shutdown 为同步签名，不在此 await。
}

impl SipPlugin {
    /// 注册 SIP 消息处理器（转发到 SipRouter）
    ///
    /// - `method`：`Some("REGISTER")` 仅处理该方法；`None` 为 catch-all。
    /// - `priority`：数值越小越优先。
    /// - `handler`：异步处理函数，签名 `async fn(SipTx) -> RIE<()>`。
    pub fn add_handler<F, Fut>(
        &self,
        method: Option<impl AsRef<str>>,
        priority: i32,
        handler: F,
    ) -> RIE<()>
    where
        F: Fn(crate::sip_tx::SipTx) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = RIE<()>> + Send + 'static,
    {
        self.sip_router.add_handler(method, priority, handler);
        Ok(())
    }

    /// 启动入站分发循环：生产者（incoming）→ bounded channel → 消费者（Semaphore 限并发）
    async fn start_dispatch_loop(&self) -> RIE<()> {
        let mut incoming = self
            .endpoint
            .get()
            .ok_or("获取 SIP 端点失败")?
            .incoming_transactions()
            .map_err(|e| anyhow!("获取 SIP 消息接收通道失败: {}", e))?;

        // 性能参数来自配置（修 P7：不再读环境变量）
        let queue_size = self.config.dispatch_queue_size;
        let max_handlers = self.config.max_concurrent_handlers;

        // Bounded mpsc channel：send() 在队列满时 await → 天然背压
        let (tx, mut rx) = mpsc::channel::<Transaction>(queue_size);

        // 共享信号量：限制并发 handler 数量
        let sem = Arc::new(Semaphore::new(max_handlers));
        let log_messages = self.config.log_messages;

        let token = self
            .cancel_token
            .get()
            .ok_or("获取 cancel_token 失败")?
            .clone();
        let sip_router = self.sip_router.clone();
        let producer_token = token.clone();

        // 生产者：将 rsipstack incoming 消息送入 bounded channel
        self.tasks.lock().await.spawn(async move {
            tokio::select! {
                _ = producer_token.cancelled() => {
                    info!("SIP 消息生产者收到停止信号");
                }
                _ = async {
                    while let Some(tx_msg) = incoming.recv().await {
                        if tx.send(tx_msg).await.is_err() { break; }
                    }
                } => {
                    info!("SIP 消息生产者已退出");
                }
            }
        });

        // 消费者：顺序从 channel 取消息，并发执行 handler（Semaphore 控并发）
        self.tasks.lock().await.spawn(async move {
            let mut handlers = FuturesUnordered::new();

            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        info!("SIP 消息消费者收到停止信号，停止接收新消息");
                        break;
                    }
                    msg_opt = rx.recv() => {
                        match msg_opt {
                            Some(msg) => {
                                let permit = match sem.clone().acquire_owned().await {
                                    Ok(permit) => permit,
                                    Err(e) => {
                                        error!("Semaphore 获取失败: {}，停止消费者", e);
                                        break;
                                    }
                                };
                                let router = sip_router.clone();
                                let fut = async move {
                                    if log_messages {
                                        info!("收到 SIP 消息：{}", msg.original.method);
                                    }
                                    router.dispatch(msg).await;
                                    drop(permit);
                                };
                                handlers.push(fut);
                            }
                            None => {
                                info!("SIP 消息接收通道已关闭");
                                break;
                            }
                        }
                    }
                }
            }

            // 等待所有处理中的任务完成
            if !handlers.is_empty() {
                info!("等待 {} 个处理中的任务完成", handlers.len());
                while handlers.next().await.is_some() {}
            }
            info!("SIP 消息分发引擎已退出");
        });

        Ok(())
    }

    /// 设置 SIP 端点
    pub fn set_end_point(&self, end_point: Endpoint) -> RIE<()> {
        Ok(self
            .endpoint
            .set(end_point)
            .map_err(|_e| SipErr::EndpointAlreadySet)?)
    }

    /// 获取 SIP 发送器
    ///
    /// 需要在 `app_async_init` 完成后（即 `build_and_run()` 返回后）使用。
    pub fn sender(&self) -> RIE<SipSender> {
        let inner = self
            .endpoint
            .get()
            .as_ref()
            .ok_or("未设置sip端点")?
            .inner
            .clone();
        Ok(SipSender::new(inner, self.config.clone()))
    }

    /// 设置取消令牌（只能成功一次）
    pub fn set_cancel_token(&self, token: CancellationToken) -> RIE<()> {
        self.cancel_token
            .set(token)
            .map_err(|_e| SipErr::TokenAlreadySet)?;
        Ok(())
    }

    /// 获取取消令牌的克隆
    pub fn get_cancel_token(&self) -> RIE<CancellationToken> {
        Ok(self.cancel_token.get().cloned().ok_or("令牌尚未设置")?)
    }

    /// 触发优雅关闭（如果令牌已设置）
    pub fn shutdown_signal(&self) {
        if let Some(token) = self.cancel_token.get() {
            info!("触发 SIP 服务关闭信号...");
            token.cancel();
        }
    }
}

// ── 内部辅助函数 ─────────────────────────────────────────────────────────────

/// 构建传输层（根据配置绑定 UDP/TCP）
async fn build_transport_layer(
    config: &SipConfig,
    cancel_token: CancellationToken,
) -> RIE<TransportLayer> {
    let transport_layer = TransportLayer::new(cancel_token.clone());
    let addr: SocketAddr = config.socket_addr()?;

    // ── UDP ────────────────────────────────────────────────────────────────
    if config.enable_udp() {
        let udp_conn = UdpConnection::create_connection(
            addr,
            // external_ip 用于 NAT 场景
            config
                .external_ip
                .as_ref()
                .and_then(|ip| format!("{}:{}", ip, config.port).parse::<SocketAddr>().ok()),
            Some(cancel_token.child_token()),
        )
        .await
        .map_err(|e| anyhow::anyhow!("UDP transport 绑定 {} 失败: {}", addr, e))?;

        transport_layer.add_transport(udp_conn.into());
        info!("SIP UDP transport 已绑定: {}", addr);
    }

    // ── TCP ────────────────────────────────────────────────────────────────
    if config.enable_tcp() {
        let local_addr = SipAddr {
            r#type: Some(rsipstack::sip::transport::Transport::Tcp),
            addr: addr.into(),
        };
        let external = config
            .external_ip
            .as_ref()
            .and_then(|ip| format!("{}:{}", ip, config.port).parse::<SocketAddr>().ok());

        let tcp_conn = TcpListenerConnection::new(local_addr, external)
            .await
            .map_err(|e| anyhow::anyhow!("TCP transport 绑定 {} 失败: {}", addr, e))?;

        transport_layer.add_transport(tcp_conn.into());
        info!("SIP TCP transport 已绑定: {}", addr);
    }

    // ── TLS ────────────────────────────────────────────────────────
    #[cfg(feature = "rustls")]
    if matches!(config.transport, crate::config::SipTransport::Tls) {
        build_tls_transport(config, addr, &mut transport_layer).await?;
    }

    // ── WebSocket ──────────────────────────────────────────────────
    #[cfg(feature = "websocket")]
    if matches!(config.transport, crate::config::SipTransport::Ws) {
        build_ws_transport(config, addr, &mut transport_layer).await?;
    }

    Ok(transport_layer)
}

// ── TLS ────────────────────────────────────────────────────────────────

#[cfg(feature = "rustls")]
async fn build_tls_transport(
    config: &SipConfig,
    addr: SocketAddr,
    transport_layer: &mut rsipstack::transport::TransportLayer,
) -> anyhow::Result<()> {
    let tls_cfg = config.tls.as_ref().ok_or_else(|| {
        anyhow::anyhow!("启用 TLS 传输但未配置 [sip_config.tls]")
    })?;

    let cert_bytes = tokio::fs::read(&tls_cfg.cert_pem)
        .await
        .map_err(|e| anyhow::anyhow!("读取 TLS 证书失败: {}", e))?;
    let key_bytes = tokio::fs::read(&tls_cfg.key_pem)
        .await
        .map_err(|e| anyhow::anyhow!("读取 TLS 私钥失败: {}", e))?;

    let rsip_tls = rsipstack::transport::tls::TlsConfig {
        cert: Some(cert_bytes),
        key: Some(key_bytes),
        client_cert: None,
        client_key: None,
        ca_certs: None,
        sni_hostname: None,
    };

    let local_addr = rsipstack::transport::SipAddr {
        r#type: Some(rsipstack::sip::transport::Transport::Tls),
        addr: addr.into(),
    };
    let external = config
        .external_ip
        .as_ref()
        .and_then(|ip| format!("{}:{}", ip, config.port).parse::<SocketAddr>().ok());

    let tls_conn = rsipstack::transport::tls::TlsListenerConnection::new(
        local_addr,
        external,
        rsip_tls,
    )
    .await
    .map_err(|e| anyhow::anyhow!("TLS transport 绑定 {} 失败: {}", addr, e))?;

    transport_layer.add_transport(tls_conn.into());
    info!("SIP TLS transport 已绑定: {}", addr);
    Ok(())
}

// ── WebSocket ──────────────────────────────────────────────────────────

#[cfg(feature = "websocket")]
async fn build_ws_transport(
    config: &SipConfig,
    addr: SocketAddr,
    transport_layer: &mut rsipstack::transport::TransportLayer,
) -> anyhow::Result<()> {
    let local_addr = rsipstack::transport::SipAddr {
        r#type: Some(rsipstack::sip::transport::Transport::Ws),
        addr: addr.into(),
    };
    let external = config
        .external_ip
        .as_ref()
        .and_then(|ip| format!("{}:{}", ip, config.port).parse::<SocketAddr>().ok());

    let ws_conn = rsipstack::transport::websocket::WebSocketListenerConnection::new(
        local_addr,
        external,
        false, // is_secure (WSS 需另行配置)
    )
    .await
    .map_err(|e| anyhow::anyhow!("WS transport 绑定 {} 失败: {}", addr, e))?;

    transport_layer.add_transport(ws_conn.into());
    info!("SIP WS transport 已绑定: {}", addr);
    Ok(())
}
