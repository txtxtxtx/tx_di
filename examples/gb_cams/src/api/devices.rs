//! 设备管理 API

use axum::{
    extract::{Path, Json as ExtJson},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::device::DeviceManager;
use crate::dto::{ChannelDto, DeviceDto, StatsDto};

/// 通用 API 响应包装
#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    code: i32,
    message: String,
    data: T,
}

fn ok<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse { code: 0, message: "ok".into(), data })
}

fn err<T: Serialize + Default>(code: i32, msg: impl Into<String>) -> Json<ApiResponse<T>> {
    Json(ApiResponse { code, message: msg.into(), data: T::default() })
}

/// GET /api/gb_cams/stats
pub async fn stats() -> impl IntoResponse {
    let mgr = DeviceManager::instance();
    let (total, online, channels) = mgr.stats();
    ok(StatsDto { total, online, channels })
}

/// GET /api/gb_cams/devices
pub async fn list() -> impl IntoResponse {
    let mgr = DeviceManager::instance();
    let devices: Vec<DeviceDto> = mgr.all_devices().into_iter().map(DeviceDto::from).collect();
    ok(devices)
}

/// GET /api/gb_cams/devices/:id
pub async fn detail(Path(id): Path<String>) -> impl IntoResponse {
    let mgr = DeviceManager::instance();
    match mgr.get_device(&id) {
        Some(dev) => {
            let channels: Vec<ChannelDto> = dev.channels.iter().map(ChannelDto::from).collect();
            let mut dto = DeviceDto::from(dev);
            dto.channels = Some(channels);
            Json(ApiResponse { code: 0, message: "ok".into(), data: Some(dto) })
        }
        None => {
            let msg = format!("设备 {} 不存在", id);
            Json(ApiResponse { code: 404, message: msg, data: None::<DeviceDto> })
        }
    }
}

/// 创建单个设备请求体
#[derive(Deserialize)]
pub struct CreateDeviceReq {
    pub device_id: String,
    pub name: Option<String>,
    pub channels: Option<Vec<CreateChannelReq>>,
}

#[derive(Deserialize)]
pub struct CreateChannelReq {
    pub channel_id: String,
    pub name: Option<String>,
}

/// POST /api/gb_cams/devices — 创建单个设备
pub async fn create(ExtJson(req): ExtJson<CreateDeviceReq>) -> impl IntoResponse {
    let mgr = DeviceManager::instance();
    let channels: Vec<(String, String)> = req.channels
        .unwrap_or_default()
        .into_iter()
        .map(|ch| {
            let name = ch.name.unwrap_or_else(|| format!("CH-{}", &ch.channel_id[ch.channel_id.len().saturating_sub(4)..]));
            (ch.channel_id, name)
        })
        .collect();

    let name = req.name.unwrap_or_else(|| format!("Cam-{}", &req.device_id[req.device_id.len().saturating_sub(6)..]));
    let device_id = req.device_id.clone();
    mgr.add_device(req.device_id, channels, name);

    ok(serde_json::json!({
        "device_id": device_id,
        "message": "设备已创建"
    }))
}

/// 批量生成请求体
#[derive(Deserialize)]
pub struct GenerateReq {
    /// 设备数量
    pub count: usize,
    /// 每个设备通道数
    #[serde(default = "default_channels")]
    pub channels_per_device: usize,
    /// 设备ID前缀（14位）
    #[serde(default = "default_prefix")]
    pub prefix: String,
    /// 起始序号
    #[serde(default = "default_base_seq")]
    pub base_seq: u64,
    /// 生成后是否自动注册
    #[serde(default)]
    pub auto_register: bool,
}
fn default_channels() -> usize { 1 }
fn default_prefix() -> String { "34020000001320".to_string() }
fn default_base_seq() -> u64 { 1 }

/// POST /api/gb_cams/devices/generate — 批量随机生成
pub async fn generate(ExtJson(req): ExtJson<GenerateReq>) -> impl IntoResponse {
    if req.count == 0 || req.count > 1000 {
        return err::<serde_json::Value>(-1, "设备数量须在 1-1000 之间");
    }
    if req.channels_per_device == 0 || req.channels_per_device > 64 {
        return err::<serde_json::Value>(-1, "通道数量须在 1-64 之间");
    }

    let mgr = DeviceManager::instance();
    let devices = crate::generator::generate_devices(
        req.count, req.channels_per_device, &req.prefix, req.base_seq,
    );

    let mut device_ids = Vec::new();
    for (device_id, channel_ids, name) in devices {
        let channels: Vec<(String, String)> = channel_ids
            .into_iter()
            .enumerate()
            .map(|(i, cid)| (cid, format!("CH-{:02}", i + 1)))
            .collect();
        mgr.add_device(device_id.clone(), channels, name);
        device_ids.push(device_id);
    }

    if req.auto_register {
        mgr.start_all();
    }

    ok(serde_json::json!({
        "count": device_ids.len(),
        "device_ids": device_ids,
        "auto_register": req.auto_register,
    }))
}

/// DELETE /api/gb_cams/devices/:id — 删除设备
pub async fn remove(Path(id): Path<String>) -> impl IntoResponse {
    let mgr = DeviceManager::instance();
    if mgr.remove_device(&id).await {
        ok(serde_json::json!({ "device_id": id }))
    } else {
        err::<serde_json::Value>(404, format!("设备 {} 不存在", id))
    }
}
