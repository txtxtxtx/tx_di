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
use crate::device_registry::{ChannelInfo, ChannelStatus, DeviceInfo, DeviceRegistry};
use crate::event::{emit, Gb28181Event};
use crate::xml::{
    parse_alarm_notify, parse_catalog_items, parse_config_download_response,
    parse_cruise_list, parse_cruise_track, parse_guard_info, parse_media_status,
    parse_preset_list, parse_ptz_precise_status, parse_record_items,
    parse_time_sync_response, parse_xml_field,
};
use rsipstack::sip::{Header, HeadersExt, StatusCode};
use rsipstack::transaction::transaction::Transaction;
use std::str::FromStr;
use std::sync::Arc;
use dashmap::DashMap;
use tracing::{debug, info, warn};
use tx_di_sip::SipRouter;

/// 向 SipRouter 注册所有 GB28181 服务端消息处理器
pub fn register_server_handlers(
    registry: Arc<DeviceRegistry>,
    config: Arc<Gb28181ServerConfig>,
    nonce_store: Arc<NonceStore>,
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
    SipRouter::add_handler(Some("NOTIFY"), 0, move |mut tx| async move {
        tx.reply(StatusCode::OK)
            .await
            .map_err(|e| anyhow::anyhow!("回复 NOTIFY 200 OK 失败: {}", e))?;
        Ok(())
    });

    // SUBSCRIBE — 订阅请求（简单回 200 OK）
    SipRouter::add_handler(Some("SUBSCRIBE"), 0, move |mut tx| async move {
        tx.reply(StatusCode::OK)
            .await
            .map_err(|e| anyhow::anyhow!("回复 SUBSCRIBE 200 OK 失败: {}", e))?;
        Ok(())
    });

    // OPTIONS — 探活 / keep-alive
    SipRouter::add_handler(Some("OPTIONS"), 0, |mut tx| async move {
        tx.reply(StatusCode::OK)
            .await
            .map_err(|e| anyhow::anyhow!("回复 OPTIONS 200 OK 失败: {}", e))?;
        Ok(())
    });
}

// ── Nonce 存储（用于摘要认证）────────────────────────────────────────────────

/// Nonce 存储：记录已发给每个设备的 nonce，防止重放攻击
pub struct NonceStore {
    inner: DashMap<String, String>,
}

impl NonceStore {
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    /// 生成并存储 nonce
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

/// 生成随机 nonce（16字节十六进制）
fn generate_nonce() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    // 简单 nonce：时间戳 + 简单散列 0x9e37_79b9 著名的 Golden Ratio constant（黄金比例常数）
    format!("{:08x}{:08x}", t, t.wrapping_mul(0x9e37_79b9))
}

/// 验证 SIP 摘要认证
///
/// 按 RFC 2617 / GB28181 规范验证 Authorization 头
fn verify_digest_auth(
    auth_header: &str,
    method: &str,
    uri: &str,
    expected_password: &str,
    realm: &str,
    nonce: &str,
) -> bool {
    // 从 Authorization 头提取各字段
    let get_field = |name: &str| -> Option<String> {
        let prefix = format!("{}=\"", name);
        let start = auth_header.find(&prefix)? + prefix.len();
        let end = auth_header[start..].find('"')?;
        Some(auth_header[start..start + end].to_string())
    };

    let auth_response = match get_field("response") {
        Some(r) => r,
        None => return false,
    };
    let auth_nonce = get_field("nonce").unwrap_or_default();
    let auth_realm = get_field("realm").unwrap_or_default();
    let auth_username = get_field("username").unwrap_or_default();

    // 验证 realm 匹配
    if auth_realm != realm {
        debug!("realm 不匹配: expected={}, got={}", realm, auth_realm);
        return false;
    }

    // 验证 nonce 匹配
    if auth_nonce != nonce {
        debug!("nonce 不匹配: expected={}, got={}", nonce, auth_nonce);
        return false;
    }

    // 计算期望的 response = MD5(MD5(A1):nonce:MD5(A2))
    // A1 = username:realm:password （使用传入的 realm 而非 Authorization 中的）
    let a1 = format!("{}:{}:{}", auth_username, realm, expected_password);
    // A2 = method:uri （使用传入的 uri）
    let a2 = format!("{}:{}", method, uri);
    let ha1 = md5_hex(md5_digest(a1.as_bytes()));
    let ha2 = md5_hex(md5_digest(a2.as_bytes()));
    let expected = md5_hex(md5_digest(format!("{}:{}:{}", ha1, nonce, ha2).as_bytes()));

    if auth_response == expected {
        true
    } else {
        debug!(
            "摘要验证失败: username={}, realm={}, uri={}",
            auth_username, realm, uri
        );
        false
    }
}

