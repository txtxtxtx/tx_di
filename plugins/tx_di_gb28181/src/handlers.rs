//! GB28181 服务端 SIP 消息处理器
//!
//! 响应来自设备的 REGISTER / MESSAGE / SUBSCRIBE / NOTIFY / 以及 INVITE/BYE 响应。
//!
//! ## SIP 摘要认证流程（GB28181-2022 §5.2）
//!
//! ```text
//! 设备  ──── REGISTER (无 Authorization) ───→ 平台
//! 设备  ←─── 401 Unauthorized (WWW-Authenticate: Digest ...) ── 平台
//! 设备  ──── REGISTER (Authorization: Digest ...) ────────────→ 平台
//! 设备  ←─── 200 OK ──────────────────────────────────────────  平台
//! ```

use crate::config::Gb28181ServerConfig;
use crate::device_registry::{DeviceRegistry};
use crate::plugin::Gb28181Server;
use tx_gb28181::device::GbDevice;
use crate::event::{emit, Gb28181Event};
use crate::xml::{
    parse_alarm_notify, parse_catalog_items, parse_config_download_response,
    parse_cruise_list, parse_cruise_track, parse_guard_info, parse_media_status,
    parse_preset_list, parse_ptz_precise_status, parse_record_items,
    parse_time_sync_response, parse_xml_field,
};

// ── 从公共模块 re-export Gb28181CmdType（向后兼容）─────────────────────────
pub use tx_gb28181::Gb28181CmdType;
use rsipstack::sip::{Header, HeadersExt, StatusCode};
use std::sync::Arc;
use tracing::{info, warn};
use tx_di_core::RIE;
use tx_di_sip::{SipTx, SipUasManager};

