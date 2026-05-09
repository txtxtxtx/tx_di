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
use crate::device_registry::{DeviceInfo, DeviceRegistry};
use crate::event::{self, Gb28181Event};
use crate::handlers::{NonceStore, register_server_handlers};
use crate::media::{MediaBackend, OpenRtpRequest, PlayUrls, build_backend};
use crate::sdp::{
    AudioCodec, SessionType, build_audio_invite_sdp, build_invite_sdp, build_snapshot_sdp,
    parse_audio_sdp, parse_snapshot_sdp,
};
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
use rsipstack::dialog::dialog::DialogState;
use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::dialog::invitation::InviteOption;
use rsipstack::sip as rsip;
use rsipstack::transaction::key::{TransactionKey, TransactionRole};
use rsipstack::transaction::transaction::Transaction;
use std::future::Future;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::time::{Duration, interval};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tx_di_core::{App, BoxFuture, CompInit, RIE, tx_comp, IE};
use tx_di_sip::SipPlugin;

/// 全局服务实例
static INSTANCE: OnceLock<Arc<Gb28181ServerInner>> = OnceLock::new();

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

/// 语音广播会话信息
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BroadcastSessionInfo {
    pub device_id: String,
    pub source_id: String,
    pub audio_port: u16,
    pub codec: AudioCodec,
}

pub(crate) struct Gb28181ServerInner {
    pub(crate) config: Arc<Gb28181ServerConfig>,
    pub(crate) registry: DeviceRegistry,
    pub(crate) sn: AtomicU32,
    /// 活跃媒体会话表（call_id → SessionInfo）
    pub(crate) sessions: DashMap<String, SessionInfo>,
    /// 活跃语音广播会话表（device_id → BroadcastSessionInfo）
    pub(crate) broadcast_sessions: DashMap<String, BroadcastSessionInfo>,
    /// 统一流媒体后端（ZLM / MediaMTX / Null）
    pub(crate) media: Arc<dyn MediaBackend>,
}

/// GB28181 服务端插件
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
    pub config: Arc<Gb28181ServerConfig>,
}

