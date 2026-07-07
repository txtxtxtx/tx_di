//! INVITE / BYE 处理（UAS 侧）
//!
//! - [`handle_invite`]：作为 UAS 收到点播/语音广播 `INVITE`，
//!   经 [`crate::handler::DeviceHandler::on_invite`] 取 SDP answer 并回复 `200 OK`。
//! - [`handle_bye`]：收到 `BYE` 回复 `200 OK` 并触发 [`DeviceHandler::on_bye`]。
//!
//! > 注：本实现以事务级响应承载点播 answer，覆盖设备端骨架的标准交互；
//! > 若需完整 dialog 状态机（ACK/重协商），可在 gb_cams 迁移阶段替换为
//! > `rsipstack::dialog` 的 server invite。

use std::sync::Arc;

use rsipstack::sip::{ContentType, Header, HeadersExt, StatusCode};
use tracing::warn;
use tx_di_core::RIE;
use tx_di_sip::SipTx;
use tx_gb28181::sip::extract_user_from_sip_uri;

use crate::plugin::Gb28181Device;

/// 作为 UAS 处理 INVITE（点播/语音广播）
pub async fn handle_invite(dev: &Arc<Gb28181Device>, tx: &SipTx) -> RIE<()> {
    // 提取通道 ID（INVITE 的 To 头中的 DeviceID）
    let to = tx
        .request()
        .to_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();
    let channel = extract_user_from_sip_uri(&to).unwrap_or_default();
    let offer = std::str::from_utf8(&tx.request().body)
        .unwrap_or("")
        .to_string();

    let answer = dev.handler().on_invite(&channel, &offer).await;
    if answer.is_empty() {
        warn!(channel = %channel, "on_invite 返回空 SDP，回复 500");
        tx.reply(StatusCode::ServerInternalError)
            .await
            .map_err(|e| anyhow::anyhow!("回复 INVITE 500 失败: {}", e))?;
        return Ok(());
    }

    let encoded = dev.config.version.serialize(&answer);
    let headers = vec![Header::ContentType(ContentType::new("application/sdp"))];
    tx.reply_with(StatusCode::OK, headers, Some(encoded))
        .await
        .map_err(|e| anyhow::anyhow!("回复 INVITE 200 OK 失败: {}", e))?;
    Ok(())
}

/// 处理 BYE（挂断）
pub async fn handle_bye(dev: &Arc<Gb28181Device>, tx: &SipTx) -> RIE<()> {
    let call_id = tx
        .request()
        .call_id_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();
    let to = tx
        .request()
        .to_header()
        .map(|h| h.value().to_string())
        .unwrap_or_default();
    let channel = extract_user_from_sip_uri(&to).unwrap_or_default();
    dev.handler().on_bye(&call_id, &channel).await;
    tx.reply(StatusCode::OK)
        .await
        .map_err(|e| anyhow::anyhow!("回复 BYE 200 OK 失败: {}", e))?;
    Ok(())
}
