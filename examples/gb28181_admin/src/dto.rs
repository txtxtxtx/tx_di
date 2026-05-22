//! 响应 DTO — 序列化友好的数据传输对象
//!
//! GB28181 内部类型（DeviceInfo、ChannelInfo 等）不实现 Serialize，
//! 这里创建镜像 DTO 供 JSON 序列化。

use serde::Serialize;
use tx_gb28181::device::GbDevice;
use tx_di_gb28181::SessionInfo;

/// 统计概要
#[derive(Serialize)]
pub struct StatsDto {
    pub total: usize,
    pub online: usize,
    pub sessions: usize,
}

/// 设备 DTO
#[derive(Serialize, Default)]
pub struct DeviceDto {
    pub device_id: String,
    pub contact: String,
    pub remote_addr: String,
    pub online: bool,
    pub manufacturer: String,
    pub model: String,
    pub firmware: String,
    pub registered_at: String,
    pub channel_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<ChannelDto>>,
}

impl From<GbDevice> for DeviceDto {
    fn from(d: GbDevice) -> Self {
        Self {
            channel_count: d.channel as usize,
            device_id: d.device_id,
            contact: d.contact,
            remote_addr: d.remote_addr,
            online: d.online,
            manufacturer: d.item.manufacturer.clone(),
            model: d.item.model.clone(),
            firmware: d.firmware,
            registered_at: d.registered_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            channels: None,
        }
    }
}

/// 通道 DTO
#[derive(Serialize)]
pub struct ChannelDto {
    pub channel_id: String,
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub status: String,
    pub address: String,
    pub parent_id: String,
    pub ip_address: String,
    pub port: u16,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub civil_code: String,
}

impl From<&GbDevice> for ChannelDto {
    fn from(c: &GbDevice) -> Self {
        Self {
            channel_id: c.item.device_id.to_string(),
            name: c.item.name.clone(),
            manufacturer: c.item.manufacturer.clone(),
            model: c.item.model.clone(),
            status: c.item.status.as_str().to_string(),
            address: c.item.address.clone(),
            parent_id: c.item.parent_id.clone(),
            ip_address: c.item.ip_address.clone().unwrap_or_default(),
            port: c.item.port.unwrap_or(0),
            longitude: c.item.longitude,
            latitude: c.item.latitude,
            civil_code: c.item.civil_code.clone(),
        }
    }
}

/// 会话 DTO
#[derive(Serialize)]
pub struct SessionDto {
    pub call_id: String,
    pub device_id: String,
    pub channel_id: String,
    pub rtp_port: u16,
    pub ssrc: String,
    pub stream_id: String,
    pub is_realtime: bool,
}

impl From<SessionInfo> for SessionDto {
    fn from(s: SessionInfo) -> Self {
        Self {
            call_id: s.call_id,
            device_id: s.device_id,
            channel_id: s.channel_id,
            rtp_port: s.rtp_port,
            ssrc: s.ssrc,
            stream_id: s.stream_id,
            is_realtime: s.is_realtime,
        }
    }
}
