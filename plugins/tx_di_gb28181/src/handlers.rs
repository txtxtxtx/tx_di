//! GB28181 服务端 SIP 消息处理器
//!
//! 响应来自设备的 REGISTER / MESSAGE / 以及服务端发出 INVITE 后的 ACK/BYE。

use crate::device_registry::{ChannelInfo, ChannelStatus, DeviceInfo, DeviceRegistry};
use crate::event::{emit, Gb28181Event};
use crate::xml::{parse_catalog_items, parse_xml_field};
use rsipstack::sip::{HeadersExt, StatusCode};
use rsipstack::transaction::transaction::Transaction;
use std::sync::Arc;
use tracing::{info, warn};
use tx_di_sip::SipRouter;

/// 向 SipRouter 注册所有 GB28181 服务端消息处理器
pub fn register_server_handlers(registry: Arc<DeviceRegistry>) {
    let reg_register = registry.clone();
    let reg_message = registry.clone();

    // REGISTER — 设备注册/注销/刷新
    SipRouter::add_handler(Some("REGISTER"), 0, move |tx| {
        let reg = reg_register.clone();
        async move { handle_register(tx, reg).await }
    });

    // MESSAGE — 心跳、目录响应、设备信息响应
    SipRouter::add_handler(Some("MESSAGE"), 0, move |tx| {
        let reg = reg_message.clone();
        async move { handle_message(tx, reg).await }
    });

    // OPTIONS — 探活 / keep-alive
    SipRouter::add_handler(Some("OPTIONS"), 0, |mut tx| async move {
        tx.reply(StatusCode::OK)
            .await
            .map_err(|e| anyhow::anyhow!("回复 OPTIONS 200 OK 失败: {}", e))?;
        Ok(())
    });
}

// ─── REGISTER ────────────────────────────────────────────────────────────────

async fn handle_register(mut tx: Transaction, registry: Arc<DeviceRegistry>) -> anyhow::Result<()> {
    // 解析 From 头中的 device_id
    let from_str = tx
        .original
        .from_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();

    // GB28181 规范：From: <sip:device_id@realm>
    let device_id = extract_user_from_sip_uri(&from_str)
        .unwrap_or_else(|| from_str.clone());

    // 解析 Contact 头
    let contact = tx
        .original
        .contact_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();

    // 解析 Expires（注销时为 0）
    let expires = tx
        .original
        .expires_header()
        .map(|h| h.value().to_string().parse::<u32>().unwrap_or(3600))
        .unwrap_or(3600);

    // 获取远端地址
    let remote_addr = tx
        .original
        .via_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();

    info!(
        device_id = %device_id,
        expires = expires,
        contact = %contact,
        "📡 收到 REGISTER"
    );

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

        // 回 200 OK
        tx.reply(StatusCode::OK)
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

// ─── MESSAGE ─────────────────────────────────────────────────────────────────

async fn handle_message(mut tx: Transaction, registry: Arc<DeviceRegistry>) -> anyhow::Result<()> {
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
        "RecordInfo" => {
            info!(device_id = %device_id, "📼 收到录像文件列表（RecordInfo）");
            Ok(())
        }
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
    let was_refreshed = registry.refresh_heartbeat(device_id);

    if !was_refreshed {
        warn!(device_id = %device_id, "收到未注册设备的心跳（已忽略）");
        return Ok(());
    }

    info!(device_id = %device_id, status = %status, "💓 收到 Keepalive");

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
        .into_iter()
        .map(|(ch_id, name, status)| ChannelInfo {
            channel_id: ch_id,
            name,
            manufacturer: String::new(),
            model: String::new(),
            status: ChannelStatus::from_str(&status),
            address: String::new(),
        })
        .collect();

    registry.update_channels(device_id, channels);

    tokio::spawn(emit(Gb28181Event::CatalogReceived {
        device_id: device_id.to_string(),
        channel_count,
    }));

    Ok(())
}

async fn handle_device_info(device_id: &str, body: &str) -> anyhow::Result<()> {
    let manufacturer = parse_xml_field(body, "Manufacturer").unwrap_or_default();
    let model = parse_xml_field(body, "Model").unwrap_or_default();
    let firmware = parse_xml_field(body, "Firmware").unwrap_or_default();

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
    }));

    Ok(())
}

// ─── 工具函数 ─────────────────────────────────────────────────────────────────

/// 从 SIP URI 字符串中提取 user 部分
///
/// 示例: `"<sip:34020000001320000001@192.168.1.200>"` → `Some("34020000001320000001")`
pub fn extract_user_from_sip_uri(uri_str: &str) -> Option<String> {
    // 去掉尖括号
    let clean = uri_str
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();

    // 去掉 sip: 前缀
    let after_scheme = if let Some(rest) = clean.strip_prefix("sip:") {
        rest
    } else if let Some(rest) = clean.strip_prefix("sips:") {
        rest
    } else {
        clean
    };

    // 取 @ 之前的 user 部分
    if let Some(at) = after_scheme.find('@') {
        Some(after_scheme[..at].to_string())
    } else {
        // 无 @ 则整个是 host/user
        Some(after_scheme.to_string())
    }
}
