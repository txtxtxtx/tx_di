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
use crate::crypto::{generate_nonce, verify_digest_auth};
use crate::device_registry::{ChannelInfo, ChannelStatus, DeviceInfo, DeviceRegistry};
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
use rsipstack::transaction::transaction::Transaction;
use std::str::FromStr;
use std::sync::Arc;
use dashmap::DashMap;
use tracing::{debug, info, warn};
use tx_di_sip::SipRouter;

// ── 创建简单的 SIP 响应处理器（回复 200 OK）─────────────────────────────────
fn create_ok_handler(method_name: &'static str) -> impl Fn(Transaction) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send>> + Send + Sync + 'static {
    move |mut tx| {
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
pub fn register_server_handlers(
    registry: DeviceRegistry,
    config: Arc<Gb28181ServerConfig>,
    nonce_store: NonceStore,
) {
    let reg_register = registry.clone();
    let cfg_register = config.clone();
    let nonce_register = nonce_store.clone();
    let reg_message = registry.clone();
    let cfg_message = config.clone();

    // REGISTER — 设备注册/注销/刷新（含摘要认证）
    SipRouter::add_handler(Some("REGISTER"), 0, move |tx| {
        let reg = reg_register.clone();
        let cfg = cfg_register.clone();
        let nonce = nonce_register.clone();
        async move { handle_register(tx, reg, cfg, nonce).await }
    });

    // MESSAGE — 心跳、目录响应、设备信息响应、报警、录像等
    SipRouter::add_handler(Some("MESSAGE"), 0, move |tx| {
        let reg = reg_message.clone();
        let cfg = cfg_message.clone();
        async move { handle_message(tx, reg, cfg).await }
    });

    // NOTIFY — 报警订阅通知
    SipRouter::add_handler(Some("NOTIFY"), 0, create_ok_handler("NOTIFY"));

    // SUBSCRIBE — 订阅请求（简单回 200 OK）
    SipRouter::add_handler(Some("SUBSCRIBE"), 0, create_ok_handler("SUBSCRIBE"));

    // OPTIONS — 探活 / keep-alive
    SipRouter::add_handler(Some("OPTIONS"), 0, create_ok_handler("OPTIONS"));
}

// ── Nonce 存储（用于摘要认证）────────────────────────────────────────────────
// Nonce 生成和摘要验证函数已提取到 crate::crypto 模块

/// Nonce 存储：记录已发给每个设备的 nonce，防止重放攻击
#[derive(Clone)]
pub struct NonceStore {
    inner: Arc<DashMap<String, String>>,
}

impl NonceStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()) ,
        }
    }

    /// 生成并存储 nonce（使用加密安全随机数）
    pub fn issue(&self, device_id: &str) -> String {
        let nonce = generate_nonce();
        self.inner
            .insert(device_id.to_string(), nonce.clone());
        nonce
    }

    /// 验证 nonce（验证后不自动删除，允许重用）
    #[allow(dead_code)]
    pub fn verify(&self, device_id: &str, nonce: &str) -> bool {
        self.inner.get(device_id)
            .map(|n| n.value() == nonce)
            .unwrap_or(false)
    }

    /// 删除 nonce（认证成功后清除）
    pub fn remove(&self, device_id: &str) {
        self.inner.remove(device_id);
    }
}

// ── REGISTER ─────────────────────────────────────────────────────────────────

