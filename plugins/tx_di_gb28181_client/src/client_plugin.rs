//! GB28181 设备客户端插件主体
//!
//! `Gb28181Device` 是设备端的门面，集成到 tx-di 框架中：
//! - 自动完成注册（含 401 摘要认证）
//! - 后台自动心跳（MESSAGE Keepalive）
//! - 刷新注册（在 TTL 一半时自动刷新）
//! - 断线重连（指数退避）
//! - 注册所有设备端 SIP 消息处理器

use crate::channel::ChannelConfig;
use crate::config::Gb28181DeviceConfig;
use crate::device_event::{self, DeviceEvent};
use crate::device_handlers::register_device_handlers;
use rsipstack::dialog::authenticate::Credential;
use rsipstack::dialog::registration::Registration;
use rsipstack::sip as rsip;
use rsipstack::transaction::key::{TransactionKey, TransactionRole};
use rsipstack::transaction::transaction::Transaction;
use std::future::Future;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::time::{interval, sleep, Duration};
use tracing::{error, info, warn};
use tx_di_core::{tx_comp, App, BoxFuture, BuildContext, CompInit, RIE};
use tx_di_sip::SipPlugin;

/// 全局设备实例
static DEVICE: OnceLock<Arc<DeviceInner>> = OnceLock::new();

struct DeviceInner {
    config: Arc<Gb28181DeviceConfig>,
    channels: Arc<Vec<ChannelConfig>>,
    sn: AtomicU32,
}

impl DeviceInner {
    fn new(config: Arc<Gb28181DeviceConfig>, channels: Vec<ChannelConfig>) -> Self {
        Self {
            config,
            channels: Arc::new(channels),
            sn: AtomicU32::new(1),
        }
    }
}

/// GB28181 设备客户端插件
///
/// # 配置
/// ```toml
/// [gb28181_device_config]
/// device_id   = "34020000001320000001"
/// platform_ip = "192.168.1.200"
/// platform_id = "34020000002000000001"
/// username    = "34020000001320000001"
/// password    = "12345678"
/// local_ip    = "192.168.1.100"
/// ```
///
/// # 使用
/// ```rust,no_run
/// use tx_di_gb28181_client::{Gb28181Device, DeviceEvent, ChannelConfig};
/// use tx_di_core::BuildContext;
///
/// // 添加通道（目录上报时使用）
/// Gb28181Device::add_channel(ChannelConfig::new("34020000001320000001", "Camera-01"));
///
/// // 订阅事件
/// Gb28181Device::on_event(|ev| async move {
///     match ev {
///         DeviceEvent::InviteReceived { call_id, rtp_target_ip, rtp_target_port, .. } => {
///             println!("点播！推流到 {}:{}", rtp_target_ip, rtp_target_port);
///         }
///         DeviceEvent::InviteEnded { call_id } => {
///             println!("点播结束: {}", call_id);
///         }
///         _ => {}
///     }
///     Ok(())
/// });
///
/// let mut ctx = BuildContext::new(Some("gb28181-device.toml"));
/// ctx.build().await.unwrap();
/// // 框架自动完成注册 + 心跳，业务只需响应事件
/// ```
#[tx_comp(init)]
pub struct Gb28181Device {
    pub config: Arc<Gb28181DeviceConfig>,
}

/// 全局通道列表（在 build 前通过 add_channel 添加）
static CHANNELS: OnceLock<std::sync::Mutex<Vec<ChannelConfig>>> = OnceLock::new();

fn channels_store() -> &'static std::sync::Mutex<Vec<ChannelConfig>> {
    CHANNELS.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

impl Gb28181Device {
    /// 订阅设备事件（需在 ctx.build() 之前调用）
    pub fn on_event<F, Fut>(handler: F)
    where
        F: Fn(DeviceEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        device_event::add_event_listener(handler);
    }

    /// 添加设备通道（需在 ctx.build() 之前调用，用于目录上报）
    pub fn add_channel(channel: ChannelConfig) {
        channels_store()
            .lock()
            .expect("channels store lock")
            .push(channel);
    }

    /// 获取操作句柄（需在 ctx.build() 完成后调用）
    pub fn handle() -> Gb28181DeviceHandle {
        let inner = DEVICE
            .get()
            .expect("Gb28181Device 尚未初始化，请确保 ctx.build().await 已完成")
            .clone();
        Gb28181DeviceHandle { inner }
    }
}