/// 简单的 MD5 实现（GB28181 只使用 MD5，无需引入额外依赖）
fn md5_digest(data: &[u8]) -> [u8; 16] {
    // RFC 1321 MD5 实现
    let mut state = [0x67452301u32, 0xefcdab89, 0x98badcfe, 0x10325476];

    // 填充
    let orig_len_bits = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&orig_len_bits.to_le_bytes());

    // K 表
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
        0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
        0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
        0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
        0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
        0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
        0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
        0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
        0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
        0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
        0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
        0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
        0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
    ];
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
        5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
        6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 16];
        for (i, b) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }
        let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
        for i in 0..64usize {
            let (f, g): (u32, usize) = if i < 16 {
                ((b & c) | (!b & d), i)
            } else if i < 32 {
                ((d & b) | (!d & c), (5 * i + 1) % 16)
            } else if i < 48 {
                (b ^ c ^ d, (3 * i + 5) % 16)
            } else {
                (c ^ (b | !d), (7 * i) % 16)
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(K[i])
                    .wrapping_add(w[g])
                    .rotate_left(S[i]),
            );
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }

    let mut result = [0u8; 16];
    for (i, &s) in state.iter().enumerate() {
        let b = s.to_le_bytes();
        result[i * 4..i * 4 + 4].copy_from_slice(&b);
    }
    result
}

/// 将 MD5 结果转换为十六进制字符串
fn md5_hex(hash: [u8; 16]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

// ── REGISTER ─────────────────────────────────────────────────────────────────

async fn handle_register(
    mut tx: Transaction,
    registry: Arc<DeviceRegistry>,
    config: Arc<Gb28181ServerConfig>,
    nonce_store: Arc<NonceStore>,
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
                    &config.auth_password,
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
    registry: Arc<DeviceRegistry>,
    _config: Arc<Gb28181ServerConfig>,
) -> anyhow::Result<()> {
    // 先回 200 OK（GB28181 要求先确认再处理）
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
        .from_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();
    let device_id =
        extract_user_from_sip_uri(&from_str).unwrap_or_else(|| from_str.clone());

    match cmd_type.as_str() {
        "Keepalive" => handle_keepalive(&device_id, &body, &registry).await,
        "Catalog" => handle_catalog_response(&device_id, &body, &registry).await,
        "DeviceInfo" => handle_device_info(&device_id, &body).await,
        "DeviceStatus" => handle_device_status(&device_id, &body).await,
        "RecordInfo" => handle_record_info(&device_id, &body).await,
        "Alarm" => handle_alarm(&device_id, &body).await,
        "MediaStatus" => handle_media_status(&device_id, &body).await,
        "MobilePosition" => handle_mobile_position(&device_id, &body).await,
        "ConfigDownload" => handle_config_download(&device_id, &body).await,
        "PresetList" => handle_preset_list(&device_id, &body).await,
        "CruiseList" => handle_cruise_list(&device_id, &body).await,
        "CruiseTrack" => handle_cruise_track_response(&device_id, &body).await,
        "PtzPreciseStatus" => handle_ptz_precise_status_response(&device_id, &body).await,
        "GuardInfo" => handle_guard_info(&device_id, &body).await,
        "Broadcast" => handle_broadcast(&device_id, &body).await,
        other => {
            warn!(device_id = %device_id, cmd = %other, "未处理的 GB28181 指令");
            Ok(())
        }
    }
}

async fn handle_keepalive(
    device_id: &str,
    body: &str,
    registry: &Arc<DeviceRegistry>,
) -> anyhow::Result<()> {
    let status = parse_xml_field(body, "Status").unwrap_or_else(|| "OK".to_string());
    let was_offline = registry
        .get(device_id)
        .map(|d| !d.online)
        .unwrap_or(false);
    let was_refreshed = registry.refresh_heartbeat(device_id);

    if !was_refreshed {
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

async fn handle_catalog_response(
    device_id: &str,
    body: &str,
    registry: &Arc<DeviceRegistry>,
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

/// 从 SIP URI 字符串中提取 user 部分
///
/// 示例: `"<sip:34020000001320000001@192.168.1.200>"` → `Some("34020000001320000001")`
pub fn extract_user_from_sip_uri(uri_str: &str) -> Option<String> {
    // 去掉尖括号及 display-name
    let clean = uri_str
        .trim()
        .trim_start_matches('"')
        .trim();

    // 提取 < > 内的部分
    let inner = if let (Some(s), Some(e)) = (clean.find('<'), clean.rfind('>')) {
        &clean[s + 1..e]
    } else {
        clean
    };

    // 去掉 sip: 前缀
    let after_scheme = inner
        .strip_prefix("sip:")
        .or_else(|| inner.strip_prefix("sips:"))
        .unwrap_or(inner);

    // 取 @ 之前的 user 部分（去掉 ;tag=xxx 等参数）
    let user_part = after_scheme.splitn(2, '@').next().unwrap_or(after_scheme);
    // 去掉可能的参数
    let user = user_part.split(';').next().unwrap_or(user_part);

    if user.is_empty() {
        None
    } else {
        Some(user.to_string())
    }
}