/// 处理 REGISTER 请求
async fn handle_register(
    mut tx: Transaction,
    registry: DeviceRegistry,
    config: Arc<Gb28181ServerConfig>,
    nonce_store: NonceStore,
) -> anyhow::Result<()> {
    // 解析 From 头中的 device_id
    let from_str = tx
        .original
        .from_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();
    let device_id = extract_user_from_sip_uri(&from_str).unwrap_or_else(|| from_str.clone());

    // 解析 Expires
    let expires = tx
        .original
        .expires_header()
        .map(|h| h.value().to_string().parse::<u32>().unwrap_or(3600))
        .unwrap_or(3600);

    // 解析 Contact 头
    let contact = tx
        .original
        .contact_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();

    // 获取远端地址（Via 头）
    let remote_addr = tx
        .original
        .via_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();

    info!(
        device_id = %device_id,
        expires = expires,
        "📡 收到 REGISTER"
    );

    // ── 摘要认证 ────────────────────────────────────────────────────────────
    if config.enable_auth && expires > 0 {
        // 检查是否有 Authorization 头
        let auth_header = tx
            .original
            .headers
            .iter()
            .find(|h| {
                let s = format!("{}", h);
                s.to_lowercase().starts_with("authorization:")
            })
            .map(|h| format!("{}", h));

        match auth_header {
            None => {
                // 没有 Authorization → 发 401 + 生成 nonce
                let nonce = nonce_store.issue(&device_id);
                let www_auth = format!(
                    "Digest realm=\"{}\", nonce=\"{}\", algorithm=MD5",
                    config.realm, nonce
                );
                let headers = vec![Header::WwwAuthenticate(
                    rsipstack::sip::WwwAuthenticate::new(www_auth),
                )];
                tx.reply_with(StatusCode::Unauthorized, headers, None)
                    .await
                    .map_err(|e| anyhow::anyhow!("发送 401 Unauthorized 失败: {}", e))?;
                debug!(device_id = %device_id, "🔐 发送 401 质询");
                return Ok(());
            }
            Some(auth) => {
                // 有 Authorization → 验证
                let nonce = nonce_store
                    .inner
                    .get(&device_id)
                    .map(|v| v.clone())
                    .unwrap_or_default();

                // 提取 REGISTER 的 Request-URI（用于 MD5 计算）
                let req_uri = tx
                    .original
                    .uri
                    .to_string();

                let ok = verify_digest_auth(
                    &auth,
                    "REGISTER",
                    &req_uri,
                    config.get_password(&device_id),
                    &config.realm,
                    &nonce,
                );

                if !ok {
                    warn!(device_id = %device_id, "🔐 摘要认证失败，拒绝注册");
                    tx.reply(StatusCode::Forbidden)
                        .await
                        .map_err(|e| anyhow::anyhow!("发送 403 失败: {}", e))?;
                    return Ok(());
                }

                // 认证成功，清除 nonce
                nonce_store.remove(&device_id);
                debug!(device_id = %device_id, "🔐 摘要认证通过");
            }
        }
    }

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
        let dev_info = DeviceInfo::new(
            device_id.clone(),
            contact.clone(),
            expires,
            remote_addr,
        );
        registry.register(dev_info);

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
    mut tx: Transaction,
    registry: DeviceRegistry,
    _config: Arc<Gb28181ServerConfig>,
) -> anyhow::Result<()> {
    // 先回 200 OK（GB28181 要求先确认再处理）
    // create_ok_handler("MESSAGE")(tx).await?;
    tx.reply(StatusCode::OK)
        .await
        .map_err(|e| anyhow::anyhow!("回复 MESSAGE 200 OK 失败: {}", e))?;

    let body = std::str::from_utf8(&tx.original.body)
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
        .original
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
) -> anyhow::Result<()> {
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
) -> anyhow::Result<()> {
    let items = parse_catalog_items(body);
    let channel_count = items.len();

    info!(
        device_id = %device_id,
        channel_count = channel_count,
        "📂 收到目录响应"
    );

    let channels: Vec<ChannelInfo> = items
        .iter()
        .map(|item| ChannelInfo {
            channel_id: item.device_id.clone(),
            name: item.name.clone(),
            manufacturer: item.manufacturer.clone(),
            model: item.model.clone(),
            status: ChannelStatus::from_str(&item.status).unwrap(),
            address: item.address.clone(),
            parent_id: item.parent_id.clone(),
            ip_address: item.ip_address.clone(),
            port: item.port,
            longitude: item.longitude,
            latitude: item.latitude,
            parental: item.parental,
            register_way: item.register_way,
            secrecy: item.secrecy,
            civil_code: item.civil_code.clone(),
        })
        .collect();

    registry.update_channels(device_id, channels.clone());

    tokio::spawn(emit(Gb28181Event::CatalogReceived {
        device_id: device_id.to_string(),
        channel_count,
        channels,
    }));

    Ok(())
}

/// 设备信息处理器
async fn handle_device_info(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_device_status(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_record_info(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_alarm(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_media_status(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_mobile_position(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_config_download(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_preset_list(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_cruise_list(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_guard_info(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_cruise_track_response(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_ptz_precise_status_response(device_id: &str, body: &str) -> anyhow::Result<()> {
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
async fn handle_broadcast(device_id: &str, body: &str) -> anyhow::Result<()> {
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