/// 创建简单的 SIP 响应处理器（回复 200 OK）
fn create_ok_handler(method_name: &'static str) -> impl Fn(SipTx) -> std::pin::Pin<Box<dyn Future<Output = RIE<()>> + Send>> + Send + Sync + 'static {
    move |tx: SipTx| {
        let method = method_name;
        Box::pin(async move {
            tx.reply(StatusCode::OK)
                .await
                .map_err(|e| anyhow::anyhow!("回复 {} 200 OK 失败: {}", method, e))?;
            Ok(())
        })
    }
}

/// 向 SipRouter 注册所有 GB28181 服务端消息处理器
pub fn register_server_handlers(server: Arc<Gb28181Server>) -> RIE<()> {
    let sip_plugin = server.sip_plugin.clone();
    let reg_register = server.device_registry.clone();
    let cfg_register = server.config.clone();
    let reg_message = server.device_registry.clone();
    let cfg_message = server.config.clone();

    // REGISTER — 设备注册/注销/刷新
    // （摘要认证与 ACL 已由 Gb28181AuthMiddleware 在洋葱链前置处理）
    sip_plugin.add_handler(Some("REGISTER"), 0, move |tx: SipTx| {
        let reg = reg_register.clone();
        let cfg = cfg_register.clone();
        async move { handle_register(tx, reg, cfg).await }
    })?;

    // MESSAGE — 心跳、目录响应、设备信息响应、报警、录像等
    sip_plugin.add_handler(Some("MESSAGE"), 0, move |tx| {
        let reg = reg_message.clone();
        let cfg = cfg_message.clone();
        async move { handle_message(tx, reg, cfg).await }
    })?;

    // NOTIFY — 报警订阅通知
    sip_plugin.add_handler(Some("NOTIFY"), 0, create_ok_handler("NOTIFY"))?;

    // SUBSCRIBE — 订阅请求（简单回 200 OK）
    sip_plugin.add_handler(Some("SUBSCRIBE"), 0, create_ok_handler("SUBSCRIBE"))?;

    // OPTIONS — 探活 / keep-alive
    sip_plugin.add_handler(Some("OPTIONS"), 0, create_ok_handler("OPTIONS"))?;

    // INVITE — 设备发起的 INVITE（语音广播/对讲推音频，UAS 方向）
    let uas_inv = server.uas.clone();
    let srv_inv = server.clone();
    sip_plugin.add_handler(Some("INVITE"), 0, move |tx: SipTx| {
        let uas = uas_inv.clone();
        let srv = srv_inv.clone();
        async move { handle_device_invite(&srv, &uas, &tx).await }
    })?;

    // BYE — 会话挂断（UAS 会话清理 + 回复 200）
    let uas_bye = server.uas.clone();
    sip_plugin.add_handler(Some("BYE"), 0, move |tx: SipTx| {
        let uas = uas_bye.clone();
        async move { uas.on_bye(&tx).await }
    })?;

    Ok(())
}

/// 处理设备发起的 INVITE（语音广播/对讲推音频，UAS 方向）
///
/// 流程：解析设备 SDP → 分配 RTP 接收端口 → 回 200 OK（SDP answer）→
/// 监听会话状态，Terminated 时清理端口与事件。
async fn handle_device_invite(
    server: &Arc<Gb28181Server>,
    uas: &Arc<SipUasManager>,
    tx: &SipTx,
) -> RIE<()> {
    use crate::media::OpenRtpRequest;
    use rsipstack::sip::StatusCode;

    // 从 From 头提取设备 ID
    let from_str = tx
        .request()
        .from_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();
    let device_id = extract_user_from_sip_uri(&from_str).unwrap_or_else(|| from_str.clone());

    // 解析 SDP 会话类型（s= 行），如 Broadcast / Talk
    let sdp_body = String::from_utf8_lossy(&tx.request().body).to_string();
    let session_name = sdp_body
        .lines()
        .find_map(|l| l.trim().strip_prefix("s="))
        .unwrap_or("Broadcast")
        .to_string();

    info!(
        device_id = %device_id,
        session = %session_name,
        "📥 收到设备 INVITE（UAS）"
    );

    // 创建 UAS 会话（回 100 Trying，注册 dialog）
    let session = match uas.on_invite(tx, &device_id).await {
        Ok(s) => s,
        Err(e) => {
            warn!(device_id = %device_id, error = %e, "创建设备 INVITE 会话失败");
            tx.reply(StatusCode::ServerInternalError)
                .await
                .map_err(|e| anyhow::anyhow!("回复 INVITE 500 失败: {}", e))?;
            return Ok(());
        }
    };

    // 分配 RTP 接收端口
    let media = match server.media.get() {
        Some(m) => m.clone(),
        None => {
            warn!("MediaBackend 未初始化，拒绝设备 INVITE");
            let _ = uas.reject(&session, Some(StatusCode::ServerInternalError));
            return Ok(());
        }
    };
    let sn = server.next_sn_for(&device_id);
    let stream_id = format!("uas_{}_{}", device_id, sn);
    let rtp = match media.open_rtp_server(OpenRtpRequest::udp(&stream_id)).await {
        Ok(h) => h,
        Err(e) => {
            warn!(device_id = %device_id, error = %e, "分配 RTP 端口失败，拒绝设备 INVITE");
            let _ = uas.reject(&session, Some(StatusCode::ServerInternalError));
            return Ok(());
        }
    };

    // 构建接收侧 SDP answer（PCMA 音频；广播/对讲主要收音频流）
    let media_ip = server.media_ip();
    let answer = build_uas_sdp_answer(&media_ip, rtp.port, &session_name, &format!("{:010}", sn));

    if let Err(e) = uas.accept(&session, answer.as_bytes(), None) {
        warn!(device_id = %device_id, error = %e, "接受设备 INVITE 失败");
        let _ = media.close_rtp_server(&stream_id).await;
        let _ = tx.reply(StatusCode::ServerInternalError).await;
        return Ok(());
    }

    info!(
        device_id = %device_id,
        session = %session_name,
        rtp_port = rtp.port,
        "✅ 已接受设备 INVITE（200 OK）"
    );

    // 记录广播会话 + 事件
    server.broadcast_sessions.insert(device_id.clone(), rtp.port);
    tokio::spawn(emit(Gb28181Event::BroadcastSessionStarted {
        device_id: device_id.clone(),
        audio_port: rtp.port,
    }));

    // 状态监听：Terminated → 清理端口 + 会话 + 事件
    let srv2 = server.clone();
    let uas2 = uas.clone();
    let sess2 = session.clone();
    let stream_id2 = stream_id.clone();
    let device_id2 = device_id.clone();
    tokio::spawn(async move {
        let mut finished = false;
        if let Some(mut rx) = sess2.take_state_rx() {
            while let Some(state) = rx.recv().await {
                if matches!(state, rsipstack::dialog::dialog::DialogState::Terminated(..)) {
                    finished = true;
                    break;
                }
            }
        }
        if !finished {
            // 无状态流或超时：兜底延迟清理
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
        srv2.broadcast_sessions.remove(&device_id2);
        if let Some(m) = srv2.media.get() {
            let _ = m.close_rtp_server(&stream_id2).await;
        }
        let _ = uas2.hangup(&sess2).await;
        tokio::spawn(emit(Gb28181Event::BroadcastSessionEnded {
            device_id: device_id2,
        }));
    });

    Ok(())
}

/// 构建 UAS 接收侧 SDP answer（广播/对讲：音频 PCMA 8）
///
/// 设备 → 平台方向：平台作为接收方回 recvonly。
fn build_uas_sdp_answer(media_ip: &str, rtp_port: u16, session_name: &str, ssrc: &str) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (addr_type, addr) = if media_ip.contains(':') {
        ("IP6", media_ip)
    } else {
        ("IP4", media_ip)
    };
    format!(
        "v=0\r\n\
         o=- {session_id} {session_id} IN {addr_type} {addr}\r\n\
         s={session_name}\r\n\
         c=IN {addr_type} {addr}\r\n\
         t=0 0\r\n\
         m=audio {rtp_port} RTP/AVP 8\r\n\
         a=recvonly\r\n\
         a=rtcp:{rtcp_port}\r\n\
         a=rtpmap:8 PCMA/8000\r\n\
         y={ssrc}\r\n",
        session_id = session_id,
        addr_type = addr_type,
        addr = addr,
        session_name = session_name,
        rtp_port = rtp_port,
        rtcp_port = rtp_port + 1,
        ssrc = ssrc
    )
}

// ── REGISTER ─────────────────────────────────────────────────────────────────

/// 处理 REGISTER 请求
///
/// 摘要认证与 ACL 前置校验已由 `Gb28181AuthMiddleware` 在洋葱链完成，
/// 本函数只负责注册/注销业务。
async fn handle_register(
    tx: SipTx,
    registry: DeviceRegistry,
    config: Arc<Gb28181ServerConfig>,
) -> RIE<()> {
    // 解析 From 头中的 device_id
    let from_str = tx
        .request()
        .from_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();
    let device_id = extract_user_from_sip_uri(&from_str).unwrap_or_else(|| from_str.clone());

    // 解析 Expires
    let expires = tx
        .request()
        .expires_header()
        .map(|h| h.value().to_string().parse::<u32>().unwrap_or(3600))
        .unwrap_or(3600);

    // 解析 Contact 头
    let contact = tx
        .request()
        .contact_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();

    // 获取远端地址（Via 头）
    let remote_addr = tx
        .request()
        .via_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();

    info!(
        device_id = %device_id,
        expires = expires,
        "📡 收到 REGISTER"
    );

    // ── 正常注册/注销逻辑 ────────────────────────────────────────────────────
    if expires == 0 {
        // 注销
        registry.unregister(&device_id);
        tx.reply(StatusCode::OK)
            .await
            .map_err(|e| anyhow::anyhow!("回复注销 200 OK 失败: {}", e))?;

        tokio::spawn(emit(Gb28181Event::DeviceUnregistered {
            device_id: device_id.clone(),
        }));
    } else {
        // 注册或刷新
        let is_new = !registry.is_registered(&device_id);
        let dev = GbDevice {
            device_id: device_id.clone(),
            contact: contact.clone(),
            expires,
            remote_addr: remote_addr.clone(),
            online: true,
            registered_at: chrono::Utc::now(),
            last_heartbeat: tokio::time::Instant::now(),
            version: config.device_version_for(&device_id),
            ..Default::default()
        };
        registry.register(dev);

        // 回 200 OK（带 Expires 头）
        let headers = vec![Header::Expires(config.register_ttl.into())];
        tx.reply_with(StatusCode::OK, headers, None)
            .await
            .map_err(|e| anyhow::anyhow!("回复注册 200 OK 失败: {}", e))?;

        if is_new {
            tokio::spawn(emit(Gb28181Event::DeviceRegistered {
                device_id: device_id.clone(),
                contact: contact.clone(),
                remote_addr: contact,
            }));
        }
    }

    Ok(())
}

// ── MESSAGE ──────────────────────────────────────────────────────────────────

async fn handle_message(
    tx: SipTx,
    registry: DeviceRegistry,
    _config: Arc<Gb28181ServerConfig>,
) -> RIE<()> {
    // 先回 200 OK（GB28181 要求先确认再处理）
    // create_ok_handler("MESSAGE")(tx).await?;
    tx.reply(StatusCode::OK)
        .await
        .map_err(|e| anyhow::anyhow!("回复 MESSAGE 200 OK 失败: {}", e))?;

    let body = std::str::from_utf8(&tx.request().body)
        .unwrap_or("")
        .to_string();

    if body.is_empty() {
        return Ok(());
    }

    let cmd_type = match parse_xml_field(&body, "CmdType") {
        Some(cmd) => cmd,
        None => {
            warn!("收到无 CmdType 的 MESSAGE，已忽略");
            return Ok(());
        }
    };

    let from_str = tx
        .request()
        .from_header() // 从 From 头中提取
        .map(|h| h.value().to_string())
        .unwrap_or_default();
    let device_id =
        extract_user_from_sip_uri(&from_str).unwrap_or_else(|| from_str.clone());

    let cmd: Gb28181CmdType = match cmd_type.parse() {
        Ok(cmd) => cmd,
        Err(_) => {
            warn!(device_id = %device_id, cmd = %cmd_type, "未识别的 GB28181 指令类型");
            return Ok(());
        }
    };

    match cmd {
        Gb28181CmdType::Keepalive => handle_keepalive(&device_id, &body, &registry).await,
        Gb28181CmdType::Catalog => handle_catalog_response(&device_id, &body, &registry).await,
        Gb28181CmdType::DeviceInfo => handle_device_info(&device_id, &body).await,
        Gb28181CmdType::DeviceStatus => handle_device_status(&device_id, &body).await,
        Gb28181CmdType::RecordInfo => handle_record_info(&device_id, &body).await,
        Gb28181CmdType::Alarm => handle_alarm(&device_id, &body).await,
        Gb28181CmdType::MediaStatus => handle_media_status(&device_id, &body).await,
        Gb28181CmdType::MobilePosition => handle_mobile_position(&device_id, &body).await,
        Gb28181CmdType::ConfigDownload => handle_config_download(&device_id, &body).await,
        Gb28181CmdType::PresetList => handle_preset_list(&device_id, &body).await,
        Gb28181CmdType::CruiseList => handle_cruise_list(&device_id, &body).await,
        Gb28181CmdType::CruiseTrack => handle_cruise_track_response(&device_id, &body).await,
        Gb28181CmdType::PtzPreciseStatus => handle_ptz_precise_status_response(&device_id, &body).await,
        Gb28181CmdType::GuardInfo => handle_guard_info(&device_id, &body).await,
        Gb28181CmdType::Broadcast => handle_broadcast(&device_id, &body).await,
        _ => {
            warn!(device_id = %device_id, cmd = %cmd_type, "未处理的 GB28181 指令类型");
            Ok(())
        }
    }
}

/// Keepalive 心跳处理器
async fn handle_keepalive(
    device_id: &str,
    body: &str,
    registry: &DeviceRegistry,
) -> RIE<()> {
    let status = parse_xml_field(body, "Status").unwrap_or_else(|| "OK".to_string());
    let was_offline = registry
        .get(device_id)
        .map(|d| !d.online)
        .unwrap_or(false);
    let was_refreshed = registry.refresh_heartbeat(device_id);

    if !was_refreshed {
        // todo 可以重新注册，而不是忽略
        warn!(device_id = %device_id, "收到未注册设备的心跳（已忽略）");
        return Ok(());
    }

    info!(device_id = %device_id, status = %status, "💓 收到 Keepalive");

    // 如果设备之前离线，现在上报心跳则触发上线事件
    if was_offline {
        tokio::spawn(emit(Gb28181Event::DeviceOnline {
            device_id: device_id.to_string(),
        }));
    }

    tokio::spawn(emit(Gb28181Event::Keepalive {
        device_id: device_id.to_string(),
        status,
    }));

    Ok(())
}

/// 目录响应处理器
async fn handle_catalog_response(
    device_id: &str,
    body: &str,
    registry: &DeviceRegistry,
) -> RIE<()> {
    let items = parse_catalog_items(body);
    let channel_count = items.len();

    info!(
        device_id = %device_id,
        channel_count = channel_count,
        "📂 收到目录响应"
    );

    // 子设备继承父设备（网关）的协议版本
    let parent_version = registry
        .get(device_id)
        .map(|d| d.version)
        .unwrap_or_default();

    let sub_devices: Vec<GbDevice> = items
        .iter()
        .map(|item| {
            let mut sub = GbDevice::from_item_type(item);
            sub.version = parent_version;
            sub
        })
        .collect();

    // 批量注册子设备（2022 模型：每个通道是独立的 GbDevice 节点）
    for sub in &sub_devices {
        registry.register(sub.clone());
    }

    tokio::spawn(emit(Gb28181Event::CatalogReceived {
        device_id: device_id.to_string(),
        channel_count,
        channels: items,
    }));

    Ok(())
}

/// 设备信息处理器
async fn handle_device_info(device_id: &str, body: &str) -> RIE<()> {
    let manufacturer = parse_xml_field(body, "Manufacturer").unwrap_or_default();
    let model = parse_xml_field(body, "Model").unwrap_or_default();
    let firmware = parse_xml_field(body, "Firmware").unwrap_or_default();
    let channel = parse_xml_field(body, "Channel")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0u32);

    info!(
        device_id = %device_id,
        manufacturer = %manufacturer,
        model = %model,
        firmware = %firmware,
        "ℹ️ 收到设备信息"
    );

    tokio::spawn(emit(Gb28181Event::DeviceInfoReceived {
        device_id: device_id.to_string(),
        manufacturer,
        model,
        firmware,
        channel_num: channel,
    }));

    Ok(())
}

