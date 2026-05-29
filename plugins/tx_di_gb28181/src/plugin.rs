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
#[allow(unused_imports)]
use tx_gb28181::device::GbDevice;
use crate::event::{self, Gb28181Event};
use crate::handlers::{NonceStore, register_server_handlers};
#[allow(unused_imports)]
use crate::media::{MediaBackend, OpenRtpRequest, PlayUrls, build_backend};
#[allow(unused_imports)]
use crate::sdp::{
    AudioCodec, SessionType, build_audio_invite_sdp, build_invite_sdp, build_snapshot_sdp,
    parse_audio_sdp, parse_snapshot_sdp,
};
#[allow(unused_imports)]
use crate::xml::{
    ConfigType, GuardMode, PlaybackControl, PtzCommand, PtzPreciseParam, ZoomRect,
    build_alarm_reset_xml, build_alarm_subscribe_xml, build_broadcast_cancel_xml,
    build_broadcast_invite_xml, build_catalog_query_xml, build_config_download_query_xml,
    build_cruise_list_query_xml, build_cruise_start_xml, build_cruise_stop_xml,
    build_cruise_track_query_xml, build_device_info_query_xml, build_device_status_query_xml,
    build_guard_control_xml, build_guard_control_xml_v2, build_guard_info_query_xml,
    build_make_video_record_xml, build_playback_control_xml, build_preset_goto_xml,
    build_preset_list_query_xml, build_preset_set_xml, build_ptz_control_xml,
    build_ptz_precise_status_query_xml, build_ptz_precise_xml, build_record_control_xml,
    build_record_info_query_xml, build_storage_format_xml, build_storage_status_query_xml,
    build_target_track_xml, build_teleboot_xml, build_time_sync_query_xml,
    build_time_sync_response_xml, build_zoom_in_xml, build_zoom_out_xml,
};
use dashmap::DashMap;
use std::future::Future;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, OnceLock};
use tokio::time::{Duration, interval};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tx_di_core::{App, BoxFuture, CompInit, RIE, tx_comp};
use tx_di_sip::SipPlugin;

/// 活跃媒体会话信息
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub call_id: String,
    pub device_id: String,
    pub channel_id: String,
    pub rtp_port: u16,
    pub ssrc: String,
    pub stream_id: String,
    pub is_realtime: bool,
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
#[tx_comp(init)]
pub struct Gb28181Server {
    /// 配置
    pub config: Arc<Gb28181ServerConfig>,
    /// 设备注册表
    #[tx_cst(DeviceRegistry::new())]
    pub device_registry: DeviceRegistry,
    /// 随机数存储
    #[tx_cst(NonceStore::new())]
    pub nonce_store: NonceStore,
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

impl CompInit for Gb28181Server {
    fn async_init(ctx: Arc<App>, _token: CancellationToken) -> BoxFuture {
        Box::pin(async move {
            let srv = ctx.inject::<Gb28181Server>();

            let config = srv.config.clone();
            let registry = srv.device_registry.clone();
            let nonce_store = srv.nonce_store.clone();
            let sip_plugin = srv.sip_plugin.clone();
            // 构建流媒体后端
            let media_backend = build_backend(&config.media_backend);

            // 注册 SIP 消息处理器
            register_server_handlers(sip_plugin.clone(), registry.clone(), config.clone(), nonce_store)?;

            // 存储 media backend
            let _ = srv.media.set(media_backend.clone());

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
        })
    }

    fn init_sort() -> i32 {
        // 在 SipPlugin（MAX-1）之后初始化
        10001
    }
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


