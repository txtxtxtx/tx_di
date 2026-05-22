use crate::event::{self, Gb28181Event};
use crate::media::{MediaBackend, OpenRtpRequest, PlayUrls};
use crate::plugin::{Gb28181Server, SessionInfo};
use crate::sdp::{
    build_audio_invite_sdp, build_invite_sdp, build_snapshot_sdp, parse_audio_sdp,
    parse_snapshot_sdp, AudioCodec, SessionType,
};
use crate::xml::{
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
    build_time_sync_response_xml, build_zoom_in_xml, build_zoom_out_xml, ConfigType,
    GuardMode, PlaybackControl, PtzCommand, PtzPreciseParam, ZoomRect,
};
use rsipstack::dialog::dialog::DialogState;
use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::dialog::invitation::InviteOption;
use rsipstack::sip as rsip;
use rsipstack::transaction::key::{TransactionKey, TransactionRole};
use rsipstack::transaction::transaction::Transaction;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{info, warn};
use tx_gb28181::device::GbDevice;

impl Gb28181Server {
    // ── 注册表查询 ───────────────────────────────────────────────────────────

    /// 获取设备信息
    pub fn get_device(&self, device_id: &str) -> Option<GbDevice> {
        self.device_registry.get(device_id)
    }

    /// 获取所有在线设备
    pub fn online_devices(&self) -> Vec<GbDevice> {
        self.device_registry.online_devices()
    }

    /// 获取注册设备总数
    pub fn device_count(&self) -> usize {
        self.device_registry.all_devices_count()
    }

    /// 获取在线设备数
    pub fn online_count(&self) -> usize {
        self.device_registry.online_count()
    }

    /// 获取设备下所有子设备（原通道）
    pub fn get_channels(&self, device_id: &str) -> Vec<GbDevice> {
        self.device_registry.sub_devices(device_id)
    }

    // ── 主动查询 ─────────────────────────────────────────────────────────────

    /// 向设备发送目录查询（MESSAGE Catalog）
    ///
    /// 设备收到后会回复包含通道列表的 MESSAGE，触发 `Gb28181Event::CatalogReceived`。
    pub async fn query_catalog(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_catalog_query_xml(&self.config.platform_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, sn = sn, "发送目录查询");
        Ok(())
    }

    /// 向设备发送设备信息查询（MESSAGE DeviceInfo）
    pub async fn query_device_info(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_device_info_query_xml(&self.config.platform_id, device_id, sn);
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
    pub async fn time_sync(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self.get_dev_or_err(device_id)?;
        let sn = self.next_sn();
        let xml = build_time_sync_query_xml(&self.config.platform_id, device_id, sn);
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
        let session = self.sessions.get(call_id).map(|r: dashmap::mapref::one::Ref<String, SessionInfo>| r.value().clone());

        if let Some(sess) = session {
            let stream_id = sess.stream_id.clone();
            let media = self.media.get().expect("MediaBackend not initialized");
            if let Err(e) = media.close_rtp_server(&stream_id).await {
                warn!(call_id = %call_id, error = %e, "关闭 RTP 端口失败（忽略）");
            }

            self.sessions.remove(call_id);

            info!(call_id = %call_id, "📴 主动挂断");

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
        self.sessions
            .iter()
            .map(|r: dashmap::mapref::multiple::RefMulti<String, SessionInfo>| r.value().clone())
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
        let sn = self.next_sn_for(device_id);
        let media_ip = if is_unspecified_ip(&self.config.media.local_ip) {
            self.config.sip_ip.clone()
        } else {
            self.config.media.local_ip.clone()
        };

        let stream_id = format!("snapshot_{}_{}", channel_id, sn);
        let sdp_offer = build_snapshot_sdp(&media_ip, sn);

        let platform_id = &self.config.platform_id;
        let sip_ip = &self.config.sip_ip;

        let caller_str = format!("sip:{}@{}", platform_id, sip_ip);
        let callee_str = dev.contact.clone();

        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            sn = sn,
            "📸 发起抓拍 INVITE"
        );

        let sender = self.sip_plugin.sender()?;
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

        let media_clone = self.media.get().expect("MediaBackend not initialized").clone();
        let call_id_clone = call_id.clone();
        let _device_id_owned = device_id.to_string();
        let _channel_id_owned = channel_id.to_string();

        tokio::spawn(async move {
            while let Some(state) = state_rx.recv().await {
                if matches!(state, DialogState::Terminated(_, _)) {
                    info!(
                        call_id = %call_id_clone,
                        "📸 抓拍会话结束"
                    );
                    let _ = media_clone.close_rtp_server(&stream_id).await;
                    break;
                }
            }
        });

        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            "📸 抓拍请求完成"
        );

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
        let xml = build_broadcast_invite_xml(&self.config.platform_id, device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        info!(device_id = %device_id, sn = sn, "发起语音广播邀请");
        Ok(())
    }

    /// 确认广播会话（平台接收音频）
    ///
    /// 当收到 `BroadcastInviteReceived` 事件后，平台可调用此方法确认接收。
    /// 需要传入音频端口，设备会将音频推送到此端口。
    pub async fn broadcast_accept(&self, device_id: &str, audio_port: u16) -> anyhow::Result<()> {
        self.broadcast_sessions
            .insert(device_id.to_string(), audio_port);

        let sn = self.next_sn();
        let media_ip = if is_unspecified_ip(&self.config.media.local_ip) {
            self.config.sip_ip.clone()
        } else {
            self.config.media.local_ip.clone()
        };

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
            "确认广播接收，监听端口"
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
        let xml = build_broadcast_cancel_xml(&self.config.platform_id, device_id, sn);
        self.send_message_to_device(&dev.contact, &xml, sn).await?;
        self.broadcast_sessions.remove(device_id);
        info!(device_id = %device_id, "结束语音广播");
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
        let sn = self.next_sn_for(device_id);
        let media_ip = if is_unspecified_ip(&self.config.media.local_ip) {
            self.config.sip_ip.clone()
        } else {
            self.config.media.local_ip.clone()
        };

        let stream_id = format!("talkback_{}_{}", channel_id, sn);
        let media = self.media.get().expect("MediaBackend not initialized");
        let handle = media
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

        let platform_id = &self.config.platform_id;
        let sip_ip = &self.config.sip_ip;
        let caller_str = format!("sip:{}@{}", platform_id, sip_ip);
        let callee_str = dev.contact.clone();

        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            audio_port = audio_port,
            "🎤 发起对讲 INVITE"
        );

        let sender = self.sip_plugin.sender()?;
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

        let session = SessionInfo {
            call_id: call_id.clone(),
            device_id: device_id.to_string(),
            channel_id: channel_id.to_string(),
            rtp_port: video_port,
            ssrc: ssrc.clone(),
            stream_id: stream_id.clone(),
            is_realtime: true,
        };
        self.sessions.insert(call_id.clone(), session);

        let media_clone = self.media.get().expect("MediaBackend not initialized").clone();
        let sessions_clone = self.sessions.clone();
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
                        let _ = media_clone.close_rtp_server(&stream_id).await;
                        sessions_clone.remove(&call_id_clone);
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
        let media = self.media.get().expect("MediaBackend not initialized");
        media.is_stream_online(channel_id).await
    }

