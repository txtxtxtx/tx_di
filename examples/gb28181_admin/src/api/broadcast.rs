//! 语音广播 / 对讲 API
//!
//! 所有 handler 使用 `DiComp<Gb28181Server>` 从 DI 容器提取 GB28181 服务实例。
//! 返回类型统一为 `R<T>`（而非 `impl IntoResponse`），确保 axum 能正确推断类型。

use axum::{
    extract::{Path, Json as ExtJson},
};
use serde::Deserialize;
use tx_di_axum::{DiComp, R};
use tx_di_gb28181::{Gb28181Server, AudioCodec};

// ============ 语音广播 ============

/// POST /api/v1/gb28181/devices/:id/broadcast/invite — 发起语音广播邀请
///
/// GB28181-2022 §9.12：平台向设备发起语音广播邀请（MESSAGE Broadcast）。
/// 设备收到后会向平台推送音频流，触发 `BroadcastInviteReceived` SSE 事件。
pub async fn broadcast_invite(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
) -> R<String> {
    match srv.broadcast_invite(&id).await {
        Ok(_) => R::ok("语音广播邀请已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// 广播确认请求体
#[derive(Deserialize)]
pub struct BroadcastAcceptReq {
    /// 平台接收音频的 RTP 端口
    pub audio_port: u16,
}

/// POST /api/v1/gb28181/devices/:id/broadcast/accept — 确认接收广播音频
///
/// 收到 `BroadcastInviteReceived` 事件后调用，告知设备音频接收端口。
/// 触发 `BroadcastSessionStarted` 事件。
pub async fn broadcast_accept(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<BroadcastAcceptReq>,
) -> R<String> {
    match srv.broadcast_accept(&id, req.audio_port).await {
        Ok(_) => R::ok(format!("已确认广播接收，监听端口 {}", req.audio_port)),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/broadcast/stop — 结束语音广播
///
/// 发送 Broadcast Cancel 指令，清理广播会话状态。
/// 触发 `BroadcastSessionEnded` 事件。
pub async fn broadcast_stop(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
) -> R<String> {
    match srv.broadcast_stop(&id).await {
        Ok(_) => R::ok("语音广播已结束".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ============ 语音对讲 ============

/// 对讲请求体
#[derive(Deserialize)]
pub struct TalkbackReq {
    /// 通道 ID
    pub channel_id: String,
    /// 平台发送音频的 RTP 端口
    pub audio_port: u16,
    /// 音频编码（可选，默认 PCMU）
    #[serde(default)]
    pub codec: Option<String>,
}

/// 对讲响应体
use serde::Serialize;

#[derive(Serialize)]
pub struct TalkbackResp {
    pub call_id: String,
    pub device_ip: String,
    pub device_audio_port: u16,
}

/// POST /api/v1/gb28181/devices/:id/talkback/start — 发起语音对讲
///
/// GB28181-2022 §9.12：平台向设备发起双向对讲 INVITE（带音频 SDP）。
/// 返回会话 ID 和设备端音频地址，用于后续音频数据传输。
pub async fn start_talkback(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<TalkbackReq>,
) -> R<TalkbackResp> {
    // 解析 codec 字符串 → AudioCodec 枚举
    let codec = req.codec.as_deref().and_then(|s| match s.to_lowercase().as_str() {
        "pcma" => Some(AudioCodec::PCMA),
        "aac" => Some(AudioCodec::AAC),
        "g7221" | "g722" => Some(AudioCodec::G7221),
        _ => None, // 默认 PCMU
    });

    match srv
        .audio_talkback(&id, &req.channel_id, req.audio_port, codec)
        .await
    {
        Ok((call_id, device_ip, device_audio_port)) => R::ok(TalkbackResp {
            call_id,
            device_ip,
            device_audio_port,
        }),
        Err(e) => R::fail(e.to_string()),
    }
}
