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
use crate::handler::SipRouter;
use crate::sender::SipSender;
use rsipstack::transport::tcp_listener::TcpListenerConnection;
use rsipstack::transport::udp::UdpConnection;
use rsipstack::transport::TransportLayer;
use rsipstack::transport::SipAddr;
use rsipstack::transaction::endpoint::EndpointInnerRef;
use rsipstack::{transaction::Endpoint, EndpointBuilder};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tx_di_core::{tx_comp, App, BoxFuture, CompInit, InnerContext, RIE};

// ── 性能配置常量 ──────────────────────────────────────────────────────────────

/// Handler 并发上限（防止消息风暴 OOM）
const MAX_CONCURRENT_HANDLERS: usize = 1000;

/// 消息分发 channel 容量（生产者 → 消费者队列）
const DISPATCH_CHANNEL_CAPACITY: usize = 10_000;

/// 全局 SIP 端点 Inner 引用，供 `SipSender` 在异步 init 之后访问
static ENDPOINT_INNER: OnceLock<EndpointInnerRef> = OnceLock::new();

/// 全局取消令牌（用于优雅停止 SIP 服务）
static CANCEL_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

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
    #[tx_cst(skip)]
    pub endpoint: Option<Endpoint>,
}

impl CompInit for SipPlugin {
    fn inner_init(&mut self, _: &InnerContext ) -> RIE<()> {
        info!(
            host = %self.config.host,
            port = self.config.port,
            transport = ?self.config.transport,
            "SIP 插件初始化中..."
        );
        Ok(())
    }

    fn async_init(ctx: Arc<App>,token: CancellationToken) -> BoxFuture<'static, RIE<()>> {
        Box::pin(async move {
            let config = ctx.inject::<SipConfig>();
            let cancel_token = token;

            // 存储取消令牌
            let _ = CANCEL_TOKEN.set(cancel_token.clone());

            // 构建传输层
            let transport_layer = build_transport_layer(&config, cancel_token.clone()).await?;

            // 构建 Endpoint
            let endpoint = EndpointBuilder::new()
                .with_cancel_token(cancel_token.clone())
                .with_transport_layer(transport_layer)
                .with_user_agent(&config.user_agent)
                .build();

            // 存储 EndpointInner 供 Sender 使用
            let _ = ENDPOINT_INNER.set(endpoint.inner.clone());

            info!("SIP 插件启动成功，监听 {}", config.bind_addr());

            // 获取消息接收通道
            let mut incoming = endpoint
                .incoming_transactions()
                .map_err(|e| anyhow::anyhow!("获取 SIP 消息接收通道失败: {}", e))?;

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
            let (tx, mut rx) = mpsc::channel::<rsipstack::transaction::transaction::Transaction>(queue_size);

            // 共享信号量：限制并发 handler 数量
            let sem = Arc::new(Semaphore::new(max_handlers));

            // 过载标志（避免日志刷屏）
            let overload_flag = Arc::new(AtomicBool::new(false));

            let log_messages = config.log_messages;
            let ep_clone = endpoint.inner.clone();
            let token_clone = cancel_token.clone();

            // Endpoint serve 任务
            let _ = tokio::spawn(async move {
                tokio::select! {
                    _ = ep_clone.serve() => {
                        info!("SIP endpoint 服务已退出");
                    }
                    _ = token_clone.cancelled() => {
                        info!("SIP endpoint 收到停止信号");
                    }
                }
            });

            // 生产者：将 rsipstack incoming 消息送入 bounded channel
            // 队列满时：send().await 自动 park，形成背压
            // 接收sip消息放入队列
            tokio::spawn(async move {
                while let Some(sip_tx) = incoming.recv().await {
                    if tx.send(sip_tx).await.is_err() {
                        warn!("dispatch channel 已关闭，停止生产者");
                        break;
                    }
                }
                info!("SIP 消息生产者已退出");
            });

            // 消费者：顺序从 channel 取消息，并发执行 handler
            // 关键：消息接收顺序是串行的（保证同设备消息有序），
            //       但 handler 执行是并发的（Semaphore 控制并发数）
            // 使用handler 处理队列里面的sip消息
            tokio::spawn(async move {
                info!(
                    queue_capacity = queue_size,
                    max_handlers = max_handlers,
                    "SIP 消息分发引擎已启动"
                );
                while let Some(tx) = rx.recv().await {
                    // Semaphore permit：超过并发上限时 park
                    let permit = sem.clone().acquire_owned().await;
                    let Ok(_permit) = permit else {
                        error!("Semaphore 获取失败，停止消费者");
                        break;
                    };

                    let flag = overload_flag.clone();
                    let sem_clone = sem.clone();
                    let log = log_messages;

                    // Handler 在独立 task 中执行，不阻塞后续消息接收
                    tokio::spawn(async move {
                        // 感知并发压力
                        let available = sem_clone.available_permits();
                        if available == 0 {
                            if !flag.load(Ordering::Relaxed) {
                                flag.store(true, Ordering::Relaxed);
                                warn!(
                                    "SIP handler 并发已达上限({})，消息处理排队中",
                                    max_handlers
                                );
                            }
                        } else if available > max_handlers / 4
                            && flag.load(Ordering::Relaxed)
                        {
                            flag.store(false, Ordering::Relaxed);
                            info!("SIP handler 并发压力已缓解");
                        }

                        if log {
                            let method = tx.original.method.to_string();
                            info!(method = %method, "SIP 分发消息");
                        }

                        SipRouter::dispatch(tx).await;
                        // drop(permit); // permit 在此 drop，自动释放
                    });
                }
                info!("SIP 消息分发引擎已退出");
            });

            info!("SIP 插件异步初始化完成");
            Ok(())
        })
    }

    fn init_sort() -> i32 {
        10000
    }
}

impl SipPlugin {
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
    pub fn sender() -> SipSender {
        let inner = ENDPOINT_INNER
            .get()
            .expect("SipPlugin 尚未初始化，请确保 build_and_run() 已完成")
            .clone();
        SipSender::new(inner)
    }

    /// 获取底层 CancellationToken，用于优雅停止 SIP 服务
    pub fn cancel_token() -> Option<CancellationToken> {
        CANCEL_TOKEN.get().cloned()
    }

    /// 优雅地停止 SIP 服务
    pub fn shutdown() {
        if let Some(token) = CANCEL_TOKEN.get() {
            info!("正在停止 SIP 服务...");
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
            config.external_ip.as_ref().and_then(|ip| {
                format!("{}:{}", ip, config.port)
                    .parse::<SocketAddr>()
                    .ok()
            }),
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
        let external = config.external_ip.as_ref().and_then(|ip| {
            format!("{}:{}", ip, config.port)
                .parse::<SocketAddr>()
                .ok()
        });

        let tcp_conn = TcpListenerConnection::new(local_addr, external)
            .await
            .map_err(|e| anyhow::anyhow!("TCP transport 绑定 {} 失败: {}", addr, e))?;

        transport_layer.add_transport(tcp_conn.into());
        info!("SIP TCP transport 已绑定: {}", addr);
    }

    Ok(transport_layer)
}