    /// 获取通道的播放 URL
    pub fn get_play_urls(&self, channel_id: &str) -> PlayUrls {
        let media = self.media.get().expect("MediaBackend not initialized");
        media.get_play_urls(channel_id)
    }

    // ── Group E: 录像下载 ───────────────────────────────────────────────────

    /// 录像下载（INVITE s=Download）
    pub async fn invite_download(
        &self,
        device_id: &str,
        channel_id: &str,
        download_speed: Option<u32>,
    ) -> anyhow::Result<(String, PlayUrls)> {
        let _ = download_speed;
        self.invite_internal(device_id, channel_id, false, None, None).await
    }

    // ── Group F: 移动位置主动查询 ──────────────────────────────────────────

    /// 主动查询设备位置（A.2.4.5）
    ///
    /// `interval`: None = 仅查一次，Some(secs) = 设备按间隔持续上报
    pub async fn query_mobile_position(
        &self,
        device_id: &str,
        interval: Option<u32>,
    ) -> anyhow::Result<()> {
        let sn = self.next_sn_for(device_id);
        let xml = crate::xml::build_mobile_position_query_xml(device_id, sn, interval);
        let device = self.get_dev_or_err(device_id)?;
        self.send_message_to_device(&device.contact, &xml, sn).await
    }

    /// 取消移动位置订阅
    pub async fn unsubscribe_mobile_position(&self, device_id: &str) -> anyhow::Result<()> {
        self.query_mobile_position(device_id, Some(0)).await
    }

    // ── Group G: 云台锁定/解锁 + DeviceControl 抓拍 + 配置推送 ─────────────

    /// PTZ 云台锁定（A.2.3.1.12）
    pub async fn ptz_lock(&self, device_id: &str, channel_id: &str) -> anyhow::Result<()> {
        let sn = self.next_sn_for(device_id);
        let xml = crate::xml::build_ptz_lock_xml(channel_id, sn);
        self.send_device_control(device_id, channel_id, sn, &xml, "PTZ锁定").await
    }

    /// PTZ 云台解锁（A.2.3.1.12）
    pub async fn ptz_unlock(&self, device_id: &str, channel_id: &str) -> anyhow::Result<()> {
        let sn = self.next_sn_for(device_id);
        let xml = crate::xml::build_ptz_unlock_xml(channel_id, sn);
        self.send_device_control(device_id, channel_id, sn, &xml, "PTZ解锁").await
    }

