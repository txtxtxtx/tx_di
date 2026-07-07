//! gb_cams 设备业务回调：实现 `tx_di_gb_dev::DeviceHandler`
//!
//! 把平台下发的查询 / 点播转给 [`DeviceManager`]（设备目录）与 [`MediaManager`]
//! （RTP 流）。`GbCamsHandler` 通过 `#[component(as_trait = dyn DeviceHandler)]`
//! 注册为 `Arc<dyn DeviceHandler>`，由 DI 自动注入 `Gb28181Device`。

use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tx_di_core::{Component, DepsTuple};
use tx_di_gb_dev::{DeviceHandler, GbDevConfig};
use tx_gb28181::sdp::{self, GBMedia, SessionType};
use tx_gb28181::xml::PtzCommand;

use crate::device::{DeviceEvent, DeviceManager};
use crate::media::MediaStreamConfig;

/// 媒体流管理器：按通道 ID 索引的 RTP 推流生命周期
#[derive(Default)]
pub struct MediaManager {
    /// 通道 ID -> 该路流的取消令牌
    streams: DashMap<String, CancellationToken>,
}

impl MediaManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 启动某通道媒体流（若已存在则先停旧流，支持重播）
    pub fn start(&self, channel_id: &str, config: MediaStreamConfig) {
        self.stop(channel_id);
        let token = CancellationToken::new();
        self.streams.insert(channel_id.to_string(), token.clone());
        crate::media::MediaStream::new(config, token).start();
    }

    /// 停止某通道媒体流
    pub fn stop(&self, channel_id: &str) {
        if let Some((_, token)) = self.streams.remove(channel_id) {
            token.cancel();
        }
    }

    /// 停止全部媒体流
    pub fn stop_all(&self) {
        for entry in self.streams.iter() {
            entry.value().cancel();
        }
        self.streams.clear();
    }
}

/// gb_cams 设备回调实现（作为 `Arc<dyn DeviceHandler>` 注入 `Gb28181Device`）
#[derive(Component)]
#[component(as_trait = dyn DeviceHandler)]
pub struct GbCamsHandler {
    /// 业务目录（单例，通过 `DeviceManager::instance()` 注入）
    #[tx_cst(DeviceManager::instance())]
    state: Arc<DeviceManager>,
    /// 设备端配置（提供 device_id 用于出网 XML / SDP answer）
    gb_dev: Arc<GbDevConfig>,
}

#[async_trait]
impl DeviceHandler for GbCamsHandler {
    async fn on_catalog(&self, _sn: u32) -> Vec<(String, String)> {
        let mut channels = Vec::new();
        for dev in self.state.devices.iter() {
            for ch in &dev.channels {
                channels.push((ch.channel_id.clone(), ch.name.clone()));
            }
        }
        channels
    }

    async fn on_device_info(&self, sn: u32) -> String {
        let did = &self.gb_dev.device_id;
        let name = self
            .state
            .devices
            .iter()
            .next()
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "GB-CAMS".to_string());
        format!(
            "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n<Response>\
             <CmdType>DeviceInfo</CmdType><SN>{sn}</SN><DeviceID>{did}</DeviceID>\
             <Result>OK</Result><BasicParam><Name>{name}</Name>\
             <Manufacturer>tx</Manufacturer><Model>SimIPC</Model>\
             <Firmware>1.0.0</Firmware></BasicParam></Response>",
            sn = sn,
            did = did,
            name = name
        )
    }

    async fn on_device_status(&self, sn: u32) -> String {
        let did = &self.gb_dev.device_id;
        format!(
            "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n<Response>\
             <CmdType>DeviceStatus</CmdType><SN>{sn}</SN><DeviceID>{did}</DeviceID>\
             <Result>OK</Result><Online>ON</Online><Status>OK</Status>\
             <Encode>ON</Encode><Record>OFF</Record></Response>",
            sn = sn,
            did = did
        )
    }

    async fn on_invite(&self, channel_id: &str, sdp_offer: &str) -> String {
        let ssrc = sdp::parse_sdp_ssrc(sdp_offer).unwrap_or_else(|| "0000000001".to_string());

        let session_type = if sdp_offer.contains("s=Download") {
            SessionType::Download
        } else if sdp_offer.contains("s=Playback") {
            SessionType::Playback
        } else {
            SessionType::Play
        };
        let is_realtime = matches!(session_type, SessionType::Play);

        // 回放 / 下载：从 offer 的 t= 字段解析时间范围
        let time_range = if !is_realtime {
            let extract_t = |sdp: &str| -> Option<(u64, u64)> {
                for line in sdp.lines() {
                    if let Some(rest) = line.strip_prefix("t=") {
                        let mut parts = rest.split_whitespace();
                        let start: u64 = parts.next()?.parse().ok()?;
                        let end: u64 = parts.next()?.parse().ok()?;
                        if start > 0 {
                            return Some((start, end));
                        }
                    }
                }
                None
            };
            extract_t(sdp_offer)
        } else {
            None
        };

        // 解析平台媒体接收地址（RTP 推送目标）
        let (target_ip, target_port) = sdp::parse_sdp_destination(sdp_offer, &GBMedia::Video)
            .unwrap_or_else(|_| ("127.0.0.1".to_string(), 0));

        let cfg = &self.state.config;
        // 本端媒体地址占位（实际 RTP 推往平台 offer 目标）
        let answer = sdp::build_sdp_answer(
            "127.0.0.1",
            0,
            &ssrc,
            &self.gb_dev.device_id,
            session_type,
            time_range,
            None,
        )
        .unwrap_or_default();

        // 启动 RTP 推流
        let ms_config = MediaStreamConfig {
            width: cfg.video_width,
            height: cfg.video_height,
            fps: cfg.video_fps,
            target_ip,
            target_port,
            ssrc,
            channel_id: channel_id.to_string(),
            background_path: cfg.background_image.clone(),
        };
        self.state.media.start(channel_id, ms_config);

        self.state.emit(DeviceEvent::InviteReceived {
            device_id: self.gb_dev.device_id.clone(),
            channel_id: channel_id.to_string(),
            call_id: String::new(),
        });

        answer
    }

    async fn on_bye(&self, call_id: &str, channel_id: &str) {
        self.state.media.stop(channel_id);
        self.state.emit(DeviceEvent::InviteEnded {
            device_id: self.gb_dev.device_id.clone(),
            call_id: call_id.to_string(),
        });
    }

    async fn on_ptz(&self, channel_id: &str, _cmd: &PtzCommand) {
        info!(channel = %channel_id, "收到 PTZ 控制");
        self.state.emit(DeviceEvent::Ptz {
            channel_id: channel_id.to_string(),
        });
    }
}
