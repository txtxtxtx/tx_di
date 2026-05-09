//! GB28181 设备端 SIP 消息处理器
//!
//! 响应平台下发的 INVITE（点播）、MESSAGE（目录/控制）、OPTIONS（探活）

use crate::channel::ChannelConfig;
use crate::config::Gb28181DeviceConfig;
use crate::device_event::{self, DeviceEvent};
use rsipstack::dialog::dialog::DialogState;
use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::sip::{Header, HeadersExt, StatusCode};
use rsipstack::transaction::transaction::Transaction;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, warn};
use tx_di_sip::SipRouter;
use tx_gb28181::Gb28181CmdType;
use tx_di_gb28181::xml::{parse_sn, parse_xml_field};

/// 注册设备端所有 SIP 消息处理器
pub fn register_device_handlers(
    config: Arc<Gb28181DeviceConfig>,
    channels: Arc<Vec<ChannelConfig>>,
) {
    let cfg_invite = config.clone();
    let cfg_message = config.clone();
    let channels_clone = channels.clone();

    // INVITE — 响应点播
    SipRouter::add_handler(Some("INVITE"), 0, move |tx| {
        let cfg = cfg_invite.clone();
        async move { handle_invite(tx, cfg).await }
    });

    // MESSAGE — 目录查询、控制指令
    SipRouter::add_handler(Some("MESSAGE"), 0, move |tx| {
        let cfg = cfg_message.clone();
        let chs = channels_clone.clone();
        async move { handle_message(tx, cfg, chs).await }
    });

    // OPTIONS — 探活
    SipRouter::add_handler(Some("OPTIONS"), 0, |mut tx| async move {
        tx.reply(StatusCode::OK)
            .await
            .map_err(|e| anyhow::anyhow!("回复 OPTIONS 200 OK 失败: {}", e))?;
        Ok(())
    });
}

// ─── INVITE ──────────────────────────────────────────────────────────────────