impl CompInit for Gb28181Server {
    fn async_init(ctx: Arc<App>, _token: CancellationToken) -> BoxFuture<'static, RIE<()>> {
        Box::pin(async move {
            let config = ctx.inject::<Gb28181ServerConfig>();

            // 创建全局注册表
            let registry = DeviceRegistry::new();
            let nonce_store = Arc::new(NonceStore::new());

            // 构建流媒体后端（优先 media_backend，兼容旧版 zlm 配置）
            let media_backend = build_backend(&config.media_backend);

            // 注册 SIP 消息处理器
            register_server_handlers(Arc::new(registry.clone()), config.clone(), nonce_store);

            info!(
                platform_id = %config.platform_id,
                heartbeat_timeout_secs = config.heartbeat_timeout_secs,
                enable_auth = config.enable_auth,
                media_backend = media_backend.backend_name(),
                "✅ GB28181 服务端处理器注册完成"
            );

            // 存储全局实例
            let inner = Arc::new(Gb28181ServerInner {
                config: config.clone(),
                registry,
                sn: AtomicU32::new(1),
                sessions: DashMap::new(),
                broadcast_sessions: DashMap::new(),
                media: media_backend,
            });
            let _ = INSTANCE.set(inner.clone());

            // 启动心跳超时检测后台任务
            let timeout_secs = config.heartbeat_timeout_secs;
            tokio::spawn(async move {
                if let Err(e) = heartbeat_watchdog(inner, timeout_secs).await{
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
    // ── 门面方法 ────────────────────────────────────────────────────────────

    /// 获取全局实例（需在 build().await 完成后调用）
    pub fn instance() -> Gb28181ServerHandle {
        let inner = INSTANCE
            .get()
            .expect("Gb28181Server 尚未初始化，请确保 ctx.build().await 已完成")
            .clone();
        Gb28181ServerHandle { inner }
    }

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
}

// ── 心跳超时检测后台任务 ──────────────────────────────────────────────────────

async fn heartbeat_watchdog(inner: Arc<Gb28181ServerInner>, timeout_secs: u64) ->RIE<()> {
    // 每 30 秒检查一次
    let check_interval = Duration::from_secs(timeout_secs.min(30));
    let mut ticker = interval(check_interval);
    ticker.tick().await; // 跳过立即触发的第一次
    
    // 获取 SIP 插件的取消令牌
    let cancel_token = SipPlugin::cancel_token()
        .ok_or_else(|| IE::Other("SIP 插件未初始化，无法获取取消令牌".to_string()))?;
    
    loop {
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                info!("GB28181 心跳检测收到停止信号");
                return Ok(());
            }
            _ = ticker.tick() => {
                let timeout_devices = inner.registry.check_timeouts(timeout_secs);
                for device_id in timeout_devices {
                    warn!(device_id = %device_id, "⚠️ 设备心跳超时，标记离线");
                    inner.registry.set_offline(&device_id);
                    tokio::spawn(event::emit(Gb28181Event::DeviceOffline {
                        device_id: device_id.clone(),
                    }));
                }
            },
        }
    }
}

// ── 操作句柄 ──────────────────────────────────────────────────────────────────

/// GB28181 服务端操作句柄（通过 `Gb28181Server::instance()` 获取）
#[derive(Clone)]
pub struct Gb28181ServerHandle {
    pub(crate) inner: Arc<Gb28181ServerInner>,
}

impl Gb28181ServerHandle {
    // ── 注册表查询 ───────────────────────────────────────────────────────────

    /// 获取设备信息
    pub fn get_device(&self, device_id: &str) -> Option<DeviceInfo> {
        self.inner.registry.get(device_id)
    }

    /// 获取所有在线设备
    pub fn online_devices(&self) -> Vec<DeviceInfo> {
        self.inner.registry.online_devices()
    }

    /// 获取注册设备总数
    pub fn device_count(&self) -> usize {
        self.inner.registry.total_count()
    }

    /// 获取在线设备数
    pub fn online_count(&self) -> usize {
        self.inner.registry.online_count()
    }

    /// 获取设备下所有通道
    pub fn get_channels(&self, device_id: &str) -> Vec<crate::device_registry::ChannelInfo> {
        self.inner
            .registry
            .get(device_id)
            .map(|d| d.channels)
            .unwrap_or_default()
    }

    // ── 主动查询 ─────────────────────────────────────────────────────────────

    /// 向设备发送目录查询（MESSAGE Catalog）
    ///
    /// 设备收到后会回复包含通道列表的 MESSAGE，触发 `Gb28181Event::CatalogReceived`。
    pub async fn query_catalog(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_catalog_query_xml(&self.inner.config.platform_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, sn = sn, "📂 发送目录查询");
        Ok(())
    }

    /// 向设备发送设备信息查询（MESSAGE DeviceInfo）
    pub async fn query_device_info(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_device_info_query_xml(&self.inner.config.platform_id, device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, "ℹ️ 发送设备信息查询");
        Ok(())
    }

    /// 向设备发送设备状态查询（MESSAGE DeviceStatus）
    pub async fn query_device_status(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_device_status_query_xml(device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, "📊 发送设备状态查询");
        Ok(())
    }

    /// 向设备发送校时查询（QUERY TimeRequest）
    ///
    /// GB28181-2022 §9.10：平台向设备查询当前时间
    /// 设备响应后触发 `Gb28181Event::TimeSyncResult`。
    ///
    /// # 返回
    /// 设备返回的 `TimeSyncInfo`，包含设备时间和时间差
    pub async fn time_sync(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_time_sync_query_xml(&self.inner.config.platform_id, device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, sn = sn, "🕐 发送校时查询");
        Ok(())
    }

    /// 向设备主动下发标准时间（Response 模式）
    ///
    /// GB28181-2022 §9.10：平台向设备下发当前标准时间
    /// 设备应回复确认。
    pub async fn sync_time_to_device(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_time_sync_response_xml(device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, sn = sn, "🕐 下发校时到设备");
        Ok(())
    }

    // ── 配置/预置位查询 ───────────────────────────────────────────────────────

    /// 向设备发送设备配置查询
    ///
    /// GB28181-2022 A.2.4.7：ConfigDownload
    ///
    /// `config_type` 支持：`ConfigType::Basic`(基本参数) / `ConfigType::Network`(网络) / `ConfigType::Video`(视频)
    ///
    /// 设备回复后触发 `Gb28181Event::ConfigDownloaded`。
    pub async fn query_config(
        &self,
        device_id: &str,
        config_type: ConfigType,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_config_download_query_xml(device_id, sn, config_type);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, config_type = ?config_type, "⚙️ 发送设备配置查询");
        Ok(())
    }

    /// 向设备发送预置位列表查询
    ///
    /// GB28181-2022 A.2.4.8：PresetList
    /// 设备回复后触发 `Gb28181Event::PresetListReceived`。
    pub async fn query_preset_list(&self, device_id: &str, channel_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_preset_list_query_xml(channel_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, "📍 发送预置位列表查询");
        Ok(())
    }

    /// 向设备发送巡航轨迹列表查询
    ///
    /// GB28181-2022 A.2.4.11：CruiseList
    /// 设备回复后触发 `Gb28181Event::CruiseListReceived`。
    pub async fn query_cruise_list(&self, device_id: &str, channel_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_cruise_list_query_xml(channel_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, "🔄 发送巡航轨迹列表查询");
        Ok(())
    }

    /// 向设备发送看守位信息查询（2022 新增）
    ///
    /// GB28181-2022 A.2.4.10：GuardInfo
    pub async fn query_guard_info(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_guard_info_query_xml(device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, "🛡️ 发送看守位信息查询");
        Ok(())
    }

    // ── 预置位/巡航控制 ───────────────────────────────────────────────────────

    /// 调用预置位
    ///
    /// GB28181-2022 A.2.3.1.10：GotoPreset
    pub async fn goto_preset(
        &self,
        device_id: &str,
        channel_id: &str,
        preset_index: u8,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_preset_goto_xml(channel_id, sn, preset_index);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, preset = preset_index, "📍 调用预置位");
        Ok(())
    }

    /// 设置预置位
    ///
    /// GB28181-2022 A.2.3.1.10：SetPreset
    pub async fn set_preset(
        &self,
        device_id: &str,
        channel_id: &str,
        preset_index: u8,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_preset_set_xml(channel_id, sn, preset_index);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, preset = preset_index, "📍 设置预置位");
        Ok(())
    }

