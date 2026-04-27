//! GB28181 SIP 消息处理器
//!
//! 注册服务端侧的消息处理逻辑：
//!
//! | 消息      | 场景                                  |
//! |-----------|---------------------------------------|
//! | `INVITE`  | 平台下发点播/回放指令                 |
//! | `BYE`     | 平台结束流媒体（孤立 BYE 兜底）       |
//! | `MESSAGE` | 平台下发目录查询等控制指令             |
//! | `OPTIONS` | 心跳探活（部分平台用 OPTIONS 探活）   |

use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::sip::{HeadersExt, StatusCode};
use rsipstack::transaction::transaction::Transaction;
use std::sync::Arc;
use tracing::{info, warn};
use tx_di_sip::SipRouter;

/// 注册所有 GB28181 相关的 SIP 消息处理器
pub fn register_all() {
    register_invite_handler();
    register_bye_handler();
    register_message_handler();
    register_options_handler();
}

// ─── INVITE：响应平台点播 ────────────────────────────────────────────────────

fn register_invite_handler() {
    SipRouter::add_handler(Some("INVITE"), 0, |tx| async move {
        handle_invite(tx).await
    });
}

async fn handle_invite(tx: Transaction) -> anyhow::Result<()> {
    let call_id = tx
        .original
        .call_id_header()
        .map(|h| h.value().to_string())
        .unwrap_or_else(|_| "N/A".into());

    let from = tx
        .original
        .from_header()
        .map(|h| h.value().to_string())
        .unwrap_or_else(|_| "N/A".into());

    info!(call_id = %call_id, from = %from, "📹 收到点播请求");

    // 解析 SDP offer（Content-Type: application/sdp）
    let sdp_offer = std::str::from_utf8(&tx.original.body)
        .unwrap_or("")
        .to_string();
    info!("收到 SDP offer:\n{}", sdp_offer);

    // 提取流媒体地址（从 SDP 的 c= 和 m= 行解析）
    let (stream_ip, stream_port) = parse_sdp_destination(&sdp_offer);
    info!(
        stream_ip = %stream_ip,
        stream_port = %stream_port,
        "解析到推流目标地址"
    );

    // 构造 SDP answer（告知平台本设备推流参数）
    //
    // ⚠️ 实际项目中应该：
    //   1. 通知媒体子系统（FFmpeg/GStreamer/rtp-rs）开始推流到 stream_ip:stream_port
    //   2. 将实际 RTP 端口填入 SDP answer
    let local_rtp_port = 10000u16; // 实际应从媒体层动态分配
    let sdp_answer = build_sdp_answer(
        "192.168.1.100", // 本机 IP（实际从 SipConfig 读取）
        local_rtp_port,
        &sdp_offer,
    );

    // 构建 DialogLayer 并创建服务端 INVITE 对话
    let endpoint_inner = tx.endpoint_inner.clone();
    let dialog_layer = Arc::new(DialogLayer::new(endpoint_inner));

    // 创建 state channel 用于监听对话状态变化
    let (state_sender, mut state_receiver) = dialog_layer.new_dialog_state_channel();

    // 创建服务端对话（同步，不 await）
    let server_dialog = dialog_layer
        .get_or_create_server_invite(
            &tx,
            state_sender,
            None,  // credential（此处不需要认证）
            None,  // local_contact（使用默认 Contact）
        )
        .map_err(|e| anyhow::anyhow!("创建服务端对话失败: {}", e))?;

    // 构造 Content-Type 头并接受呼叫（200 OK + SDP answer）
    let content_type_header = rsipstack::sip::Header::ContentType("application/sdp".into());
    server_dialog
        .accept(
            Some(vec![content_type_header]),
            Some(sdp_answer.into_bytes()),
        )
        .map_err(|e| anyhow::anyhow!("回复 200 OK 失败: {}", e))?;

    info!(call_id = %call_id, "✅ 点播会话建立，等待 ACK");

    // 在独立任务中等待对话状态变化（等待 BYE/超时）
    let call_id_clone = call_id.clone();
    tokio::spawn(async move {
        use rsipstack::dialog::dialog::DialogState;

        while let Some(state) = state_receiver.recv().await {
            match state {
                DialogState::Confirmed(id, _resp) => {
                    info!(call_id = %call_id_clone, id = %id, "🤝 点播 ACK 已确认，流媒体开始");
                    // TODO: 通知媒体层开始推流
                }
                DialogState::Terminated(id, reason) => {
                    info!(
                        call_id = %call_id_clone,
                        id = %id,
                        reason = ?reason,
                        "📹 点播结束"
                    );
                    // TODO: 通知媒体层停止推流
                    break;
                }
                _ => {
                    // 其他状态变化（Early、WaitAck 等），可按需处理
                }
            }
        }
    });

    Ok(())
}

// ─── BYE：孤立 BYE 兜底 ──────────────────────────────────────────────────────
//
// 注：rsipstack ServerInviteDialog 内部已通过 DialogState 处理 BYE，
// 此 handler 仅处理不在 dialog 上下文的孤立 BYE。