async fn handle_invite(
    tx: Transaction,
    config: Arc<Gb28181DeviceConfig>,
) -> anyhow::Result<()> {
    // 解析 call_id
    let call_id = tx
        .original
        .call_id_header()
        .map(|h| h.value().to_string())
        .unwrap_or_else(|_| "N/A".into());

    info!(call_id = %call_id, "📹 收到平台点播 INVITE");

    // 解析 SDP offer
    let sdp_offer = std::str::from_utf8(&tx.original.body)
        .unwrap_or("")
        .to_string();

    // 解析推流目标地址（平台要接收 RTP 的地址）
    let (rtp_target_ip, rtp_target_port) =
        tx_di_gb28181::sdp::parse_sdp_destination(&sdp_offer, &tx_di_gb28181::sdp::GBMedia::Video)
            .unwrap_or_else(|_| ("0.0.0.0".to_string(), 0));
    let ssrc = tx_di_gb28181::sdp::parse_sdp_ssrc(&sdp_offer)
        .unwrap_or_else(|| "0000000001".to_string());

    info!(
        call_id = %call_id,
        rtp_target = %format!("{}:{}", rtp_target_ip, rtp_target_port),
        ssrc = %ssrc,
        "解析点播目标地址"
    );

    // 通知业务层（媒体层开始推流前的准备）
    tokio::spawn(device_event::emit(DeviceEvent::InviteReceived {
        call_id: call_id.clone(),
        rtp_target_ip: rtp_target_ip.clone(),
        rtp_target_port,
        ssrc: ssrc.clone(),
    }));

    // 构造 SDP answer（设备告知平台从哪里推流）
    let local_ip = &config.local_ip;
    let rtp_port = config.rtp_port;
    // 根据 SDP offer 中的 s= 字段判断会话类型
    let session_type = if sdp_offer.contains("s=Download") {
        tx_di_gb28181::sdp::SessionType::Download
    } else if sdp_offer.contains("s=Playback") {
        tx_di_gb28181::sdp::SessionType::Playback
    } else {
        tx_di_gb28181::sdp::SessionType::Play
    };
    let is_realtime = matches!(session_type, tx_di_gb28181::sdp::SessionType::Play);

    // 回放或下载时：从 offer 的 t= 字段解析时间范围
    let time_range = if !is_realtime {
        let mut result = None;
        for line in sdp_offer.lines() {
            if let Some(rest) = line.strip_prefix("t=") {
                let mut parts = rest.split_whitespace();
                if let (Some(s), Some(e)) = (parts.next(), parts.next()) {
                    if let (Ok(start), Ok(end)) = (s.parse::<u64>(), e.parse::<u64>()) {
                        if start > 0 {
                            result = Some((start, end));
                            break;
                        }
                    }
                }
            }
        }
        result
    } else {
        None
    };
    let sdp_answer = tx_di_gb28181::sdp::build_sdp_answer(
        local_ip,
        rtp_port,
        &ssrc,
        &config.device_id,
        session_type,
        time_range,
        None,
    )
    .unwrap_or_default();

    // 创建 DialogLayer 和服务端对话
    let endpoint_inner = tx.endpoint_inner.clone();
    let dialog_layer = Arc::new(DialogLayer::new(endpoint_inner));
    let (state_tx, mut state_rx) = dialog_layer.new_dialog_state_channel();

    let server_dialog = dialog_layer
        .get_or_create_server_invite(&tx, state_tx, None, None)
        .map_err(|e| anyhow::anyhow!("创建服务端对话失败: {}", e))?;

    // 回 200 OK + SDP answer
    let ct = Header::ContentType("application/sdp".into());
    server_dialog
        .accept(Some(vec![ct]), Some(sdp_answer.clone().into_bytes()))
        .map_err(|e| anyhow::anyhow!("发送 200 OK 失败: {}", e))?;

    info!(call_id = %call_id, "✅ 点播 200 OK 已发送");

    // 通知业务层（200 OK 已发出，媒体层开始推流）
    tokio::spawn(device_event::emit(DeviceEvent::InviteAccepted {
        call_id: call_id.clone(),
        sdp_answer: sdp_answer.clone(),
    }));

    // 监听对话状态（等待 ACK → Confirmed，等待 BYE → Terminated）
    let call_id_clone = call_id.clone();
    tokio::spawn(async move {
        while let Some(state) = state_rx.recv().await {
            match state {
                DialogState::Confirmed(id, _) => {
                    info!(call_id = %call_id_clone, dialog_id = %id, "🤝 ACK 已确认，流媒体开始");
                    // 媒体层应在 InviteAccepted 事件中已开始推流
                }
                DialogState::Terminated(id, reason) => {
                    info!(
                        call_id = %call_id_clone,
                        dialog_id = %id,
                        reason = ?reason,
                        "📹 平台发送 BYE，点播结束"
                    );
                    tokio::spawn(device_event::emit(DeviceEvent::InviteEnded {
                        call_id: call_id_clone.clone(),
                    }));
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(())
}

// ─── MESSAGE ─────────────────────────────────────────────────────────────────

async fn handle_message(
    tx: Transaction,
    config: Arc<Gb28181DeviceConfig>,
    channels: Arc<Vec<ChannelConfig>>,
) -> anyhow::Result<()> {
    // 先回 200 OK
    let mut tx = tx;
    tx.reply(StatusCode::OK)
        .await
        .map_err(|e| anyhow::anyhow!("回复 MESSAGE 200 OK 失败: {}", e))?;

    let body = std::str::from_utf8(&tx.original.body)
        .unwrap_or("")
        .to_string();

    if body.is_empty() {
        return Ok(());
    }

    let cmd_type_str = match parse_xml_field(&body, "CmdType") {
        Some(c) => c,
        None => return Ok(()),
    };

    // 使用公共模块的类型安全枚举进行分发（支持大小写不敏感匹配）
    let cmd: Gb28181CmdType = match Gb28181CmdType::from_str(&cmd_type_str) {
        Ok(cmd) => cmd,
        Err(e) => {
            warn!(cmd = %cmd_type_str, err = %e, "未识别的 GB28181 指令类型");
            return Ok(());
        }
    };

    match cmd {
        Gb28181CmdType::Catalog => {
            let sn = parse_sn(&body);
            info!(sn = sn, "📂 收到目录查询，准备响应");
            tokio::spawn(device_event::emit(DeviceEvent::CatalogQueried { sn }));

            // 异步发送目录响应（避免在 handler 里做耗时操作）
            let cfg = config.clone();
            let chs = channels.clone();
            tokio::spawn(async move {
                if let Err(e) = send_catalog_response(&cfg, &chs, sn).await {
                    warn!("发送目录响应失败: {}", e);
                }
            });
        }
        Gb28181CmdType::DeviceInfo => {
            let sn = parse_sn(&body);
            info!(sn = sn, "ℹ️ 收到设备信息查询");
            tokio::spawn(device_event::emit(DeviceEvent::DeviceInfoQueried { sn }));

            let cfg = config.clone();
            tokio::spawn(async move {
                if let Err(e) = send_device_info_response(&cfg, sn).await {
                    warn!("发送设备信息响应失败: {}", e);
                }
            });
        }
        Gb28181CmdType::DeviceStatus => {
            let sn = parse_sn(&body);
            info!(sn = sn, "📊 收到设备状态查询");
            tokio::spawn(device_event::emit(DeviceEvent::DeviceStatusQueried { sn }));

            let cfg = config.clone();
            tokio::spawn(async move {
                if let Err(e) = send_device_status_response(&cfg, sn).await {
                    warn!("发送设备状态响应失败: {}", e);
                }
            });
        }
        Gb28181CmdType::RecordInfo => {
            let sn = parse_sn(&body);
            info!(sn = sn, "📼 收到录像查询");
            tokio::spawn(device_event::emit(DeviceEvent::RecordInfoQueried { sn }));

            let cfg = config.clone();
            tokio::spawn(async move {
                if let Err(e) = send_record_info_response(&cfg, sn).await {
                    warn!("发送录像信息响应失败: {}", e);
                }
            });
        }
        Gb28181CmdType::ConfigDownload => {
            let sn = parse_sn(&body);
            info!(sn = sn, "⚙️ 收到配置下载查询");
            tokio::spawn(device_event::emit(DeviceEvent::ConfigDownloadQueried { sn }));

            let cfg = config.clone();
            let chs = channels.clone();
            tokio::spawn(async move {
                if let Err(e) = send_config_download_response(&cfg, &chs, sn).await {
                    warn!("发送配置下载响应失败: {}", e);
                }
            });
        }
        Gb28181CmdType::PresetList => {
            let sn = parse_sn(&body);
            info!(sn = sn, "🎯 收到预置位查询");
            tokio::spawn(device_event::emit(DeviceEvent::PresetQueryQueried { sn }));

            let cfg = config.clone();
            tokio::spawn(async move {
                if let Err(e) = send_preset_query_response(&cfg, sn).await {
                    warn!("发送预置位查询响应失败: {}", e);
                }
            });
        }
        Gb28181CmdType::Keepalive => {
            // 平台有时会发心跳确认给设备，直接忽略
        }
        other => {
            warn!(cmd = %other, "未处理的平台指令");
        }
    }

    Ok(())
}

// ─── 目录/设备信息响应 ────────────────────────────────────────────────────────

async fn send_catalog_response(
    config: &Gb28181DeviceConfig,
    channels: &[ChannelConfig],
    sn: u32,
) -> anyhow::Result<()> {
    let ch_pairs: Vec<(String, String)> = channels
        .iter()
        .map(|c| (c.channel_id.clone(), c.name.clone()))
        .collect();

    let xml = tx_di_gb28181::xml::build_catalog_response_xml(
        &config.device_id,
        sn,
        &ch_pairs,
    );

    send_manscdp_to_platform(config, &xml, sn).await
}

async fn send_device_info_response(
    config: &Gb28181DeviceConfig,
    sn: u32,
) -> anyhow::Result<()> {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
         <Response>\r\n\
         <CmdType>DeviceInfo</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <Result>OK</Result>\r\n\
         <BasicParam>\r\n\
         <Name>Device-{device_id}</Name>\r\n\
         <Manufacturer>Simulator</Manufacturer>\r\n\
         <Model>IPC-V1</Model>\r\n\
         <Firmware>1.0.0</Firmware>\r\n\
         </BasicParam>\r\n\
         </Response>",
        sn = sn,
        device_id = config.device_id
    );

    send_manscdp_to_platform(config, &xml, sn).await
}

async fn send_device_status_response(
    config: &Gb28181DeviceConfig,
    sn: u32,
) -> anyhow::Result<()> {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
         <Response>\r\n\
         <CmdType>DeviceStatus</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <Result>OK</Result>\r\n\
         <Online>ON</Online>\r\n\
         <Status>OK</Status>\r\n\
         <Encode>ON</Encode>\r\n\
         <Record>OFF</Record>\r\n\
         </Response>",
        sn = sn,
        device_id = config.device_id
    );

    send_manscdp_to_platform(config, &xml, sn).await
}

async fn send_record_info_response(
    config: &Gb28181DeviceConfig,
    sn: u32,
) -> anyhow::Result<()> {
    // 返回空录像列表（模拟器无实际录像）
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
         <Response>\r\n\
         <CmdType>RecordInfo</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <SumNum>0</SumNum>\r\n\
         </Response>",
        sn = sn,
        device_id = config.device_id
    );

    send_manscdp_to_platform(config, &xml, sn).await
}

async fn send_config_download_response(
    config: &Gb28181DeviceConfig,
    _channels: &[ChannelConfig],
    sn: u32,
) -> anyhow::Result<()> {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
         <Response>\r\n\
         <CmdType>ConfigDownload</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <Result>OK</Result>\r\n\
         <BasicParam>\r\n\
         <Name>Device-{device_id}</Name>\r\n\
         <Manufacturer>Simulator</Manufacturer>\r\n\
         <Model>IPC-V1</Model>\r\n\
         <Firmware>1.0.0</Firmware>\r\n\
         </BasicParam>\r\n\
         </Response>",
        sn = sn,
        device_id = config.device_id
    );

    send_manscdp_to_platform(config, &xml, sn).await
}

async fn send_preset_query_response(
    config: &Gb28181DeviceConfig,
    sn: u32,
) -> anyhow::Result<()> {
    // 返回预置位列表（模拟器提供 3 个示例预置位）
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
         <Response>\r\n\
         <CmdType>PresetQuery</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <SumNum>3</SumNum>\r\n\
         <PresetList>\r\n\
         <Item>\r\n\
         <PresetID>1</PresetID>\r\n\
         <PresetName>Preset-1</PresetName>\r\n\
         </Item>\r\n\
         <Item>\r\n\
         <PresetID>2</PresetID>\r\n\
         <PresetName>Preset-2</PresetName>\r\n\
         </Item>\r\n\
         <Item>\r\n\
         <PresetID>3</PresetID>\r\n\
         <PresetName>Preset-3</PresetName>\r\n\
         </Item>\r\n\
         </PresetList>\r\n\
         </Response>",
        sn = sn,
        device_id = config.device_id
    );

    send_manscdp_to_platform(config, &xml, sn).await
}

/// 向平台发送 MANSCDP XML MESSAGE（通用）
async fn send_manscdp_to_platform(
    config: &Gb28181DeviceConfig,
    body: &str,
    seq: u32,
) -> anyhow::Result<()> {
    use rsipstack::sip as rsip;
    use rsipstack::transaction::key::{TransactionKey, TransactionRole};
    use rsipstack::transaction::transaction::Transaction;
    use tx_di_sip::SipPlugin;

    let sender = SipPlugin::sender();
    let inner = sender.inner();

    let platform_uri_str = config.platform_uri();
    let device_id = &config.device_id;
    let local_ip = &config.local_ip;
    let local_port = config.local_port;

    let req_uri = rsip::Uri::try_from(platform_uri_str.as_str())
        .map_err(|e| anyhow::anyhow!("无效的平台 URI: {}", e))?;

    let from_str = format!("sip:{}@{}:{}", device_id, local_ip, local_port);
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
