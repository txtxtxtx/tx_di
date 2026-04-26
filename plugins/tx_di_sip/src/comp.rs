//! SIP 插件主组件
//!
//! [`SipPlugin`] 是 tx_di_sip 插件的核心，负责：
//! 1. 根据 [`SipConfig`] 创建 UDP/TCP 传输层
//! 2. 构建 rsipstack `Endpoint` 并启动服务
//! 3. 将收到的 SIP 事务分发给 [`SipRouter`] 中注册的处理器
//! 4. 暴露 [`SipSender`] 供应用层发送 SIP 消息

use crate::config::SipConfig;
use crate::handler::SipRouter;
use crate::sender::SipSender;
use rsipstack::sip::HeadersExt;
use rsipstack::transport::tcp_listener::TcpListenerConnection;
use rsipstack::transport::udp::UdpConnection;
use rsipstack::transport::TransportLayer;
use rsipstack::transport::SipAddr;
use rsipstack::transaction::endpoint::EndpointInnerRef;
use rsipstack::{transaction::Endpoint, EndpointBuilder};
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use tokio_util::sync::CancellationToken;
use tracing::info;
use tx_di_core::{tx_comp, App, BoxFuture, BuildContext, CompInit, RIE};

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

    /// 端点（在 inner_init 时构建）
    #[tx_cst(skip)]
    pub endpoint: Option<Endpoint>,
}

impl CompInit for SipPlugin {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        info!(
            host = %self.config.host,
            port = self.config.port,
            transport = ?self.config.transport,
            "SIP 插件初始化中..."
        );
        Ok(())
    }

    fn async_init(ctx: Arc<App>) -> BoxFuture<'static, RIE<()>> {
        Box::pin(async move {
            let config = ctx.inject::<SipConfig>();
            let cancel_token = CancellationToken::new();

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
            let incoming = endpoint
                .incoming_transactions()
                .map_err(|e| anyhow::anyhow!("获取 SIP 消息接收通道失败: {}", e))?;

            // 在独立任务中运行 endpoint 驱动循环
            let ep_clone = endpoint.inner.clone();
            let token_clone = cancel_token.clone();
            tokio::spawn(async move {
                tokio::select! {
                    _ = ep_clone.serve() => {
                        info!("SIP endpoint 服务已退出");
                    }
                    _ = token_clone.cancelled() => {
                        info!("SIP endpoint 收到停止信号");
                    }
                }
            });

            // 在独立任务中处理入站消息
            let log_messages = config.log_messages;
            tokio::spawn(async move {
                dispatch_loop(incoming, log_messages).await;
            });

            Ok(())
        })
    }

    fn init_sort() -> i32 {
        i32::MAX - 1
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

/// 入站消息分发循环
async fn dispatch_loop(
    mut incoming: rsipstack::transaction::TransactionReceiver,
    log_messages: bool,
) {
    info!("SIP 消息分发循环已启动");
    while let Some(tx) = incoming.recv().await {
        if log_messages {
            info!(
                method = %tx.original.method,
                from = %tx.original
                    .from_header()
                    .map(|h| h.to_string())
                    .unwrap_or_else(|_| "N/A".into()),
                "收到 SIP 消息"
            );
        }
        tokio::spawn(SipRouter::dispatch(tx));
    }
    info!("SIP 消息分发循环已退出");
}