/// 设备状态处理器
async fn handle_device_status(device_id: &str, body: &str) -> RIE<()> {
    // 优先检查是否为校时响应（包含 TimeRequest 则为校时）
    if body.contains("<TimeRequest>") {
        let sync_info = parse_time_sync_response(body);
        if let Some(info) = sync_info {
            info!(
                device_id = %device_id,
                device_time = %info.device_time,
                diff_secs = info.time_diff_secs,
                "🕐 收到设备校时响应"
            );
            tokio::spawn(emit(Gb28181Event::TimeSyncResult {
                device_id: device_id.to_string(),
                device_time: info.device_time,
                time_diff_secs: info.time_diff_secs,
            }));
        }
        return Ok(());
    }

    let status = crate::xml::parse_device_status(body);
    info!(
        device_id = %device_id,
        online = %status.on_line,
        record = %status.record,
        "📊 收到设备状态"
    );

    tokio::spawn(emit(Gb28181Event::DeviceStatusReceived {
        device_id: device_id.to_string(),
        online: status.on_line,
        status: status.status,
        encode: status.encode,
        record: status.record,
    }));

    Ok(())
}

/// 录像文件列表处理器
async fn handle_record_info(device_id: &str, body: &str) -> RIE<()> {
    let items = parse_record_items(body);
    let sum_num = parse_xml_field(body, "SumNum")
        .and_then(|s| s.parse().ok())
        .unwrap_or(items.len() as u32);

    info!(
        device_id = %device_id,
        count = items.len(),
        sum_num = sum_num,
        "📼 收到录像文件列表"
    );

    tokio::spawn(emit(Gb28181Event::RecordInfoReceived {
        device_id: device_id.to_string(),
        sum_num,
        items,
    }));

    Ok(())
}

