//! SIP 插件主组件
//!
//! [`SipPlugin`] 是 tx_di_sip 插件的核心，负责：
//! 1. 根据 [`SipConfig`] 创建 UDP/TCP 传输层
//! 2. 构建 rsipstack `Endpoint` 并启动服务
//! 3. 将收到的 SIP 事务分发给 [`SipRouter`] 中注册的处理器
//! 4. 暴露 [`SipSender`] 供应用层发送 SIP 消息
//!
//! ## 性能特性
//!
//! - **Semaphore 背压**：限制并发 handler 数量，防止消息风暴 OOM
//! - **Bounded Channel**：10,000 消息队列上限，超限触发背压等待
//! - **O(1) Handler 查找**：DashMap 索引，精确匹配无锁

use crate::config::SipConfig;
use crate::err::SipErr;
use crate::handler::SipRouter;
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
use tokio::sync::{Mutex, Semaphore, mpsc};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tx_di_core::{App, CompInit, RIE, tx_comp};

// ── 性能配置常量 ──────────────────────────────────────────────────────────────

/// Handler 并发上限（防止消息风暴 OOM）
const MAX_CONCURRENT_HANDLERS: usize = 1000;

/// 消息分发 channel 容量（生产者 → 消费者队列）
const DISPATCH_CHANNEL_CAPACITY: usize = 10_000;

/// SIP 插件组件
///
/// 与 `WebPlugin` 对称，集成到 tx-di 框架中自动初始化。
///
/// # 配置示例
///
/// ```toml
/// [sip_config]
/// host     = "::"          # IPv6 双栈（同时监听 IPv4）
/// port     = 5060
/// transport = "both"       # UDP + TCP
/// user_agent = "MyApp/1.0"
/// ```
///
/// # 性能配置（环境变量覆盖）
///
/// ```bash
/// SIP_MAX_HANDLERS=2000  # 并发上限，默认 1000
/// SIP_QUEUE_SIZE=20000   # 队列容量，默认 10000
/// ```
///
/// # 使用示例
///
/// ```rust,no_run
/// use tx_di_sip::{SipPlugin, SipRouter};
/// use tx_di_core::BuildContext;
///
/// // 1. 注册消息处理器（启动前）
/// SipRouter::add_handler(
///     Some("REGISTER"),
///     0,
///     |mut tx| async move {
///         println!("收到 REGISTER: {}", tx.original);
///         tx.reply(rsipstack::sip::StatusCode::OK).await?;
///         Ok(())
///     }
/// );
///
/// // 2. 启动 DI 框架
/// let mut ctx = BuildContext::new(Some("config.toml"));
/// ctx.build_and_run().await.unwrap();
/// ```
#[tx_comp(init)]
pub struct SipPlugin {
    /// SIP 配置（注入自 SipConfig）
    pub config: Arc<SipConfig>,

    /// 端点（在 inner_init 时构建）,没有使用
    #[tx_cst(OnceLock::new())]
    pub endpoint: OnceLock<Endpoint>,
    /// 优雅关闭
    #[tx_cst(OnceLock::new())]
    pub cancel_token: OnceLock<CancellationToken>,

    /// SipRouter
    #[tx_cst(SipRouter::new())]
    pub sip_router: SipRouter,
    // 任务
    #[tx_cst(Mutex::new(JoinSet::new()))]
    tasks: Mutex<JoinSet<()>>,
}

impl CompInit for SipPlugin {
    fn init(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
        let sip_plugin = ctx.inject::<SipPlugin>();
        // 存储取消令牌,确保在异步任务中可以使用
        sip_plugin.set_cancel_token(token)?;
        Ok(())
    }

    tx_di_core::async_method!(
        fn async_init_impl(ctx: Arc<App>, cancel_token: CancellationToken) -> RIE<()> {
            let config = ctx.inject::<SipConfig>();
            let sip_plugin = ctx.inject::<SipPlugin>();
            // 构建传输层
            let transport_layer = build_transport_layer(&config, cancel_token.clone()).await?;
            // 构建 Endpoint
            let endpoint = EndpointBuilder::new()
                .with_cancel_token(cancel_token.clone())
                .with_transport_layer(transport_layer)
                .with_user_agent(&config.user_agent)
                .build();
            // 存储 Endpoint 供 Sender 使用
            sip_plugin.set_end_point(endpoint)?;
            let ep_clone = sip_plugin.endpoint.get().unwrap().inner.clone();
            // Endpoint serve 任务
            let _ = tokio::spawn(async move {
                tokio::select! {
                    _ = ep_clone.serve() => {
                        info!("SIP endpoint 服务已退出");
                    }
                    _ = cancel_token.cancelled() => {
                        info!("SIP endpoint 收到停止信号");
                    }
                }
            });
            sip_plugin.in_coming().await?;
            // 注册sip消息处理器
            info!("SIP 插件启动成功，监听 {}", config.bind_addr());
            Ok(())
        }
    );

    fn init_sort() -> i32 {
        10000
    }
}

