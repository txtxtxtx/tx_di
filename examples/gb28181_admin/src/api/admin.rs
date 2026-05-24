//! 管理能力增强 API — 校时/配置/看守位/预置位/巡航/审计日志/扩展控制
//!
//! ## 功能清单
//!
//! ### 网络校时
//! - `POST /devices/:id/time_sync`     — 向设备查询当前时间
//! - `POST /devices/:id/sync_time`      — 向设备下发标准时间
//!
//! ### 配置管理
//! - `POST /devices/:id/config`         — 查询设备配置（Basic/Network/Video）
//!
//! ### 看守位
//! - `POST  /devices/:id/guard/control` — 看守位控制（设置/调用/清除）
//! - `POST  /devices/:id/guard/info`    — 查询看守位信息
//!
//! ### 预置位 & 巡航
//! - `POST /devices/:id/preset/goto`    — 调用预置位
//! - `POST /devices/:id/preset/set`     — 设置预置位
//! - `POST /devices/:id/cruise/start`   — 启动巡航
//! - `POST /devices/:id/cruise/stop`    — 停止巡航
//! - `POST /devices/:id/cruise/list`    — 查询巡航轨迹列表
//!
// ══════════════════════════════════
//  扩展设备控制
// ══════════════════════════════════
//! - `POST /devices/:id/make_key_frame`   — 强制关键帧
//! - `POST /devices/:id/zoom/in`          — 拉框放大
//! - `POST /devices/:id/zoom/out`         — 拉框缩小
//! - `POST /devices/:id/ptz_precise`      — PTZ 精准控制
//! - `POST /devices/:id/target_track`     — 目标跟踪
//! - `POST /devices/:id/storage/format`   — 存储卡格式化
//! - `POST /devices/:id/storage/status`   — 存储卡状态查询
//! - `POST /devices/:id/cruise_track`     — 巡航轨迹详情查询
//! - `POST /devices/:id/ptz_precise_status` — PTZ 精准状态查询
//! - `POST /devices/:id/guard/basic`      — 布撤防（基础版）
//! - `POST /devices/:id/playback_ctrl`    — 回放控制
//!
//! ### 统计与报表
//! - `GET /gb28181/dashboard`             — 增强仪表盘（含报警统计）
//!
//! ### 审计日志
//! - `GET  /gb28181/audit_logs`           — 审计日志列表（分页+筛选）
//! - `GET  /gb28181/audit_logs/:id`       — 审计日志详情

use axum::{
    extract::{Path, Query, State, Json as ExtJson},
};
use serde::{Deserialize, Serialize};
use tx_di_axum::{DiComp, R};
use tx_di_gb28181::Gb28181Server;
use tx_di_gb28181::xml::{ConfigType, GuardMode, PtzPreciseParam, ZoomRect, PlaybackControl};
use toasty::Db;

use crate::dto::{PageData, Pagination};
use crate::models::{GbAlarmRecord, GbAuditLog};

// ══════════════════════════════════
//  网络校时
// ══════════════════════════════════