/// 报警通知处理器
async fn handle_alarm(device_id: &str, body: &str) -> RIE<()> {
    if let Some(alarm) = parse_alarm_notify(body) {
        info!(
            device_id = %device_id,
            alarm_type = %alarm.alarm_type,
            priority = alarm.alarm_priority,
            desc = %alarm.alarm_description,
            "🚨 收到报警通知"
        );

        tokio::spawn(emit(Gb28181Event::AlarmReceived {
            device_id: device_id.to_string(),
            alarm_time: alarm.start_alarm_time,
            alarm_type: alarm.alarm_type,
            alarm_priority: alarm.alarm_priority,
            alarm_description: alarm.alarm_description,
            longitude: alarm.longitude,
            latitude: alarm.latitude,
        }));
    }
    Ok(())
}

/// 媒体状态通知处理器
async fn handle_media_status(device_id: &str, body: &str) -> RIE<()> {
    let notify_type = parse_media_status(body).unwrap_or_else(|| "121".to_string());
    info!(
        device_id = %device_id,
        notify_type = %notify_type,
        "📡 收到媒体状态通知"
    );

    tokio::spawn(emit(Gb28181Event::MediaStatusNotify {
        device_id: device_id.to_string(),
        notify_type,
    }));

    Ok(())
}

/// 移动位置通知处理器
async fn handle_mobile_position(device_id: &str, body: &str) -> RIE<()> {
    let longitude = parse_xml_field(body, "Longitude")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0f64);
    let latitude = parse_xml_field(body, "Latitude")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0f64);
    let speed = parse_xml_field(body, "Speed")
        .and_then(|s| s.parse().ok());
    let direction = parse_xml_field(body, "Direction")
        .and_then(|s| s.parse().ok());

    info!(
        device_id = %device_id,
        lon = longitude,
        lat = latitude,
        speed = speed,
        direction = direction,
        "📍 收到移动位置通知"
    );

    tokio::spawn(emit(Gb28181Event::MobilePosition {
        device_id: device_id.to_string(),
        longitude,
        latitude,
        speed,
        direction,
    }));

    Ok(())
}

