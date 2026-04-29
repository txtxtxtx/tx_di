//! 设备相关 API 处理器

use axum::{
    extract::{Path, Json as ExtJson},
    response::IntoResponse,
};
use serde::Deserialize;
use tx_di_axum::R;
use tx_di_core::ApiR;
use tx_di_gb28181::Gb28181Server;
use tx_di_gb28181::xml::{PtzCommand, PtzSpeed};

use crate::dto::{ChannelDto, DeviceDto, StatsDto};

/// GET /api/gb28181/stats — 统计概要
pub async fn stats() -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    let dto = StatsDto {
        total: srv.device_count(),
        online: srv.online_count(),
        sessions: srv.active_sessions().len(),
    };
    R::from(ApiR::success(dto))
}

/// GET /api/gb28181/devices — 所有设备列表
pub async fn list() -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    let devices: Vec<DeviceDto> = srv
        .online_devices()
        .into_iter()
        .map(DeviceDto::from)
        .collect();
    R::from(ApiR::success(devices))
}

/// GET /api/gb28181/devices/:id — 设备详情（含通道）
pub async fn detail(Path(id): Path<String>) -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    match srv.get_device(&id) {
        Some(dev) => {
            let channels: Vec<ChannelDto> = dev.channels.iter().map(ChannelDto::from).collect();
            let mut dto = DeviceDto::from(dev);
            dto.channels = Some(channels);
            R::from(ApiR::success(dto))
        }
        None => R::from(ApiR::<DeviceDto>::error_with_data(
            404,
            format!("设备 {} 不存在", id),
            DeviceDto::default(),
        )),
    }
}

/// POST /api/gb28181/devices/:id/catalog — 触发目录查询
pub async fn query_catalog(Path(id): Path<String>) -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    match srv.query_catalog(&id).await {
        Ok(_) => R::from(ApiR::success("已发送目录查询".to_string())),
        Err(e) => R::from(ApiR::<String>::error_with_data(-1, e.to_string(), String::new())),
    }
}

/// POST /api/gb28181/devices/:id/info — 触发设备信息查询
pub async fn query_info(Path(id): Path<String>) -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    match srv.query_device_info(&id).await {
        Ok(_) => R::from(ApiR::success("已发送设备信息查询".to_string())),
        Err(e) => R::from(ApiR::<String>::error_with_data(-1, e.to_string(), String::new())),
    }
}

/// POST /api/gb28181/devices/:id/status — 触发设备状态查询
pub async fn query_status(Path(id): Path<String>) -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    match srv.query_device_status(&id).await {
        Ok(_) => R::from(ApiR::success("已发送状态查询".to_string())),
        Err(e) => R::from(ApiR::<String>::error_with_data(-1, e.to_string(), String::new())),
    }
}

/// PTZ 控制请求体
#[derive(Deserialize)]
pub struct PtzReq {
    pub channel_id: String,
    /// 方向: stop/left/right/up/down/upleft/upright/downleft/downright
    pub direction: String,
    /// 水平速度 0-255
    #[serde(default = "default_speed")]
    pub pan: u8,
    /// 垂直速度 0-255
    #[serde(default = "default_speed")]
    pub tilt: u8,
    /// 变倍速度 0-255
    #[serde(default)]
    pub zoom: u8,
}
fn default_speed() -> u8 { 64 }

/// POST /api/gb28181/devices/:id/ptz — PTZ 控制
pub async fn ptz(
    Path(id): Path<String>,
    ExtJson(req): ExtJson<PtzReq>,
) -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    let speed = PtzSpeed { pan: req.pan, tilt: req.tilt, zoom: req.zoom };
    let cmd = match req.direction.to_lowercase().as_str() {
        "left"      => PtzCommand::Left(speed),
        "right"     => PtzCommand::Right(speed),
        "up"        => PtzCommand::Up(speed),
        "down"      => PtzCommand::Down(speed),
        "upleft"    => PtzCommand::LeftUp(speed),
        "upright"   => PtzCommand::RightUp(speed),
        "downleft"  => PtzCommand::LeftDown(speed),
        "downright" => PtzCommand::RightDown(speed),
        "zoomin"    => PtzCommand::ZoomIn(speed),
        "zoomout"   => PtzCommand::ZoomOut(speed),
        _           => PtzCommand::Stop,
    };
    match srv.ptz_control(&id, &req.channel_id, cmd).await {
        Ok(_) => R::from(ApiR::success("PTZ 指令已发送".to_string())),
        Err(e) => R::from(ApiR::<String>::error_with_data(-1, e.to_string(), String::new())),
    }
}

/// POST /api/gb28181/devices/:id/teleboot — 远程重启
pub async fn teleboot(Path(id): Path<String>) -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    match srv.teleboot(&id).await {
        Ok(_) => R::from(ApiR::success("重启指令已发送".to_string())),
        Err(e) => R::from(ApiR::<String>::error_with_data(-1, e.to_string(), String::new())),
    }
}

/// POST /api/gb28181/devices/:id/alarm_reset — 报警复位
#[derive(Deserialize)]
pub struct AlarmResetReq {
    #[serde(default = "default_alarm_type")]
    pub alarm_type: String,
}
fn default_alarm_type() -> String { "0".to_string() }

pub async fn alarm_reset(
    Path(id): Path<String>,
    ExtJson(req): ExtJson<AlarmResetReq>,
) -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    match srv.alarm_reset(&id, &req.alarm_type).await {
        Ok(_) => R::from(ApiR::success("报警复位指令已发送".to_string())),
        Err(e) => R::from(ApiR::<String>::error_with_data(-1, e.to_string(), String::new())),
    }
}