impl CompInit for Gb28181Device {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        info!(
            device_id = %self.config.device_id,
            platform_ip = %self.config.platform_ip,
            "GB28181 设备客户端初始化中..."
        );
        Ok(())
    }

    fn async_init(ctx: Arc<App>) -> BoxFuture<'static, RIE<()>> {
        Box::pin(async move {
            let config = ctx.inject::<Gb28181DeviceConfig>();

            // 收集通道配置
            let channels: Vec<ChannelConfig> = channels_store()
                .lock()
                .expect("channels store lock")
                .clone();

            // 注册 SIP 处理器
            register_device_handlers(config.clone(), Arc::new(channels.clone()));

            // 存储全局实例
            let inner = Arc::new(DeviceInner::new(config.clone(), channels));
            let _ = DEVICE.set(inner.clone());

            info!(
                device_id = %config.device_id,
                platform = %config.platform_uri(),
                "✅ GB28181 设备端处理器注册完成，开始自动注册..."
            );

            // 启动注册 + 心跳后台任务
            tokio::spawn(async move {
                run_register_heartbeat_loop(inner).await;
            });

            Ok(())
        })
    }

    fn init_sort() -> i32 {
        i32::MAX
    }
}

// ── 注册 + 心跳主循环 ──────────────────────────────────────────────────────────

/// 注册 + 心跳主循环（后台 task，含指数退避重试）
async fn run_register_heartbeat_loop(inner: Arc<DeviceInner>) {
    let config = &inner.config;
    let max_retries = config.max_register_retries;
    let mut retry_count = 0u32;
    let mut retry_interval = config.retry_interval_secs;

    // ── 阶段 1：注册（带重试） ─────────────────────────────────────────────
    loop {
        match do_register(config).await {
            Ok(()) => {
                retry_count = 0;
                retry_interval = config.retry_interval_secs;
                tokio::spawn(device_event::emit(DeviceEvent::Registered {
                    platform_uri: config.platform_uri(),
                }));
                break;
            }
            Err(e) => {
                retry_count += 1;
                if max_retries > 0 && retry_count > max_retries {
                    error!(
                        error = %e,
                        retries = retry_count,
                        "注册失败次数超过上限，停止重试"
                    );
                    return;
                }
                warn!(
                    error = %e,
                    retry_count = retry_count,
                    retry_in_secs = retry_interval,
                    "注册失败，将重试"
                );
                tokio::spawn(device_event::emit(DeviceEvent::RegisterFailed {
                    reason: e.to_string(),
                    retry_in_secs: retry_interval,
                }));
                sleep(Duration::from_secs(retry_interval)).await;
                // 指数退避，最大 300 秒
                retry_interval = (retry_interval * 2).min(300);
            }
        }
    }

    // ── 阶段 2：心跳 + 刷新注册主循环 ────────────────────────────────────────
    let heartbeat_dur = Duration::from_secs(config.heartbeat_secs);
    let refresh_dur = Duration::from_secs((config.register_ttl / 2).max(60) as u64);

    let mut heartbeat_ticker = interval(heartbeat_dur);
    let mut refresh_ticker = interval(refresh_dur);
    heartbeat_ticker.tick().await; // 跳过立即触发
    refresh_ticker.tick().await;

    info!(
        heartbeat_secs = config.heartbeat_secs,
        refresh_secs = config.register_ttl / 2,
        "⏰ 心跳 & 刷新注册定时器已启动"
    );

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("收到退出信号，准备注销...");
                if let Err(e) = do_unregister(&inner.config).await {
                    warn!("注销失败: {}", e);
                } else {
                    tokio::spawn(device_event::emit(DeviceEvent::Unregistered));
                }
                SipPlugin::shutdown();
                break;
            }

            _ = heartbeat_ticker.tick() => {
                if let Err(e) = send_keepalive_inner(&inner).await {
                    warn!("心跳失败: {}（将在下次重试）", e);
                }
            }

            _ = refresh_ticker.tick() => {
                info!("🔄 刷新注册...");
                if let Err(e) = do_register(&inner.config).await {
                    error!("刷新注册失败: {}（将在下次重试）", e);
                }
            }
        }
    }
}

// ── 注册/注销/心跳的原子操作 ──────────────────────────────────────────────────

async fn do_register(config: &Gb28181DeviceConfig) -> anyhow::Result<()> {
    let sender = SipPlugin::sender();
    let resp = sender
        .register(
            &config.platform_uri(),
            &config.username,
            &config.password,
        )
        .await?;

    if resp.status_code == rsip::StatusCode::OK {
        info!(
            device_id = %config.device_id,
            platform = %config.platform_uri(),
            "✅ 注册成功"
        );
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "注册失败，服务器响应: {}",
            resp.status_code
        ))
    }
}