/// 配置下载处理器
async fn handle_config_download(device_id: &str, body: &str) -> RIE<()> {
    let config_type = parse_xml_field(body, "ConfigType").unwrap_or_default();
    let items = parse_config_download_response(body);
    info!(
        device_id = %device_id,
        config_type = %config_type,
        count = items.len(),
        "⚙️ 收到设备配置响应"
    );

    tokio::spawn(emit(Gb28181Event::ConfigDownloaded {
        device_id: device_id.to_string(),
        config_type,
        items: items.into_iter().map(|i| (i.name, i.value)).collect(),
    }));

    Ok(())
}

/// 预置位列表处理器
async fn handle_preset_list(device_id: &str, body: &str) -> RIE<()> {
    let channel_id = parse_xml_field(body, "DeviceID").unwrap_or_else(|| device_id.to_string());
    let presets = parse_preset_list(body);
    info!(
        device_id = %device_id,
        channel_id = %channel_id,
        count = presets.len(),
        "📍 收到预置位列表"
    );

    tokio::spawn(emit(Gb28181Event::PresetListReceived {
        device_id: device_id.to_string(),
        channel_id,
        presets: presets.into_iter().map(|p| (p.preset_id, p.name)).collect(),
    }));

    Ok(())
}

/// 巡航轨迹列表处理器
async fn handle_cruise_list(device_id: &str, body: &str) -> RIE<()> {
    let channel_id = parse_xml_field(body, "DeviceID").unwrap_or_else(|| device_id.to_string());
    let cruises = parse_cruise_list(body);
    info!(
        device_id = %device_id,
        channel_id = %channel_id,
        count = cruises.len(),
        "🔄 收到巡航轨迹列表"
    );

    tokio::spawn(emit(Gb28181Event::CruiseListReceived {
        device_id: device_id.to_string(),
        channel_id,
        cruises: cruises.into_iter().map(|c| (c.cruise_id, c.name)).collect(),
    }));

    Ok(())
}