fn register_bye_handler() {
    SipRouter::add_handler(Some("BYE"), 0, |mut tx| async move {
        let call_id = tx
            .original
            .call_id_header()
            .map(|h| h.value().to_string())
            .unwrap_or_else(|_| "N/A".to_string());
        info!(call_id = %call_id, "收到孤立 BYE，回复 200 OK");
        tx.reply(StatusCode::OK)
            .await
            .map_err(|e| anyhow::anyhow!("回复 BYE 200 OK 失败: {}", e))?;
        Ok(())
    });
}

// ─── MESSAGE：处理目录查询、控制指令 ─────────────────────────────────────────

fn register_message_handler() {
    SipRouter::add_handler(Some("MESSAGE"), 0, |mut tx| async move {
        let body = std::str::from_utf8(&tx.original.body)
            .unwrap_or("")
            .to_string();
        info!("收到 MESSAGE:\n{}", body);

        // 先回 200 OK（GB28181 规范要求先确认再处理）
        tx.reply(StatusCode::OK)
            .await
            .map_err(|e| anyhow::anyhow!("回复 200 OK 失败: {}", e))?;

        // 解析 GB28181 XML 指令类型
        if let Some(cmd) = extract_xml_field(&body, "CmdType") {
            match cmd.as_str() {
                "Catalog" => {
                    info!("📂 收到目录查询，稍后异步推送目录响应");
                    // TODO: 通过 Gb28181Manager::send_catalog_response() 推送目录
                }
                "DeviceInfo" => {
                    info!("ℹ️  收到设备信息查询");
                    // TODO: 推送设备信息
                }
                "Keepalive" => {
                    info!("💓 收到平台心跳确认");
                }
                "RecordInfo" => {
                    info!("📼 收到录像文件查询");
                    // TODO: 推送录像文件列表
                }
                other => {
                    warn!("未处理的 GB28181 指令: {}", other);
                }
            }
        }

        Ok(())
    });
}

// ─── OPTIONS：心跳探活 ───────────────────────────────────────────────────────

fn register_options_handler() {
    SipRouter::add_handler(Some("OPTIONS"), 0, |mut tx| async move {
        info!("💓 收到 OPTIONS 心跳探活，回复 200 OK");
        tx.reply(StatusCode::OK)
            .await
            .map_err(|e| anyhow::anyhow!("回复 OPTIONS 200 OK 失败: {}", e))?;
        Ok(())
    });
}

// ─── 工具函数 ─────────────────────────────────────────────────────────────────

/// 从 SDP 中提取流媒体目标 IP 和端口
///
/// GB28181 规范使用标准 SDP，关键字段：
/// - `c=IN IP4 <ip>` — 流媒体目标 IP
/// - `m=video <port> RTP/AVP 96` — RTP 端口
fn parse_sdp_destination(sdp: &str) -> (String, u16) {
    let mut ip = "0.0.0.0".to_string();
    let mut port = 0u16;

    for line in sdp.lines() {
        if let Some(rest) = line.strip_prefix("c=IN IP4 ") {
            ip = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("m=video ") {
            if let Some(port_str) = rest.split_whitespace().next() {
                port = port_str.parse().unwrap_or(0);
            }
        }
    }
    (ip, port)
}

/// 构造 SDP answer（GB28181 PS 流格式）
///
/// payload type 96 = PS（GB28181 标准），98 = H.264
fn build_sdp_answer(local_ip: &str, rtp_port: u16, offer: &str) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 提取 ssrc（GB28181 要求 y= 字段携带 SSRC）
    let ssrc = extract_sdp_field(offer, "y=").unwrap_or_else(|| "0000000001".to_string());

    format!(
        "v=0\r\n\
         o=- {session_id} {session_id} IN IP4 {local_ip}\r\n\
         s=Play\r\n\
         c=IN IP4 {local_ip}\r\n\
         t=0 0\r\n\
         m=video {rtp_port} RTP/AVP 96\r\n\
         a=rtpmap:96 PS/90000\r\n\
         a=sendonly\r\n\
         y={ssrc}\r\n",
        session_id = session_id,
        local_ip = local_ip,
        rtp_port = rtp_port,
        ssrc = ssrc
    )
}

/// 从 GB28181 XML 中提取指定字段值
///
/// 例：`<CmdType>Catalog</CmdType>` → `Some("Catalog")`
fn extract_xml_field(xml: &str, field: &str) -> Option<String> {
    let open = format!("<{}>", field);
    let close = format!("</{}>", field);
    let start = xml.find(&open)? + open.len();
    let end = xml.find(&close)?;
    if start < end {
        Some(xml[start..end].to_string())
    } else {
        None
    }
}

/// 从 SDP 中提取以指定前缀开头的行值
fn extract_sdp_field(sdp: &str, prefix: &str) -> Option<String> {
    sdp.lines()
        .find(|l| l.starts_with(prefix))
        .map(|l| l[prefix.len()..].trim().to_string())
}