    /// 手动抓拍（DeviceControl 方式，非 INVITE 模式）
    pub async fn snapshot_control(&self, device_id: &str, channel_id: &str) -> anyhow::Result<()> {
        let sn = self.next_sn_for(device_id);
        let xml = crate::xml::build_snapshot_control_xml(channel_id, sn);
        self.send_device_control(device_id, channel_id, sn, &xml, "手动抓拍").await
    }

    /// 推送配置参数到设备（A.2.3.2）
    pub async fn push_config(
        &self,
        device_id: &str,
        config_type: crate::xml::ConfigType,
        params: &[(String, String)],
    ) -> anyhow::Result<()> {
        let sn = self.next_sn_for(device_id);
        let xml = crate::xml::build_config_push_xml(device_id, sn, config_type, params);
        let device = self.get_dev_or_err(device_id)?;
        self.send_message_to_device(&device.contact, &xml, sn).await
    }

    // ── 内部工具 ─────────────────────────────────────────────────────────────

    fn get_dev_or_err(&self, device_id: &str) -> anyhow::Result<GbDevice> {
        self.device_registry
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("设备 {} 未注册或已离线", device_id))
    }

    fn next_sn(&self) -> u32 {
        self.next_sn_for(&self.config.platform_id)
    }

    /// 为指定设备获取下一个 SN 序列号（每设备独立计数）
    fn next_sn_for(&self, device_id: &str) -> u32 {
        self.sn_map
            .entry(device_id.to_string())
            .or_insert_with(|| AtomicU32::new(1))
            .fetch_add(1, Ordering::Relaxed)
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

        let media_ip = if is_unspecified_ip(&self.config.media.local_ip) {
            self.config.sip_ip.clone()
        } else {
            self.config.media.local_ip.clone()
        };

        let stream_id = format!("{}_{}", channel_id, self.next_sn_for(device_id));
        let media = self.media.get().expect("MediaBackend not initialized");
        let rtp_handle = media
            .open_rtp_server(OpenRtpRequest::udp(&stream_id))
            .await
            .map_err(|e| anyhow::anyhow!("开启 RTP 端口失败: {}，请检查流媒体后端配置", e))?;
        let rtp_port = rtp_handle.port;

        info!(
            device_id = %device_id,
            channel_id = %channel_id,
            stream_id = %stream_id,
            rtp_port = rtp_port,
            backend = media.backend_name(),
            "🎥 媒体后端分配 RTP 端口"
        );

        let ssrc = format!("{:010}", self.next_sn_for(device_id));
        let sdp_offer = if is_realtime {
            build_invite_sdp(&media_ip, rtp_port, &ssrc, SessionType::Play, None, None)
                .unwrap_or_default()
        } else {
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

        let platform_id = &self.config.platform_id;
        let sip_ip = &self.config.sip_ip;

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

        let sender = self.sip_plugin.sender()?;
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

        let session = SessionInfo {
            call_id: call_id.clone(),
            device_id: device_id.to_string(),
            channel_id: channel_id.to_string(),
            rtp_port,
            ssrc: ssrc.clone(),
            stream_id: stream_id.clone(),
            is_realtime,
        };
        self.sessions.insert(call_id.clone(), session);

        let play_urls = media.get_play_urls(&stream_id);

        let call_id_clone = call_id.clone();
        let device_id_owned = device_id.to_string();
        let channel_id_owned = channel_id.to_string();
        let media_clone = self.media.get().expect("MediaBackend not initialized").clone();
        let sessions_clone = self.sessions.clone();
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

                        if let Err(e) = media_clone.close_rtp_server(&stream_id_clone).await {
                            warn!(call_id = %call_id_clone, error = %e, "关闭 RTP 端口失败");
                        }

                        sessions_clone.remove(&call_id_clone);

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

    /// 发送 DeviceControl MESSAGE 的通用方法
    #[allow(unused_variables)]
    async fn send_device_control(
        &self,
        device_id: &str,
        _channel_id: &str,
        sn: u32,
        xml_body: &str,
        desc: &str,
    ) -> anyhow::Result<()> {
        let device = self.get_dev_or_err(device_id)?;
        self.send_message_to_device(&device.contact, xml_body, sn).await
    }

    /// 向指定设备 Contact URI 发送 MESSAGE
    async fn send_message_to_device(
        &self,
        contact: &str,
        body: &str,
        seq: u32,
    ) -> anyhow::Result<()> {
        let sender = self.sip_plugin.sender()?;
        let inner = sender.inner();

        let req_uri = rsip::Uri::try_from(contact)
            .map_err(|e| anyhow::anyhow!("无效的设备 Contact URI '{}': {}", contact, e))?;

        let platform_id = &self.config.platform_id;
        let sip_ip = &self.config.sip_ip;
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