/// 处理看守位信息
async fn handle_guard_info(device_id: &str, body: &str) -> RIE<()> {
    let guard_info = match parse_guard_info(body) {
        Some(info) => info,
        None => {
            warn!(device_id = %device_id, "无法解析看守位信息");
            return Ok(());
        }
    };
    info!(
        device_id = %device_id,
        guard_id = guard_info.guard_id,
        preset_index = guard_info.preset_index,
        "🛡️ 收到看守位信息"
    );

    tokio::spawn(emit(Gb28181Event::GuardInfoReceived {
        device_id: device_id.to_string(),
        guard_id: guard_info.guard_id,
        preset_index: guard_info.preset_index,
    }));

    Ok(())
}

/// 处理巡航轨迹详情响应
///
/// GB28181-2022 A.2.4.12：巡航轨迹查询响应（2022 新增）
async fn handle_cruise_track_response(device_id: &str, body: &str) -> RIE<()> {
    let channel_id = parse_xml_field(body, "DeviceID").unwrap_or_else(|| device_id.to_string());
    let tracks = parse_cruise_track(body);

    info!(
        device_id = %device_id,
        channel_id = %channel_id,
        track_count = tracks.len(),
        "🔄 收到巡航轨迹详情"
    );

    tokio::spawn(emit(Gb28181Event::CruiseTrackReceived {
        device_id: device_id.to_string(),
        channel_id,
        tracks,
    }));

    Ok(())
}