/// POST /api/v1/gb28181/devices/:id/time_sync — 向设备查询当前时间
///
/// GB28181-2022 §9.10：平台向设备发送时间查询请求。
/// 设备响应后触发 `Gb28181Event::TimeSyncResult`。
pub async fn time_sync(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
) -> R<String> {
    match srv.time_sync(&id).await {
        Ok(_) => R::ok("已发送校时查询".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/sync_time — 向设备下发标准时间
///
/// GB28181-2022 §9.10：平台向设备主动下发当前系统时间。
pub async fn sync_time(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
) -> R<String> {
    match srv.sync_time_to_device(&id).await {
        Ok(_) => R::ok("已下发标准时间到设备".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ══════════════════════════════════
//  配置管理
// ══════════════════════════════════

/// 配置查询请求体
#[derive(Deserialize)]
pub struct ConfigQueryReq {
    /// 配置类型：Basic / Network / Video（不区分大小写）
    pub config_type: String,
}

/// POST /api/v1/gb28181/devices/:id/config — 查询设备配置
///
/// 支持三种配置类型：
/// - `BasicParam` — 基本参数（厂商、型号、固件版本等）
/// - `NetworkParam` — 网络参数（IP、子网掩码、网关等）
/// - `VideoParam` — 视频参数（分辨率、码率、帧率等）
///
/// 设备回复后触发 `Gb28181Event::ConfigDownloaded`。
pub async fn query_config(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<ConfigQueryReq>,
) -> R<String> {
    let ct = match req.config_type.to_lowercase().as_str() {
        "basic" | "basicparam" => ConfigType::Basic,
        "network" | "networkparam" => ConfigType::Network,
        "video" | "videoparam" => ConfigType::Video,
        _ => return R::error(400, format!("无效的配置类型: {}，支持 Basic/Network/Video", req.config_type)),
    };
    match srv.query_config(&id, ct).await {
        Ok(_) => R::ok(format!("已发送{}配置查询", ct.as_str())),
        Err(e) => R::fail(e.to_string()),
    }
}

// ══════════════════════════════════
//  看守位控制
// ══════════════════════════════════

/// 看守位控制请求体
#[derive(Deserialize)]
pub struct GuardControlReq {
    /// 通道 ID
    pub channel_id: String,
    /// 控制模式：set（设置看守位） / call（调用看守位） / clear（清除看守位）
    pub mode: String,
    /// 预置位编号（0-255）
    #[serde(default)]
    pub preset_index: u8,
}

/// POST /api/v1/gb28181/devices/:id/guard/control — 看守位控制
///
/// GB28181-2022 A.2.3.1.10：看守位控制（v2 增强版，支持 Set/Call/Clear 三种模式）
pub async fn guard_control(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<GuardControlReq>,
) -> R<String> {
    let mode = match req.mode.to_lowercase().as_str() {
        "set" => GuardMode::Set,
        "call" => GuardMode::Call,
        "clear" => GuardMode::Clear,
        _ => return R::error(400, format!("无效的看守位模式: {}，支持 set/call/clear", req.mode)),
    };
    match srv.guard_control_v2(&id, &req.channel_id, mode, req.preset_index).await {
        Ok(_) => R::ok("看守位指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/guard/info — 查询看守位信息
///
/// GB28181-2022 A.2.4.10：看守位信息查询（2022 新增）
pub async fn guard_info(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
) -> R<String> {
    match srv.query_guard_info(&id).await {
        Ok(_) => R::ok("已发送看守位信息查询".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ══════════════════════════════════
//  预置位控制
// ══════════════════════════════════

/// 预置位操作请求体
#[derive(Deserialize)]
pub struct PresetReq {
    /// 通道 ID
    pub channel_id: String,
    /// 预置位编号（0-255）
    pub preset_index: u8,
}

/// POST /api/v1/gb28181/devices/:id/preset/goto — 调用预置位
pub async fn goto_preset(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<PresetReq>,
) -> R<String> {
    match srv.goto_preset(&id, &req.channel_id, req.preset_index).await {
        Ok(_) => R::ok("预置位调用指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/preset/set — 设置预置位
pub async fn set_preset(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<PresetReq>,
) -> R<String> {
    match srv.set_preset(&id, &req.channel_id, req.preset_index).await {
        Ok(_) => R::ok("预置位设置指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ══════════════════════════════════
//  巡航控制
// ══════════════════════════════════

/// 巡航操作请求体
#[derive(Deserialize)]
pub struct CruiseReq {
    /// 通道 ID
    pub channel_id: String,
    /// 巡航轨迹编号（0-255）
    pub cruise_no: u8,
}

/// POST /api/v1/gb28181/devices/:id/cruise/start — 启动巡航
pub async fn start_cruise(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<CruiseReq>,
) -> R<String> {
    match srv.start_cruise(&id, &req.channel_id, req.cruise_no).await {
        Ok(_) => R::ok("巡航启动指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/cruise/stop — 停止巡航
pub async fn stop_cruise(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<CruiseReq>,
) -> R<String> {
    match srv.stop_cruise(&id, &req.channel_id, req.cruise_no).await {
        Ok(_) => R::ok("巡航停止指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// 巡航列表请求体
#[derive(Deserialize)]
pub struct CruiseListReq {
    /// 通道 ID
    pub channel_id: String,
}

/// POST /api/v1/gb28181/devices/:id/cruise/list — 查询巡航轨迹列表
pub async fn cruise_list(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<CruiseListReq>,
) -> R<String> {
    match srv.query_cruise_list(&id, &req.channel_id).await {
        Ok(_) => R::ok("已发送巡航轨迹列表查询".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ══════════════════════════════════
//  扩展设备控制
// ══════════════════════════════════

/// 通道级操作请求体（仅需 channel_id）
#[derive(Deserialize)]
pub struct ChannelOpReq {
    pub channel_id: String,
}

/// POST /devices/:id/make_key_frame — 强制关键帧
///
/// GB28181-2022 A.2.3.1.7
pub async fn make_key_frame(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<ChannelOpReq>,
) -> R<String> {
    match srv.make_video_record(&id, &req.channel_id).await {
        Ok(_) => R::ok("强制关键帧请求已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// 拉框缩放请求体
#[derive(Deserialize)]
pub struct ZoomReq {
    pub channel_id: String,
    /// 左上角 X（归一化坐标 0-65535）
    pub x1: u16,
    /// 左上角 Y
    pub y1: u16,
    /// 右下角 X
    pub x2: u16,
    /// 右下角 Y
    pub y2: u16,
}

/// POST /devices/:id/zoom/in — 拉框放大
pub async fn zoom_in(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<ZoomReq>,
) -> R<String> {
    let rect = ZoomRect {
        x1: req.x1,
        y1: req.y1,
        x2: req.x2,
        y2: req.y2,
    };
    match srv.zoom_in(&id, &req.channel_id, rect).await {
        Ok(_) => R::ok("拉框放大指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /devices/:id/zoom/out — 拉框缩小
pub async fn zoom_out(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<ZoomReq>,
) -> R<String> {
    let rect = ZoomRect {
        x1: req.x1,
        y1: req.y1,
        x2: req.x2,
        y2: req.y2,
    };
    match srv.zoom_out(&id, &req.channel_id, rect).await {
        Ok(_) => R::ok("拉框缩小指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// PTZ 精准控制请求体
#[derive(Deserialize)]
pub struct PtzPreciseReq {
    pub channel_id: String,
    /// 水平角度（-360 ~ 360，0.1° 精度，实际传输时 x10）
    pub pan: f64,
    /// 垂直角度（-90 ~ 90，0.1° 精度，实际传输时 x10）
    pub tilt: f64,
    /// 变倍倍数（1x ~ 16x，精度根据设备而定）
    pub zoom: f64,
}

/// POST /devices/:id/ptz_precise — PTZ 精准控制（绝对位置）
///
/// GB28181-2022 A.2.3.1.11
pub async fn ptz_precise(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<PtzPreciseReq>,
) -> R<String> {
    let param = PtzPreciseParam {
        pan_position: (req.pan * 100.0).round() as u16,
        tilt_position: (req.tilt * 100.0).round() as u16,
        zoom_position: (req.zoom * 100.0).round() as u16,
        focus_position: None,
        iris_position: None,
    };
    match srv.ptz_precise_control(&id, &req.channel_id, param).await {
        Ok(_) => R::ok("PTZ 精准控制指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// 目标跟踪请求体
#[derive(Deserialize)]
pub struct TargetTrackReq {
    pub channel_id: String,
    /// true=启动跟踪，false=停止跟踪
    pub start: bool,
}

/// POST /devices/:id/target_track — 目标跟踪控制
///
/// GB28181-2022 A.2.3.1.14（2022 新增）
pub async fn target_track(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<TargetTrackReq>,
) -> R<String> {
    match srv.target_track(&id, &req.channel_id, req.start).await {
        Ok(_) => R::ok(if req.start { "目标跟踪已启动".to_string() } else { "目标跟踪已停止".to_string() }),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /devices/:id/storage/format — 存储卡格式化
///
/// GB28181-2022 A.2.3.1.13
pub async fn storage_format(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<ChannelOpReq>,
) -> R<String> {
    match srv.storage_format(&id, &req.channel_id).await {
        Ok(_) => R::ok("存储卡格式化指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /devices/:id/storage/status — 存储卡状态查询
///
/// GB28181-2022 A.2.4.14（2022 新增）
pub async fn storage_status(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<ChannelOpReq>,
) -> R<String> {
    match srv.query_storage_status(&id, &req.channel_id).await {
        Ok(_) => R::ok("存储卡状态查询已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// 巡航轨迹详情请求体
#[derive(Deserialize)]
pub struct CruiseTrackReq {
    pub channel_id: String,
    /// 巡航轨迹 ID
    pub cruise_id: String,
}

/// POST /devices/:id/cruise_track — 巡航轨迹详情查询
///
/// GB28181-2022 A.2.4.12（2022 新增）
pub async fn cruise_track(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<CruiseTrackReq>,
) -> R<String> {
    match srv.query_cruise_track(&id, &req.channel_id, &req.cruise_id).await {
        Ok(_) => R::ok("巡航轨迹详情查询已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /devices/:id/ptz_precise_status — PTZ 精准状态查询
///
/// GB28181-2022 A.2.4.13（2022 新增）
pub async fn ptz_precise_status(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
) -> R<String> {
    match srv.query_ptz_precise_status(&id).await {
        Ok(_) => R::ok("PTZ 精准状态查询已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// 布撤防请求体
#[derive(Deserialize)]
pub struct GuardBasicReq {
    pub channel_id: String,
    /// true=布防，false=撤防
    pub guard: bool,
}

/// POST /devices/:id/guard/basic — 布撤防控制（基础版）
///
/// 通过 PTZ 命令中的布撤防功能实现。
pub async fn guard_basic(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<GuardBasicReq>,
) -> R<String> {
    match srv.guard_control(&id, &req.channel_id, req.guard).await {
        Ok(_) => R::ok(if req.guard { "布防指令已发送".to_string() } else { "撤防指令已发送".to_string() }),
        Err(e) => R::fail(e.to_string()),
    }
}

/// 回放控制请求体
#[derive(Deserialize)]
pub struct PlaybackCtrlReq {
    /// 控制类型：pause/resume/tear_down/drag/forward/backward
    pub ctrl: String,
    /// 拖动位置或速度参数（可选）
    #[serde(default)]
    pub value: Option<f64>,
}

impl PlaybackCtrlReq {
    /// 将字符串映射为 PlaybackControl 枚举
    fn to_playback_control(&self) -> Result<PlaybackControl, String> {
        match self.ctrl.to_lowercase().as_str() {
            "pause" => Ok(PlaybackControl::Pause),
            "resume" => Ok(PlaybackControl::Resume),
            "stop" | "teardown" | "tear_down" => Ok(PlaybackControl::Stop),
            "seek" | "drag" => {
                // value 为时间字符串（格式 "YYYY-MM-DDTHH:MM:SS"）或归一化位置
                self.value.map(|v| {
                    // 归一化位置转为秒数偏移（示例：v=0.5 表示 50% 位置）
                    // 实际 GB28181 使用绝对时间字符串，这里用简单转换
                    PlaybackControl::Seek(format!("{:.0}", v))
                }).ok_or_else(|| "seek 操作需要 value 参数".to_string())
            },
            "fast_forward" | "forward" => {
                let speed = self.value.unwrap_or(2.0) as u8;
                Ok(PlaybackControl::FastForward(speed))
            },
            "slow_forward" | "backward" => {
                let speed = self.value.unwrap_or(2.0) as u8;
                Ok(PlaybackControl::SlowForward(speed))
            },
            _ => Err(format!(
                "无效的回放控制类型: {}，支持 pause/resume/stop/seek/fast_forward/slow_forward",
                self.ctrl
            )),
        }
    }
}

/// POST /devices/:id/playback_ctrl — 回放控制
///
/// GB28181-2022 §9.2：暂停/继续/拖动/快放等
pub async fn playback_ctrl(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<PlaybackCtrlReq>,
) -> R<String> {
    let ctrl = match req.to_playback_control() {
        Ok(c) => c,
        Err(e) => return R::error(400, e),
    };
    match srv.playback_control(&id, ctrl).await {
        Ok(_) => R::ok("回放控制指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ══════════════════════════════════
//  统计与报表
// ══════════════════════════════════

/// 增强仪表盘数据
#[derive(Serialize)]
pub struct DashboardDto {
    /// 设备总数
    pub total_devices: usize,
    /// 在线设备数
    pub online_devices: usize,
    /// 活跃会话数
    pub active_sessions: usize,
    /// 报警记录总数
    pub total_alarms: u64,
    /// 未处理报警数
    pub pending_alarms: u64,
    /// 已处理报警数
    pub handled_alarms: u64,
}

/// GET /api/v1/gb28181/dashboard — 增强仪表盘统计
///
/// 组合内存实时数据 + 数据库聚合统计数据。
pub async fn dashboard(
    State(mut db): State<Db>,
    srv: DiComp<Gb28181Server>,
) -> R<DashboardDto> {
    // 内存实时数据
    let total_devices = srv.device_count();
    let online_devices = srv.online_count();
    let active_sessions = srv.active_sessions().len();

    // 数据库聚合：报警统计
    let total_alarms = match GbAlarmRecord::all().count().exec(&mut db).await {
        Ok(n) => n,
        Err(_) => 0,
    };

    // 未处理报警（status = 0）— 用内存过滤（status 无 index 时用通用查询）
    let pending_records = match GbAlarmRecord::all()
        .limit(10000)   // 合理上限
        .exec(&mut db)
        .await
    {
        Ok(r) => r,
        Err(_) => vec![],
    };
    let pending_alarms = pending_records.iter().filter(|r| r.status == 0).count() as u64;
    let handled_alarms = total_alarms.saturating_sub(pending_alarms);

    R::ok(DashboardDto {
        total_devices,
        online_devices,
        active_sessions,
        total_alarms,
        pending_alarms,
        handled_alarms,
    })
}

// ══════════════════════════════════
//  操作审计日志
// ══════════════════════════════════

/// 审计日志 DTO
#[derive(Serialize)]
pub struct AuditLogDto {
    pub id: u64,
    pub operator: String,
    pub action: String,
    pub target: String,
    pub detail: String,
    pub client_ip: String,
    pub user_agent: String,
    pub result: String,
    pub created_at: String,
}

impl From<GbAuditLog> for AuditLogDto {
    fn from(r: GbAuditLog) -> Self {
        Self {
            id: r.id,
            operator: r.operator,
            action: r.action,
            target: r.target,
            detail: r.detail,
            client_ip: r.client_ip,
            user_agent: r.user_agent,
            result: r.result,
            created_at: r.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

/// 审计日志查询参数
#[derive(Debug, Deserialize)]
pub struct AuditLogQueryParams {
    /// 按操作人筛选
    pub operator: Option<String>,
    /// 按操作类型筛选
    pub action: Option<String>,
    /// 按操作结果筛选
    pub result: Option<String>,
    /// 分页参数
    #[serde(flatten)]
    pub pagination: Pagination,
}

/// GET /api/v1/gb28181/audit_logs — 审计日志列表（分页）
///
/// 从数据库查询操作审计日志，支持按 operator/action/result 筛选。
/// GbAuditLog 的 action 字段有 #[index]，支持 filter_by_action 高效查询。
pub async fn list_audit_logs(
    State(mut db): State<Db>,
    Query(qp): Query<AuditLogQueryParams>,
) -> R<PageData<AuditLogDto>> {
    // 总数
    let total = if qp.operator.is_some() || qp.action.is_some() || qp.result.is_some() {
        // 有筛选条件时使用 filter_by_action 作为主索引入口
        let base_query = match &qp.action {
            Some(act) => GbAuditLog::filter_by_action(act.clone()),
            None => GbAuditLog::all(),
        };
        match base_query.count().exec(&mut db).await {
            Ok(n) => n,
            Err(e) => return R::error(500, format!("查询审计日志总数失败: {}", e)),
        }
    } else {
        match GbAuditLog::all().count().exec(&mut db).await {
            Ok(n) => n,
            Err(e) => return R::error(500, format!("查询审计日志总数失败: {}", e)),
        }
    };

    // 分页查询（按 ID 倒序：最新的在前）
    let offset = qp.pagination.offset();
    let page_size = qp.pagination.page_size as usize;
    let records = if let Some(ref act) = qp.action {
        GbAuditLog::filter_by_action(act.clone())
            .offset(offset as usize)
            .limit(page_size)
            .exec(&mut db)
            .await
    } else {
        GbAuditLog::all()
            .offset(offset as usize)
            .limit(page_size)
            .exec(&mut db)
            .await
    };

    let records = match records {
        Ok(r) => r,
        Err(e) => return R::error(500, format!("查询审计日志失败: {}", e)),
    };

    // 内存二次筛选（operator 和 result 条件）
    let filtered: Vec<AuditLogDto> = records
        .into_iter()
        .map(AuditLogDto::from)
        .filter(|l| {
            if let Some(ref op) = qp.operator {
                if l.operator != *op { return false; }
            }
            if let Some(ref res) = qp.result {
                if l.result != *res { return false; }
            }
            true
        })
        .collect();

    R::ok(PageData::from_offset(filtered, total, qp.pagination))
}

/// GET /api/v1/gb28181/audit_logs/:id — 审计日志详情
pub async fn get_audit_log(
    State(mut db): State<Db>,
    Path(id): Path<u64>,
) -> R<AuditLogDto> {
    match GbAuditLog::get_by_id(&mut db, id).await {
        Ok(record) => R::ok(AuditLogDto::from(record)),
        Err(e) => R::error(404, format!("审计日志不存在: {}", e)),
    }
}

// ══════════════════════════════════
//  公共辅助函数
// ══════════════════════════════════

/// 写操作审计日志（公共辅助函数）
///
/// 供 group.rs / audit.rs 等模块调用，记录管理员操作。
/// 写入失败仅 warn 不影响主流程。
pub async fn write_audit(
    db: &mut Db,
    operator: &str,
    action: &str,
    target: &str,
    detail: &str,
) -> Result<(), String> {
    match toasty::create!(GbAuditLog {
        operator: operator.to_string(),
        action: action.to_string(),
        target: target.to_string(),
        detail: detail.to_string(),
        client_ip: String::new(),
        user_agent: String::new(),
        result: "ok".to_string(),
    })
    .exec(db)
    .await
    {
        Ok(_) => {}
        Err(e) => tracing::warn!("写入审计日志失败: {}", e),
    }
    Ok(())
}
