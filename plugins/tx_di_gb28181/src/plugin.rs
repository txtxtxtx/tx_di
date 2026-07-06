//! GB28181 服务端插件主体
//!
//! `Gb28181Server` 是整个服务端的门面，集成到 tx-di 框架中：
//! - 持有全局 `DeviceRegistry`
//! - 提供主动查询（目录、设备信息、设备状态、录像查询）
//! - 提供主动点播（INVITE 实时/历史回放）
//! - 提供 PTZ 云台控制、设备控制
//! - 通过统一 `MediaBackend` trait 接入流媒体服务（ZLM / MediaMTX / 自定义）
//! - 提供事件订阅接口
//! - 后台运行心跳超时检测

use crate::config::Gb28181ServerConfig;
use crate::device_registry::DeviceRegistry;
use tx_gb28181::device::GbDevice;
use crate::event::{self, Gb28181Event};
use crate::handlers::register_server_handlers;
use crate::media::{MediaBackend, build_backend};
use dashmap::DashMap;
use std::future::Future;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, OnceLock};
use tokio::time::{Duration, interval};
use tracing::{error, info, warn};
use rsipstack::dialog::client_dialog::ClientInviteDialog;
use tx_di_core::{App, Component, DepsTuple, RIE};
use tx_di_sip::SipPlugin;

/// 活跃媒体会话信息
#[derive(Clone)]
pub struct SessionInfo {
    pub call_id: String,
    pub device_id: String,
    pub channel_id: String,
    pub rtp_port: u16,
    pub ssrc: String,
    pub stream_id: String,
    pub is_realtime: bool,
    /// SIP 对话句柄（UAC INVITE 创建），用于主动发送 BYE 挂断
    pub dialog: ClientInviteDialog,
}

impl std::fmt::Debug for SessionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionInfo")
            .field("call_id", &self.call_id)
            .field("device_id", &self.device_id)
            .field("channel_id", &self.channel_id)
            .field("rtp_port", &self.rtp_port)
            .field("ssrc", &self.ssrc)
            .field("stream_id", &self.stream_id)
            .field("is_realtime", &self.is_realtime)
            .field("dialog", &"<ClientInviteDialog>")
            .finish()
    }
}

/// GB28181 服务端插件
///
/// 通过 DI 框架管理，使用 `DiComp<Gb28181Server>` 在 axum handler 中注入。
///
/// # 配置文件（TOML）
/// ```toml
/// [gb28181_server_config]
/// platform_id            = "34020000002000000001"
/// realm                  = "3402000000"
/// sip_ip                 = "192.168.1.100"
/// heartbeat_timeout_secs = 120
/// enable_auth            = true
/// auth_password          = "12345678"
///
/// [gb28181_server_config.media]
/// local_ip       = "192.168.1.100"
/// rtp_port_start = 10000
/// rtp_port_end   = 20000
///
/// [gb28181_server_config.zlm]
/// base_url = "http://127.0.0.1:8080"
/// secret   = "035c73f7-bb6b-4889-a715-d9eb2d1925cc"
/// ```
#[derive(Component)]
#[component(app_async_init, init_sort = 10001)]
pub struct Gb28181Server {
    /// 配置
    pub config: Arc<Gb28181ServerConfig>,
    /// 设备注册表
    #[tx_cst(DeviceRegistry::new())]
    pub device_registry: DeviceRegistry,
    /// sip 插件
    pub sip_plugin: Arc<SipPlugin>,
    /// 每设备独立 SN 序列号
    #[tx_cst(DashMap::new())]
    pub sn_map: DashMap<String, AtomicU32>,
    /// 活跃媒体会话表（call_id → SessionInfo）
    #[tx_cst(DashMap::new())]
    pub sessions: DashMap<String, SessionInfo>,
    /// 活跃语音广播会话表（device_id → audio_port）
    #[tx_cst(DashMap::new())]
    pub broadcast_sessions: DashMap<String, u16>,
    /// 统一流媒体后端（在 async_init 中初始化）
    #[tx_cst(OnceLock::new())]
    pub media: OnceLock<Arc<dyn MediaBackend>>,
}