    /// 启动巡航轨迹
    ///
    /// GB28181-2022 A.2.3.1.10：巡航控制
    pub async fn start_cruise(
        &self,
        device_id: &str,
        channel_id: &str,
        cruise_no: u8,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_cruise_start_xml(channel_id, sn, cruise_no);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, cruise = cruise_no, "🔄 启动巡航");
        Ok(())
    }

    /// 停止巡航轨迹
    pub async fn stop_cruise(
        &self,
        device_id: &str,
        channel_id: &str,
        cruise_no: u8,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_cruise_stop_xml(channel_id, sn, cruise_no);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, cruise = cruise_no, "🔄 停止巡航");
        Ok(())
    }

    /// 向设备查询录像文件列表
    ///
    /// # 参数
    /// - `channel_id`：通道 ID
    /// - `start_time`：开始时间（ISO8601，如 "2024-01-01T00:00:00"）
    /// - `end_time`：结束时间
    /// - `record_type`：录像类型（0=全部，1=定时，2=报警，3=手动）
    ///
    /// 设备回复后触发 `Gb28181Event::RecordInfoReceived`。
    pub async fn query_record_info(
        &self,
        device_id: &str,
        channel_id: &str,
        start_time: &str,
        end_time: &str,
        record_type: u8,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_record_info_query_xml(
            device_id,
            channel_id,
            sn,
            start_time,
            end_time,
            record_type,
            "",
        );
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            start = %start_time,
            end = %end_time,
            "📼 发送录像查询"
        );
        Ok(())
    }

    // ── 点播控制 ─────────────────────────────────────────────────────────────

    /// 向设备发起实时点播（INVITE）
    ///
    /// 自动通过 ZLM API 申请 RTP 端口，INVITE 成功后触发 `Gb28181Event::SessionStarted`。
    ///
    /// # 返回
    /// `(call_id, play_urls)` — call_id 用于 BYE，play_urls 是各协议播放地址
    pub async fn invite(
        &self,
        device_id: &str,
        channel_id: &str,
    ) -> anyhow::Result<(String, PlayUrls)> {
        self.invite_internal(device_id, channel_id, true, None, None)
            .await
    }

    /// 向设备发起历史回放（INVITE s=Playback）
    ///
    /// # 参数
    /// - `start_time`：回放开始时间（ISO8601）
    /// - `end_time`：回放结束时间（ISO8601）
    pub async fn invite_playback(
        &self,
        device_id: &str,
        channel_id: &str,
        start_time: &str,
        end_time: &str,
    ) -> anyhow::Result<(String, PlayUrls)> {
        self.invite_internal(
            device_id,
            channel_id,
            false,
            Some(start_time.to_string()),
            Some(end_time.to_string()),
        )
        .await
    }

    /// 挂断通话（发送 BYE，并释放 ZLM RTP 端口）
    ///
    /// GB28181-2022 §9.1.4
    pub async fn hangup(&self, call_id: &str) -> anyhow::Result<()> {
        let session = self.inner.sessions.get(call_id).map(|r| r.value().clone());

        if let Some(sess) = session {
            // 释放 RTP 端口
            let stream_id = sess.stream_id.clone();
            if let Err(e) = self.inner.media.close_rtp_server(&stream_id).await {
                warn!(call_id = %call_id, error = %e, "关闭 RTP 端口失败（忽略）");
            }

            // 从会话表中移除
            self.inner.sessions.remove(call_id);

            info!(call_id = %call_id, "📴 主动挂断");

            // 触发会话结束事件
            tokio::spawn(event::emit(Gb28181Event::SessionEnded {
                device_id: sess.device_id,
                channel_id: sess.channel_id,
                call_id: call_id.to_string(),
            }));
        } else {
            warn!(call_id = %call_id, "BYE：未找到对应会话");
        }

        Ok(())
    }

    /// 获取活跃会话列表
    pub fn active_sessions(&self) -> Vec<SessionInfo> {
        self.inner
            .sessions
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    // ── 图像抓拍 ─────────────────────────────────────────────────────────────

    /// 向设备发起图像抓拍（INVITE s=SnapShot）
    ///
    /// GB28181-2022 §9.14：平台向设备请求抓拍
    ///
    /// 流程：INVITE → 200 OK（SDP 含图片URL）→ ACK → BYE → 下载图片
    ///
    /// # 参数
    /// - `device_id`：设备 ID
    /// - `channel_id`：通道 ID
    ///
    /// # 返回
    /// 抓拍图片的 URL 列表（从设备 SDP 中解析）
    pub async fn snapshot(&self, device_id: &str, channel_id: &str) -> anyhow::Result<String> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.inner.sn.fetch_add(1, Ordering::Relaxed);
        let media_ip = if is_unspecified_ip(&self.inner.config.media.local_ip) {
            self.inner.config.sip_ip.clone()
        } else {
            self.inner.config.media.local_ip.clone()
        };

        let stream_id = format!("snapshot_{}_{}", channel_id, sn);
        let sdp_offer = build_snapshot_sdp(&media_ip, sn);

        let platform_id = &self.inner.config.platform_id;
        let sip_ip = &self.inner.config.sip_ip;

        let caller_str = format!("sip:{}@{}", platform_id, sip_ip);
        let callee_str = dev.contact.clone();

        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            sn = sn,
            "📸 发起抓拍 INVITE"
        );

        let sender = SipPlugin::sender();
        let endpoint = sender.inner();
        let dialog_layer = Arc::new(DialogLayer::new(endpoint.clone()));
        let (state_tx, mut state_rx) = dialog_layer.new_dialog_state_channel();

        let caller_uri = rsip::Uri::try_from(caller_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的主叫 URI: {}", e))?;
        let callee_uri = rsip::Uri::try_from(callee_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的被叫 URI: {}", e))?;

        let invite_option = InviteOption {
            caller: caller_uri.clone(),
            callee: callee_uri,
            contact: caller_uri,
            content_type: Some("application/sdp".to_string()),
            offer: Some(sdp_offer.into_bytes().into()),
            credential: None,
            ..Default::default()
        };

        let (dialog, resp) = dialog_layer
            .do_invite(invite_option, state_tx)
            .await
            .map_err(|e| anyhow::anyhow!("抓拍 INVITE 失败: {}", e))?;

        // 解析 200 OK 中的 SDP，获取图片 URL
        let call_id = dialog.id().call_id.clone();
        let image_url = if let Some(response) = resp {
            let body = std::str::from_utf8(&response.body)
                .unwrap_or_default()
                .to_string();
            if !body.is_empty() {
                let info = parse_snapshot_sdp(&body);
                info!(
                    device_id = %device_id,
                    channel_id = %channel_id,
                    image_url = %info.image_url,
                    "📸 收到抓拍响应，图片URL"
                );
                info.image_url
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // 发送 ACK 并等待 BYE（后台监听）
        let inner_clone = self.inner.clone();
        let call_id_clone = call_id.clone();
        let _device_id_owned = device_id.to_string();
        let _channel_id_owned = channel_id.to_string();

        tokio::spawn(async move {
            // 监听对话状态，等待 BYE
            while let Some(state) = state_rx.recv().await {
                if matches!(state, DialogState::Terminated(_, _)) {
                    info!(
                        call_id = %call_id_clone,
                        "📸 抓拍会话结束"
                    );
                    // 释放 stream
                    let _ = inner_clone.media.close_rtp_server(&stream_id).await;
                    break;
                }
            }
        });

        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            "📸 抓拍请求完成"
        );

        // 发射抓拍完成事件
        if !image_url.is_empty() {
            tokio::spawn(event::emit(Gb28181Event::SnapshotTaken {
                device_id: device_id.to_string(),
                channel_id: channel_id.to_string(),
                image_url: image_url.clone(),
            }));
        }

        Ok(image_url)
    }

    // ── 语音广播 ─────────────────────────────────────────────────────────────

    /// 向设备发起语音广播邀请
    ///
    /// GB28181-2022 §9.12：平台向设备发起语音广播
    /// 设备收到后会向平台推送音频流。
    pub async fn broadcast_invite(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_broadcast_invite_xml(&self.inner.config.platform_id, device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, sn = sn, "📢 发起语音广播邀请");
        Ok(())
    }

    /// 确认广播会话（平台接收音频）
    ///
    /// 当收到 `BroadcastInviteReceived` 事件后，平台可调用此方法确认接收。
    /// 需要传入音频端口，设备会将音频推送到此端口。
    pub async fn broadcast_accept(&self, device_id: &str, audio_port: u16) -> anyhow::Result<()> {
        // 记录广播会话
        let session = BroadcastSessionInfo {
            device_id: device_id.to_string(),
            source_id: self.inner.config.platform_id.clone(),
            audio_port,
            codec: AudioCodec::PCMU,
        };
        self.inner
            .broadcast_sessions
            .insert(device_id.to_string(), session.clone());

        // 通过 MESSAGE 回复确认（含音频端口）
        let sn = self.next_sn();
        let media_ip = if is_unspecified_ip(&self.inner.config.media.local_ip) {
            self.inner.config.sip_ip.clone()
        } else {
            self.inner.config.media.local_ip.clone()
        };

        // 构建广播确认 XML（200 OK body）
        let ack_xml = format!(
            "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
             <Response>\r\n\
             <CmdType>Broadcast</CmdType>\r\n\
             <SN>{sn}</SN>\r\n\
             <DeviceID>{device_id}</DeviceID>\r\n\
             <Result>OK</Result>\r\n\
             <AudioPort>{audio_port}</AudioPort>\r\n\
             <AudioCodec>PCMU</AudioCodec>\r\n\
             <IP>{media_ip}</IP>\r\n\
             </Response>",
            sn = sn,
            device_id = device_id,
            audio_port = audio_port,
            media_ip = media_ip
        );
        self.send_message_to_device(&self.get_dev_or_err(device_id)?.contact, &ack_xml, sn)
            .await?;

        info!(
            device_id = %device_id,
            audio_port = audio_port,
            "📢 确认广播接收，监听端口"
        );

        tokio::spawn(event::emit(Gb28181Event::BroadcastSessionStarted {
            device_id: device_id.to_string(),
            audio_port,
        }));

        Ok(())
    }

    /// 结束语音广播
    pub async fn broadcast_stop(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_broadcast_cancel_xml(&self.inner.config.platform_id, device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;

        // 清除广播会话
        self.inner.broadcast_sessions.remove(device_id);

        info!(device_id = %device_id, "📢 结束语音广播");

        tokio::spawn(event::emit(Gb28181Event::BroadcastSessionEnded {
            device_id: device_id.to_string(),
        }));

        Ok(())
    }

    // ── 语音对讲 ─────────────────────────────────────────────────────────────

    /// 向设备发起带音频的对讲 INVITE
    ///
    /// GB28181-2022 §9.12：平台向设备发起双向对讲
    /// SDP 中同时包含视频和音频，a=sendonly 表示平台向设备发送音频。
    ///
    /// # 参数
    /// - `device_id`：设备 ID
    /// - `channel_id`：通道 ID
    /// - `audio_port`：平台发送音频的 RTP 端口
    /// - `codec`：音频编码（默认 PCMU）
    ///
    /// # 返回
    /// `(call_id, device_ip, device_audio_port)`
    pub async fn audio_talkback(
        &self,
        device_id: &str,
        channel_id: &str,
        audio_port: u16,
        codec: Option<AudioCodec>,
    ) -> anyhow::Result<(String, String, u16)> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.inner.sn.fetch_add(1, Ordering::Relaxed);
        let media_ip = if is_unspecified_ip(&self.inner.config.media.local_ip) {
            self.inner.config.sip_ip.clone()
        } else {
            self.inner.config.media.local_ip.clone()
        };

        // 申请视频 RTP 端口
        let stream_id = format!("talkback_{}_{}", channel_id, sn);
        let handle = self
            .inner
            .media
            .open_rtp_server(OpenRtpRequest::udp(&stream_id))
            .await?;
        let video_port = handle.port;
        let ssrc = format!("{:010}", sn);

        let sdp_offer = build_audio_invite_sdp(
            &media_ip,
            video_port,
            audio_port,
            codec.unwrap_or(AudioCodec::PCMU),
            &ssrc,
        );

        let platform_id = &self.inner.config.platform_id;
        let sip_ip = &self.inner.config.sip_ip;
        let caller_str = format!("sip:{}@{}", platform_id, sip_ip);
        let callee_str = dev.contact.clone();

        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            audio_port = audio_port,
            "🎤 发起对讲 INVITE"
        );

        let sender = SipPlugin::sender();
        let endpoint = sender.inner();
        let dialog_layer = Arc::new(DialogLayer::new(endpoint.clone()));
        let (state_tx, mut state_rx) = dialog_layer.new_dialog_state_channel();

        let caller_uri = rsip::Uri::try_from(caller_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的主叫 URI: {}", e))?;
        let callee_uri = rsip::Uri::try_from(callee_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的被叫 URI: {}", e))?;

        let invite_option = InviteOption {
            caller: caller_uri.clone(),
            callee: callee_uri,
            contact: caller_uri,
            content_type: Some("application/sdp".to_string()),
            offer: Some(sdp_offer.into_bytes().into()),
            credential: None,
            ..Default::default()
        };

        let (dialog, resp) = dialog_layer
            .do_invite(invite_option, state_tx)
            .await
            .map_err(|e| anyhow::anyhow!("对讲 INVITE 失败: {}", e))?;

        let call_id = dialog.id().call_id.clone();

        // 解析设备响应 SDP，获取设备音频信息
        let (device_ip, device_audio_port) = if let Some(response) = resp {
            let body = std::str::from_utf8(&response.body).unwrap_or_default();
            if let Some(audio_info) = parse_audio_sdp(body) {
                (audio_info.device_ip, audio_info.device_port)
            } else {
                (String::new(), 0)
            }
        } else {
            (String::new(), 0)
        };

        // 记录会话
        let session = SessionInfo {
            call_id: call_id.clone(),
            device_id: device_id.to_string(),
            channel_id: channel_id.to_string(),
            rtp_port: video_port,
            ssrc: ssrc.clone(),
            stream_id: stream_id.clone(),
            is_realtime: true,
        };
        self.inner.sessions.insert(call_id.clone(), session);

        // 监听会话状态
        let inner_clone = self.inner.clone();
        let call_id_clone = call_id.clone();
        let device_id_owned = device_id.to_string();
        let channel_id_owned = channel_id.to_string();
        let device_ip_owned = device_ip.clone();
        let device_audio_port_owned = device_audio_port;

        tokio::spawn(async move {
            while let Some(state) = state_rx.recv().await {
                match state {
                    DialogState::Confirmed(id, _) => {
                        info!(call_id = %call_id_clone, dialog_id = %id, "🎤 对讲会话已确认");
                        tokio::spawn(event::emit(Gb28181Event::AudioTalkbackStarted {
                            device_id: device_id_owned.clone(),
                            channel_id: channel_id_owned.clone(),
                            call_id: call_id_clone.clone(),
                            device_ip: device_ip_owned.clone(),
                            device_port: device_audio_port_owned,
                        }));
                    }
                    DialogState::Terminated(id, _) => {
                        info!(call_id = %call_id_clone, dialog_id = %id, "🎤 对讲会话结束");
                        let _ = inner_clone.media.close_rtp_server(&stream_id).await;
                        inner_clone.sessions.remove(&call_id_clone);
                        tokio::spawn(event::emit(Gb28181Event::AudioTalkbackEnded {
                            device_id: device_id_owned.clone(),
                            call_id: call_id_clone.clone(),
                        }));
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok((call_id, device_ip, device_audio_port))
    }

    // ── PTZ 云台控制 ─────────────────────────────────────────────────────────

    /// 向设备发送 PTZ 控制指令
    ///
    /// GB28181-2022 §8.4：DeviceControl/PTZCmd
    ///
    /// # 示例
    /// ```rust,ignore
    /// use tx_di_gb28181::xml::{PtzCommand, PtzSpeed};
    /// let server = Gb28181Server::instance();
    /// // 向右转动，速度 64
    /// server.ptz_control("device_id", "channel_id",
    ///     PtzCommand::Right(PtzSpeed { pan: 64, tilt: 0, zoom: 0 })).await?;
    /// // 停止
    /// server.ptz_control("device_id", "channel_id", PtzCommand::Stop).await?;
    /// ```
    pub async fn ptz_control(
        &self,
        device_id: &str,
        channel_id: &str,
        cmd: PtzCommand,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_ptz_control_xml(device_id, channel_id, sn, &cmd);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, cmd = ?cmd, "🎮 PTZ 控制");
        Ok(())
    }

    /// 录像控制（开始/停止录像）
    pub async fn record_control(
        &self,
        device_id: &str,
        channel_id: &str,
        start: bool,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_record_control_xml(device_id, channel_id, sn, start);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, start = start, "🎬 录像控制");
        Ok(())
    }

    /// 布撤防控制（看守位设置）
    pub async fn guard_control(
        &self,
        device_id: &str,
        channel_id: &str,
        guard: bool,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_guard_control_xml(device_id, channel_id, sn, guard);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, guard = guard, "🔒 布撤防控制");
        Ok(())
    }

    // ── 扩展设备控制 ─────────────────────────────────────────────────────────

    /// 远程启动设备（唤醒休眠设备）
    ///
    /// GB28181-2022 A.2.3.1.3：远程启动
    pub async fn teleboot(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_teleboot_xml(device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, "🔌 发送远程启动命令");
        Ok(())
    }

    /// 报警复位
    ///
    /// GB28181-2022 A.2.3.1.6：报警复位
    ///
    /// # 参数
    /// - `alarm_type`：报警类型（"1"=紧急报警，"2"=模块故障等）
    pub async fn alarm_reset(&self, device_id: &str, alarm_type: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_alarm_reset_xml(device_id, sn, alarm_type);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, alarm_type = %alarm_type, "🔔 报警复位");
        Ok(())
    }

    /// 强制关键帧
    ///
    /// GB28181-2022 A.2.3.1.7：强制关键帧
    /// 请求设备立即生成一个 I 帧，改善视频传输质量
    pub async fn make_video_record(&self, device_id: &str, channel_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_make_video_record_xml(channel_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, "🎬 请求强制关键帧");
        Ok(())
    }

    /// 拉框放大
    ///
    /// GB28181-2022 A.2.3.1.8：拉框放大
    /// 指定矩形区域将被放大至全屏
    ///
    /// # 参数
    /// - `rect`：归一化坐标（0-65535），x1,y1 为左上角，x2,y2 为右下角
    pub async fn zoom_in(
        &self,
        device_id: &str,
        channel_id: &str,
        rect: ZoomRect,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_zoom_in_xml(channel_id, sn, &rect);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, "🔍 拉框放大");
        Ok(())
    }

    /// 拉框缩小
    ///
    /// GB28181-2022 A.2.3.1.9：拉框缩小
    pub async fn zoom_out(
        &self,
        device_id: &str,
        channel_id: &str,
        rect: ZoomRect,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_zoom_out_xml(channel_id, sn, &rect);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, "🔍 拉框缩小");
        Ok(())
    }

    /// PTZ 精准控制（绝对位置控制）
    ///
    /// GB28181-2022 A.2.3.1.11：PTZ 精准控制
    /// 使用绝对位置（0-10000）进行云台控制，而非相对速度
    ///
    /// # 示例
    /// ```rust,ignore
    /// use tx_di_gb28181::xml::PtzPreciseParam;
    /// let param = PtzPreciseParam {
    ///     pan_position: 5000,    // 水平居中
    ///     tilt_position: 5000,   // 垂直居中
    ///     zoom_position: 5000,   // 变倍居中
    ///     focus_position: None,
    ///     iris_position: None,
    /// };
    /// server.ptz_precise_control("device_id", "channel_id", param).await?;
    /// ```
    pub async fn ptz_precise_control(
        &self,
        device_id: &str,
        channel_id: &str,
        param: PtzPreciseParam,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_ptz_precise_xml(channel_id, sn, &param);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, "🎯 PTZ 精准控制");
        Ok(())
    }

    /// 存储卡格式化
    ///
    /// GB28181-2022 A.2.3.1.13：存储卡格式化
    pub async fn storage_format(&self, device_id: &str, channel_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_storage_format_xml(device_id, sn, channel_id);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, "💾 存储卡格式化");
        Ok(())
    }

    /// 目标跟踪控制
    ///
    /// GB28181-2022 A.2.3.1.14：目标跟踪（2022 新增）
    pub async fn target_track(
        &self,
        device_id: &str,
        channel_id: &str,
        start: bool,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_target_track_xml(
            channel_id,
            sn,
            if start {
                crate::xml::TargetTrackMode::Start
            } else {
                crate::xml::TargetTrackMode::Stop
            },
        );
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, start = start, "🎯 目标跟踪控制");
        Ok(())
    }

    // ── 扩展查询功能 ─────────────────────────────────────────────────────────

    /// 查询存储卡状态（2022 新增）
    ///
    /// GB28181-2022 A.2.4.14：存储卡状态查询
    pub async fn query_storage_status(
        &self,
        device_id: &str,
        channel_id: &str,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_storage_status_query_xml(device_id, sn, channel_id);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, "💾 存储卡状态查询");
        Ok(())
    }

    /// 查询巡航轨迹详情（2022 新增）
    ///
    /// GB28181-2022 A.2.4.12：巡航轨迹查询
    pub async fn query_cruise_track(
        &self,
        device_id: &str,
        channel_id: &str,
        cruise_id: &str,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_cruise_track_query_xml(channel_id, sn, cruise_id);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, cruise_id = %cruise_id, "🔄 巡航轨迹查询");
        Ok(())
    }

    /// 查询 PTZ 精准状态（2022 新增）
    ///
    /// GB28181-2022 A.2.4.13：PTZ 精准状态查询
    pub async fn query_ptz_precise_status(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_ptz_precise_status_query_xml(device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, "🎯 PTZ 精准状态查询");
        Ok(())
    }

    // ── 看守位控制 ───────────────────────────────────────────────────────────

    /// 看守位控制（独立 API）
    ///
    /// GB28181-2022 A.2.3.1.10：看守位控制
    ///
    /// # 示例
    /// ```rust,ignore
    /// use tx_di_gb28181::xml::GuardMode;
    /// // 设置看守位
    /// server.guard_control_v2("device_id", "channel_id", GuardMode::Set, 1).await?;
    /// // 调用看守位
    /// server.guard_control_v2("device_id", "channel_id", GuardMode::Call, 1).await?;
    /// // 清除看守位
    /// server.guard_control_v2("device_id", "channel_id", GuardMode::Clear, 1).await?;
    /// ```
    pub async fn guard_control_v2(
        &self,
        device_id: &str,
        channel_id: &str,
        mode: GuardMode,
        preset_index: u8,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_guard_control_xml_v2(channel_id, sn, mode, preset_index);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, channel_id = %channel_id, mode = ?mode, "🛡️ 看守位控制");
        Ok(())
    }

    // ── 回放控制 ─────────────────────────────────────────────────────────────

    /// 历史回放控制（暂停/继续/快放/拖动）
    ///
    /// GB28181-2022 §9.2
    pub async fn playback_control(
        &self,
        device_id: &str,
        ctrl: PlaybackControl,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_playback_control_xml(device_id, sn, &ctrl);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, "⏩ 回放控制");
        Ok(())
    }

    // ── 报警订阅 ─────────────────────────────────────────────────────────────

    /// 向设备订阅报警事件（SUBSCRIBE）
    ///
    /// GB28181-2022 §11：报警订阅
    pub async fn subscribe_alarm(
        &self,
        device_id: &str,
        alarm_type: u8,
        expire: u32,
    ) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_alarm_subscribe_xml(device_id, sn, alarm_type, expire);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, alarm_type = alarm_type, "🔔 订阅报警");
        Ok(())
    }

    // ── ZLM 流媒体 ───────────────────────────────────────────────────────────

    /// 检查通道是否有活跃流（通过流媒体后端 API）
    pub async fn is_streaming(&self, channel_id: &str) -> bool {
        self.inner.media.is_stream_online(channel_id).await
    }

    /// 获取通道的播放 URL
    pub fn get_play_urls(&self, channel_id: &str) -> PlayUrls {
        self.inner.media.get_play_urls(channel_id)
    }

    // ── 内部工具 ─────────────────────────────────────────────────────────────

    fn get_dev_or_err(&self, device_id: &str) -> anyhow::Result<DeviceInfo> {
        self.inner
            .registry
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("设备 {} 未注册或已离线", device_id))
    }

    fn next_sn(&self) -> u32 {
        self.inner.sn.fetch_add(1, Ordering::Relaxed)
    }

    async fn invite_internal(
        &self,
        device_id: &str,
        channel_id: &str,
        is_realtime: bool,
        start_time: Option<String>,
        end_time: Option<String>,
    ) -> anyhow::Result<(String, PlayUrls)> {
        let dev = self.get_dev_or_err(device_id)?;

        let media_ip = if is_unspecified_ip(&self.inner.config.media.local_ip) {
            self.inner.config.sip_ip.clone()
        } else {
            self.inner.config.media.local_ip.clone()
        };

        // 开启 RTP 接收端口
        let stream_id = format!("{}_{}", channel_id, self.next_sn());
        let rtp_handle = self
            .inner
            .media
            .open_rtp_server(OpenRtpRequest::udp(&stream_id))
            .await
            .map_err(|e| anyhow::anyhow!("开启 RTP 端口失败: {}，请检查流媒体后端配置", e))?;
        let rtp_port = rtp_handle.port;

        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            stream_id = %stream_id,
            rtp_port = rtp_port,
            backend = self.inner.media.backend_name(),
            "🎥 媒体后端分配 RTP 端口"
        );

        let ssrc = format!("{:010}", self.next_sn());
        let sdp_offer = if is_realtime {
            build_invite_sdp(&media_ip, rtp_port, &ssrc, SessionType::Play, None, None)
                .unwrap_or_default()
        } else {
            // ISO8601 字符串 → Unix 时间戳（秒），用于 SDP t= 字段
            let parse_ts = |s: &str| -> u64 {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ"))
                    .map(|dt| dt.and_utc().timestamp() as u64)
                    .unwrap_or(0)
            };
            let t_start = parse_ts(start_time.as_deref().unwrap_or_default());
            let t_end = parse_ts(end_time.as_deref().unwrap_or_default());
            build_invite_sdp(
                &media_ip,
                rtp_port,
                &ssrc,
                SessionType::Playback,
                Some((t_start, t_end)),
                None,
            )
            .unwrap_or_default()
        };

        let platform_id = &self.inner.config.platform_id;
        let sip_ip = &self.inner.config.sip_ip;

        let caller_str = format!("sip:{}@{}", platform_id, sip_ip);
        let callee_str = dev.contact.clone();

        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            callee = %callee_str,
            rtp_port = rtp_port,
            ssrc = %ssrc,
            "📹 发起 {} INVITE",
            if is_realtime { "实时点播" } else { "历史回放" }
        );

        let sender = SipPlugin::sender();
        let endpoint = sender.inner();
        let dialog_layer = Arc::new(DialogLayer::new(endpoint.clone()));
        let (state_tx, mut state_rx) = dialog_layer.new_dialog_state_channel();

        let caller_uri = rsip::Uri::try_from(caller_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的主叫 URI: {}", e))?;
        let callee_uri = rsip::Uri::try_from(callee_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的被叫 URI: {}", e))?;

        let invite_option = InviteOption {
            caller: caller_uri.clone(),
            callee: callee_uri,
            contact: caller_uri,
            content_type: Some("application/sdp".to_string()),
            offer: Some(sdp_offer.into_bytes().into()),
            credential: None,
            ..Default::default()
        };

        let (dialog, _resp) = dialog_layer
            .do_invite(invite_option, state_tx)
            .await
            .map_err(|e| anyhow::anyhow!("INVITE 失败: {}", e))?;

        let call_id = dialog.id().call_id.clone();

        // 记录会话信息
        let session = SessionInfo {
            call_id: call_id.clone(),
            device_id: device_id.to_string(),
            channel_id: channel_id.to_string(),
            rtp_port,
            ssrc: ssrc.clone(),
            stream_id: stream_id.clone(),
            is_realtime,
        };
        self.inner.sessions.insert(call_id.clone(), session);

        // 获取播放 URL
        let play_urls = self.inner.media.get_play_urls(&stream_id);

        // 在独立任务监听对话状态
        let call_id_clone = call_id.clone();
        let device_id_owned = device_id.to_string();
        let channel_id_owned = channel_id.to_string();
        let inner_clone = self.inner.clone();
        let rtp_port_clone = rtp_port;
        let ssrc_clone = ssrc.clone();
        let stream_id_clone = stream_id.clone();

        tokio::spawn(async move {
            while let Some(state) = state_rx.recv().await {
                match state {
                    DialogState::Confirmed(id, _resp) => {
                        info!(
                            call_id = %call_id_clone,
                            dialog_id = %id,
                            "✅ 点播会话已确认"
                        );
                        tokio::spawn(event::emit(Gb28181Event::SessionStarted {
                            device_id: device_id_owned.clone(),
                            channel_id: channel_id_owned.clone(),
                            call_id: call_id_clone.clone(),
                            rtp_port: rtp_port_clone,
                            ssrc: ssrc_clone.clone(),
                        }));
                    }
                    DialogState::Terminated(id, reason) => {
                        info!(
                            call_id = %call_id_clone,
                            dialog_id = %id,
                            reason = ?reason,
                            "📹 点播会话结束"
                        );

                        // 释放 RTP 端口
                        if let Err(e) = inner_clone.media.close_rtp_server(&stream_id_clone).await {
                            warn!(call_id = %call_id_clone, error = %e, "关闭 RTP 端口失败");
                        }

                        // 从会话表移除
                        inner_clone.sessions.remove(&call_id_clone);

                        tokio::spawn(event::emit(Gb28181Event::SessionEnded {
                            device_id: device_id_owned.clone(),
                            channel_id: channel_id_owned.clone(),
                            call_id: call_id_clone.clone(),
                        }));
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok((call_id, play_urls))
    }

    /// 向指定设备 Contact URI 发送 MESSAGE
    async fn send_message_to_device(
        &self,
        contact: &str,
        body: &str,
        seq: u32,
    ) -> anyhow::Result<()> {
        let sender = SipPlugin::sender();
        let inner = sender.inner();

        let req_uri = rsip::Uri::try_from(contact)
            .map_err(|e| anyhow::anyhow!("无效的设备 Contact URI '{}': {}", contact, e))?;

        let platform_id = &self.inner.config.platform_id;
        let sip_ip = &self.inner.config.sip_ip;
        let from_str = format!("sip:{}@{}", platform_id, sip_ip);
        let from_uri = rsip::Uri::try_from(from_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的 From URI: {}", e))?;

        let via = inner
            .get_via(None, None)
            .map_err(|e| anyhow::anyhow!("获取 Via 头失败: {}", e))?;

        let from = rsip::typed::From {
            display_name: None,
            uri: from_uri,
            params: vec![rsip::Param::Tag(rsip::uri::Tag::new(
                rsipstack::transaction::make_tag(),
            ))],
        };
        let to = rsip::typed::To {
            display_name: None,
            uri: req_uri.clone(),
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
        request.body = body.as_bytes().to_vec();

        let key = TransactionKey::from_request(&request, TransactionRole::Client)
            .map_err(|e| anyhow::anyhow!("构造事务 key 失败: {}", e))?;

        let mut tx = Transaction::new_client(key, request, inner, None);
        tx.send()
            .await
            .map_err(|e| anyhow::anyhow!("发送 MESSAGE 失败: {}", e))?;

        Ok(())
    }
}

/// 判断 IP 是否为"未指定"（any）地址，同时兼容 IPv4 `0.0.0.0` 和 IPv6 `::`
fn is_unspecified_ip(ip: &str) -> bool {
    ip == "0.0.0.0" || ip == "::" || ip == "::0" || ip == "[::]"
}