/// 处理 PTZ 精准状态响应
///
/// GB28181-2022 A.2.4.13：PTZ 精准状态查询响应（2022 新增）
async fn handle_ptz_precise_status_response(device_id: &str, body: &str) -> RIE<()> {
    let channel_id = parse_xml_field(body, "DeviceID").unwrap_or_else(|| device_id.to_string());

    match parse_ptz_precise_status(body) {
        Some(status) => {
            info!(
                device_id = %device_id,
                channel_id = %channel_id,
                pan = status.pan_position,
                tilt = status.tilt_position,
                zoom = status.zoom_position,
                "📷 收到 PTZ 精准状态"
            );

            tokio::spawn(emit(Gb28181Event::PtzPreciseStatusReceived {
                device_id: device_id.to_string(),
                channel_id,
                pan_position: status.pan_position,
                tilt_position: status.tilt_position,
                zoom_position: status.zoom_position,
                focus_position: status.focus_position,
                iris_position: status.iris_position,
            }));
        }
        None => {
            warn!(device_id = %device_id, "无法解析 PTZ 精准状态");
        }
    }

    Ok(())
}

/// 处理语音广播 MESSAGE
///
/// GB28181-2022 §9.12：
/// - Invite：设备邀请平台接收广播
/// - TearDown：广播结束通知
async fn handle_broadcast(device_id: &str, body: &str) -> RIE<()> {
    let source_id = parse_xml_field(body, "SourceID").unwrap_or_default();
    let notify_type = parse_xml_field(body, "NotifyType");

    match notify_type.as_deref() {
        Some("TearDown") | Some("BYE") => {
            info!(
                device_id = %device_id,
                source_id = %source_id,
                "📢 广播结束通知"
            );
            tokio::spawn(emit(Gb28181Event::BroadcastSessionEnded {
                device_id: device_id.to_string(),
            }));
        }
        _ => {
            // 广播邀请
            info!(
                device_id = %device_id,
                source_id = %source_id,
                "📢 收到语音广播邀请"
            );
            tokio::spawn(emit(Gb28181Event::BroadcastInviteReceived {
                device_id: device_id.to_string(),
                source_id,
            }));
        }
    }
    Ok(())
}

// ── 工具函数 ──────────────────────────────────────────────────────────────────

// 从公共模块 re-export（向后兼容）
pub use tx_gb28181::sip::extract_user_from_sip_uri;