/// 应用异步初始化：注册 SIP 处理器、构建流媒体后端、启动级联与心跳检测
async fn app_async_init(comp: Arc<Gb28181Server>, _app: Arc<App>) -> RIE<()> {
        let config = comp.config.clone();
        let registry = comp.device_registry.clone();
        let sip_plugin = comp.sip_plugin.clone();

        // 构建流媒体后端
        let media_backend = build_backend(&config.media_backend);

        // 注册 SIP 消息处理器
        register_server_handlers(comp.clone())?;

        // 存储 media backend
        let _ = comp.media.set(media_backend.clone());

        // ── 级联：下级平台模式（向上级注册）────────────────────────────────
        if config.cascade.enable_lower {
            if let Some(cascade_lower) = crate::cascade::CascadeLower::new(
                &config.cascade,
                &config.platform_id,
                &config.sip_ip,
                sip_plugin.clone(),
                registry.clone(),
            ) {
                let cancel_token = sip_plugin.get_cancel_token()?;
                cascade_lower.start(cancel_token);
                info!("下级平台级联任务已启动");
            } else {
                warn!("enabled_lower=true 但缺少 upper_platform_sip 或 upper_platform_id 配置");
            }
        }

        info!(
            platform_id = %config.platform_id,
            realm = %config.realm,
            sip_ip = %config.sip_ip,
            heartbeat_timeout_secs = config.heartbeat_timeout_secs,
            enable_auth = config.enable_auth,
            media_backend = media_backend.backend_name(),
            "GB28181 服务端处理器注册完成"
        );

        // 启动心跳超时检测后台任务
        let timeout_secs = config.heartbeat_timeout_secs;
        let registry_clone = registry.clone();
        let sip_plugin_clone = sip_plugin.clone();
        tokio::spawn(async move {
            if let Err(e) = heartbeat_watchdog(registry_clone, sip_plugin_clone, timeout_secs).await {
                error!("心跳检测任务出错：{}", e);
            }
        });
        Ok(())
}

impl Gb28181Server {
    /// 订阅 GB28181 事件
    ///
    /// 必须在 `ctx.build().await` 之前调用。
    pub fn on_event<F, Fut>(handler: F)
    where
        F: Fn(Gb28181Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        event::add_event_listener(handler);
    }

    /// 从数据库恢复设备注册状态（服务启动时调用）
    ///
    /// 恢复的设备默认标记为离线（`online = false`），
    /// 等设备重新 REGISTER 或发送心跳时会自动上线。
    ///
    /// # 使用场景
    /// Admin 示例在 `async_init` 中从 toasty DB 加载 `online=true` 的设备记录，
    /// 调用此方法恢复到内存注册表。
    pub fn restore_devices(app: &App, devices: Vec<GbDevice>) {
        let srv = app.inject::<Gb28181Server>();
        srv.device_registry.restore_batch(devices);
    }
}

// ── 心跳超时检测后台任务 ──────────────────────────────────────────────────────

async fn heartbeat_watchdog(
    registry: DeviceRegistry,
    sip_plugin: Arc<SipPlugin>,
    timeout_secs: u64,
) -> RIE<()> {
    let check_interval = Duration::from_secs(timeout_secs.min(30));
    let mut ticker = interval(check_interval);
    ticker.tick().await;

    let cancel_token = sip_plugin.get_cancel_token()?;

    loop {
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                info!("GB28181 心跳检测收到停止信号");
                return Ok(());
            }
            _ = ticker.tick() => {
                let timeout_devices = registry.check_timeouts(timeout_secs);
                for device_id in timeout_devices {
                    warn!(device_id = %device_id, "⚠️ 设备心跳超时，标记离线");
                    registry.set_offline(&device_id);
                    tokio::spawn(event::emit(Gb28181Event::DeviceOffline {
                        device_id: device_id.clone(),
                    }));
                }
            },
        }
    }
}