async fn do_unregister(config: &Gb28181DeviceConfig) -> anyhow::Result<()> {
    let sender = SipPlugin::sender();
    let inner = sender.inner();

    let credential = Credential {
        username: config.username.clone(),
        password: config.password.clone(),
        realm: Some(config.realm.clone()),
    };

    let server_uri = rsip::Uri::try_from(config.platform_uri().as_str())
        .map_err(|e| anyhow::anyhow!("无效的平台 URI: {}", e))?;

    let mut reg = Registration::new(inner, Some(credential));
    let resp = reg
        .register(server_uri, Some(0)) // Expires: 0 = 注销
        .await
        .map_err(|e| anyhow::anyhow!("注销失败: {}", e))?;

    info!(status = %resp.status_code, "注销响应");
    Ok(())
}

async fn send_keepalive_inner(inner: &Arc<DeviceInner>) -> anyhow::Result<()> {
    let sn = inner.sn.fetch_add(1, Ordering::Relaxed);
    let config = &inner.config;
    let xml = tx_di_gb28181::xml::build_keepalive_xml(&config.device_id, sn);
    send_manscdp(config, &xml, sn).await?;
    info!(sn = sn, "💓 心跳已发送");
    Ok(())
}

// ── 通用 MESSAGE 发送 ─────────────────────────────────────────────────────────

async fn send_manscdp(
    config: &Gb28181DeviceConfig,
    body: &str,
    seq: u32,
) -> anyhow::Result<()> {
    let sender = SipPlugin::sender();
    let inner = sender.inner();

    let req_uri = rsip::Uri::try_from(config.platform_uri().as_str())
        .map_err(|e| anyhow::anyhow!("无效的平台 URI: {}", e))?;

    let from_str = format!("sip:{}@{}:{}", config.device_id, config.local_ip, config.local_port);
    let from_uri = rsip::Uri::try_from(from_str.as_str())
        .map_err(|e| anyhow::anyhow!("无效的 From URI: {}", e))?;

    let via = inner
        .get_via(None, None)
        .map_err(|e| anyhow::anyhow!("获取 Via 头失败: {}", e))?;

    let from = rsip::typed::From {
        display_name: None,
        uri: from_uri.into(),
        params: vec![rsip::Param::Tag(rsip::uri::Tag::new(
            rsipstack::transaction::make_tag(),
        ))],
    };
    let to = rsip::typed::To {
        display_name: None,
        uri: req_uri.clone().into(),
        params: vec![],
    };

    let mut request = inner.make_request(
        rsip::method::Method::Message,
        req_uri,
        via,
        from,
        to,
        seq,
        None,
    );
    request
        .headers
        .push(rsip::Header::ContentType("Application/MANSCDP+xml".into()));
    request.body = body.as_bytes().to_vec().into();

    let key = TransactionKey::from_request(&request, TransactionRole::Client)
        .map_err(|e| anyhow::anyhow!("构造事务 key 失败: {}", e))?;

    let mut tx = Transaction::new_client(key, request, inner, None);
    tx.send()
        .await
        .map_err(|e| anyhow::anyhow!("发送 MESSAGE 失败: {}", e))?;

    Ok(())
}

// ── 操作句柄 ──────────────────────────────────────────────────────────────────

/// 设备操作句柄（通过 `Gb28181Device::handle()` 获取）
#[derive(Clone)]
pub struct Gb28181DeviceHandle {
    inner: Arc<DeviceInner>,
}

impl Gb28181DeviceHandle {
    /// 手动发送一次心跳
    pub async fn send_keepalive(&self) -> anyhow::Result<()> {
        send_keepalive_inner(&self.inner).await
    }

    /// 手动触发目录响应（主动上报）
    pub async fn send_catalog(&self, sn: u32) -> anyhow::Result<()> {
        let ch_pairs: Vec<(String, String)> = self
            .inner
            .channels
            .iter()
            .map(|c| (c.channel_id.clone(), c.name.clone()))
            .collect();

        let xml = tx_di_gb28181::xml::build_catalog_response_xml(
            &self.inner.config.device_id,
            sn,
            &ch_pairs,
        );
        send_manscdp(&self.inner.config, &xml, sn).await
    }

    /// 获取设备 ID
    pub fn device_id(&self) -> &str {
        &self.inner.config.device_id
    }

    /// 获取平台 URI
    pub fn platform_uri(&self) -> String {
        self.inner.config.platform_uri()
    }
}
