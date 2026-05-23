//! 录像回放 / 录像下载 / 回放控制 API
//!
//! 所有 handler 使用 `DiComp<Gb28181Server>` 从 DI 容器提取 GB28181 服务实例。
//! 返回类型统一为 `R<T>`（而非 `impl IntoResponse`），确保 axum 能正确推断类型。

use axum::{
    extract::{Path, Json as ExtJson},
};
use serde::{Deserialize, Serialize};
use tx_di_axum::{DiComp, R};
use tx_di_gb28181::{Gb28181Server, PlayUrls, PlaybackControl};

// ============ 录像查询 ============

/// 录像查询请求体
#[derive(Deserialize)]
pub struct RecordQueryReq {
    /// 通道 ID
    pub channel_id: String,
    /// 开始时间（ISO8601，如 "2024-01-01T00:00:00"）
    pub start_time: String,
    /// 结束时间
    pub end_time: String,
    /// 录像类型：0=全部，1=定时，2=报警，3=手动
    #[serde(default)]
    pub record_type: u8,
}

/// POST /api/v1/gb28181/devices/:id/records/query — 查询录像文件列表
///
/// 向设备发送录像查询 MESSAGE，设备回复后触发 SSE 事件 `RecordInfoReceived`。
pub async fn query_records(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<RecordQueryReq>,
) -> R<String> {
    match srv
        .query_record_info(&id, &req.channel_id, &req.start_time, &req.end_time, req.record_type)
        .await
    {
        Ok(_) => R::ok("录像查询已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ============ 历史回放 ============

/// 历史回放请求体
#[derive(Deserialize)]
pub struct PlaybackStartReq {
    /// 通道 ID
    pub channel_id: String,
    /// 回放开始时间（ISO8601）
    pub start_time: String,
    /// 回放结束时间（ISO8601）
    pub end_time: String,
}

/// 历史回放响应体
#[derive(Serialize)]
pub struct PlaybackResp {
    pub call_id: String,
    pub urls: PlayUrls,
}

/// POST /api/v1/gb28181/devices/:id/playback/start — 发起历史回放
///
/// 通过 INVITE (s=Playback) 向设备请求历史视频流。
/// 返回 call_id 用于后续控制/挂断，以及各协议播放地址。
pub async fn start_playback(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<PlaybackStartReq>,
) -> R<PlaybackResp> {
    match srv
        .invite_playback(&id, &req.channel_id, &req.start_time, &req.end_time)
        .await
    {
        Ok((call_id, urls)) => R::ok(PlaybackResp { call_id, urls }),
        Err(e) => R::fail(e.to_string()),
    }
}

// ============ 回放控制 ============

/// 回放控制请求体
///
/// 支持暂停、继续、快放、慢放、拖动、停止。
/// 快放/慢放的 speed 参数为倍速：1/2/4/8。
/// 拖动的 time 参数为目标时间点（ISO8601）。
#[derive(Deserialize)]
#[serde(tag = "cmd")]
pub enum PlaybackCtrlReq {
    /// 暂停
    #[serde(rename = "pause")]
    Pause,
    /// 继续
    #[serde(rename = "resume")]
    Resume,
    /// 快放（speed: 1/2/4/8）
    #[serde(rename = "fast_forward")]
    FastForward { speed: u8 },
    /// 慢放（speed: 1/2/4/8）
    #[serde(rename = "slow_forward")]
    SlowForward { speed: u8 },
    /// 拖动到指定时间
    #[serde(rename = "seek")]
    Seek { time: String },
    /// 停止
    #[serde(rename = "stop")]
    Stop,
}

impl From<PlaybackCtrlReq> for PlaybackControl {
    fn from(val: PlaybackCtrlReq) -> Self {
        match val {
            PlaybackCtrlReq::Pause => PlaybackControl::Pause,
            PlaybackCtrlReq::Resume => PlaybackControl::Resume,
            PlaybackCtrlReq::FastForward { speed } => PlaybackControl::FastForward(speed),
            PlaybackCtrlReq::SlowForward { speed } => PlaybackControl::SlowForward(speed),
            PlaybackCtrlReq::Seek { time } => PlaybackControl::Seek(time),
            PlaybackCtrlReq::Stop => PlaybackControl::Stop,
        }
    }
}

/// POST /api/v1/gb28181/devices/:id/playback/control — 回放控制
///
/// GB28181-2022 §9.2：向设备发送回放控制指令（暂停/继续/快放/慢放/拖动/停止）。
pub async fn playback_control(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<PlaybackCtrlReq>,
) -> R<String> {
    let ctrl: PlaybackControl = req.into();
    match srv.playback_control(&id, ctrl).await {
        Ok(_) => R::ok("回放控制指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ============ 录像控制（录制启停） ============

/// 录像控制请求体
#[derive(Deserialize)]
pub struct RecordControlReq {
    /// 通道 ID
    pub channel_id: String,
    /// true=开始录像，false=停止录像
    pub start: bool,
}

/// POST /api/v1/gb28181/devices/:id/record/control — 录像控制
///
/// 远程控制设备端录像的启停。
pub async fn record_control(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<RecordControlReq>,
) -> R<String> {
    match srv.record_control(&id, &req.channel_id, req.start).await {
        Ok(_) => R::ok("录像控制指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ============ 录像下载 ============

/// 录像下载请求体
#[derive(Deserialize)]
pub struct DownloadStartReq {
    /// 通道 ID
    pub channel_id: String,
    /// 下载速度（可选）
    #[serde(default)]
    pub download_speed: Option<u32>,
}

/// POST /api/v1/gb28181/devices/:id/download/start — 发起录像下载
///
/// 通过 INVITE (s=Download) 向设备请求录像文件下载。
/// 返回 call_id 和播放地址。
pub async fn start_download(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<DownloadStartReq>,
) -> R<PlaybackResp> {
    match srv.invite_download(&id, &req.channel_id, req.download_speed).await {
        Ok((call_id, urls)) => R::ok(PlaybackResp { call_id, urls }),
        Err(e) => R::fail(e.to_string()),
    }
}