impl SipPlugin {
    /// 注册 SIP 消息处理器
    ///
    /// 向 SIP 路由器注册一个异步消息处理器，用于处理特定类型的 SIP 请求。
    ///
    /// # 参数
    ///
    /// - `method`: SIP 方法名过滤器
    ///   - `Some("REGISTER")`: 仅处理 REGISTER 方法的请求
    ///   - `Some("INVITE")`: 仅处理 INVITE 方法的请求
    ///   - `None`: 作为 catch-all 处理器，匹配所有未精确匹配的方法
    ///
    /// - `priority`: 处理器优先级，数值越小优先级越高
    ///   - 同一方法可以注册多个处理器，按优先级升序执行
    ///   - 推荐范围：0-100，catch-all 建议使用较高值（如 99）
    ///
    /// - `handler`: 异步处理函数，接收 `Transaction` 并返回 `RIE<()>`
    ///   - 函数签名：`async fn(Transaction) -> RIE<()>`
    ///   - 必须实现 `Send + Sync + 'static` 以支持跨线程执行
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 成功注册处理器
    /// - `Err(_)`: SipRouter 未初始化时返回错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use tx_di_sip::SipPlugin;
    /// use rsipstack::sip::StatusCode;
    ///
    /// // 注册 REGISTER 处理器
    /// plugin.add_sip_handler(
    ///     Some("REGISTER"),
    ///     0,
    ///     |mut tx| async move {
    ///         println!("收到 REGISTER 请求");
    ///         tx.reply(StatusCode::OK).await?;
    ///         Ok(())
    ///     }
    /// )?;
    ///
    /// // 注册 catch-all 处理器（兜底）
    /// plugin.add_sip_handler(
    ///     None,
    ///     99,
    ///     |mut tx| async move {
    ///         tx.reply(StatusCode::MethodNotAllowed).await?;
    ///         Ok(())
    ///     }
    /// )?;
    /// ```
    pub fn add_handler<F, Fut>(
        &self,
        method: Option<impl AsRef<str>>,
        priority: i32,
        handler: F,
    ) -> RIE<()>
    where
        F: Fn(Transaction) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RIE<()>> + Send + 'static,
    {
        self.sip_router.add_handler(method, priority, handler);
        Ok(())
    }

    pub async fn in_coming(&self) -> RIE<()> {
        // 获取消息接收通道
        let mut incoming = self
            .endpoint
            .get()
            .ok_or("获取 SIP 端点失败")?
            .incoming_transactions()
            .map_err(|e| anyhow!("获取 SIP 消息接收通道失败: {}", e))?;

        // ── 性能优化：Semaphore 背压 + Bounded Channel ──────────────────
        let queue_size = std::env::var("SIP_QUEUE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DISPATCH_CHANNEL_CAPACITY);

        let max_handlers = std::env::var("SIP_MAX_HANDLERS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(MAX_CONCURRENT_HANDLERS);

        // Bounded mpsc channel：send() 在队列满时 await → 天然背压 多生产者单消费者
        let (tx, mut rx) = mpsc::channel::<Transaction>(queue_size);

        // 共享信号量：限制并发 handler 数量
        let sem = Arc::new(Semaphore::new(max_handlers));

        let log_messages = self.config.log_messages;

        let token_clone = self
            .cancel_token
            .get()
            .ok_or("获取 cancel_token 失败")?
            .clone();
        let sip_router = self.sip_router.clone();
        let producer_token_clone = token_clone.clone();

        // 生产者：将 rsipstack incoming 消息送入 bounded channel
        // 队列满时：send().await 自动 park，形成背压
        // 接收sip消息放入队列
        let producer = async move {
            tokio::select! {
                _ = producer_token_clone.cancelled() => {
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
        };
        self.tasks.lock().await.spawn(producer);

        // 消费者任务：使用 FuturesUnordered 代替手动 spawn
        // 消费者：顺序从 channel 取消息，并发执行 handler
        // 关键：消息接收顺序是串行的（保证同设备消息有序），
        //       但 handler 执行是并发的（Semaphore 控制并发数）
        // 使用handler 处理队列里面的sip消息
        let consumer = async move {
            let mut handlers = FuturesUnordered::new();

            // 主循环：接收消息或等待取消信号
            loop {
                tokio::select! {
                    _ = token_clone.cancelled() => {
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
                                        let method = msg.original.method.to_string();
                                        info!("收到 SIP 消息：{}", method);
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
                while let Some(_) = handlers.next().await {}
            }
            info!("SIP 消息分发引擎已退出");
        };

        self.tasks.lock().await.spawn(consumer);

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
    /// 需要在 `async_init` 完成后（即 `build_and_run()` 返回后）使用。
    ///
    /// # Panics
    ///
    /// 若在 SIP 插件初始化前调用，会 panic。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// let sender = SipPlugin::sender();
    /// sender.register("sip:192.168.1.1", "1001", "password").await?;
    /// ```
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
    ///
    /// # 返回
    /// - `Ok(())`：成功设置
    /// - `Err(CancellationToken)`：令牌已存在
    pub fn set_cancel_token(&self, token: CancellationToken) -> RIE<()> {
        self.cancel_token.set(token).map_err(|_e| SipErr::TokenAlreadySet)?;
        Ok(())
    }

    /// 获取取消令牌的克隆
    ///
    /// # 返回
    /// - `Ok(CancellationToken)`：成功获取
    pub fn get_cancel_token(&self) -> RIE<CancellationToken> {
        Ok(self.cancel_token.get().cloned().ok_or("令牌尚未设置")?)
    }

    /// 触发优雅关闭（如果令牌已设置）
    pub fn shutdown(&self) {
        if let Some(token) = self.cancel_token.get() {
            info!("优雅关闭 SIP 服务...");
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
    if matches!(config.transport, SipTransport::Tls) {
        build_tls_transport(config, addr, &mut transport_layer).await?;
    }

    // ── WebSocket ──────────────────────────────────────────────────
    #[cfg(feature = "websocket")]
    if matches!(config.transport, SipTransport::Ws) {
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
