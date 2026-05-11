//! API 响应 DTO

use serde::Serialize;
use crate::device::virtual_device::{VirtualDevice, VirtualChannel};

/// 统计概要
#[derive(Serialize)]
pub struct StatsDto {
    pub total: usize,
    pub online: usize,
    pub channels: usize,
}

/// 设备 DTO
#[derive(Serialize)]
pub struct DeviceDto {
    pub device_id: String,
    pub name: String,
    pub sip_port: u16,
    pub status: String,
    pub channel_count: usize,
    pub keepalive_count: u64,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<ChannelDto>>,
}

impl From<VirtualDevice> for DeviceDto {
    fn from(dev: VirtualDevice) -> Self {
        Self {
            device_id: dev.device_id,
            name: dev.name,
            sip_port: dev.sip_port,
            status: dev.status.to_string(),
            channel_count: dev.channels.len(),
            keepalive_count: dev.keepalive_count,
            error: dev.error,
            channels: None,
        }
    }
}

/// 通道 DTO
#[derive(Serialize)]
pub struct ChannelDto {
    pub channel_id: String,
    pub name: String,
    pub status: String,
}

impl From<&VirtualChannel> for ChannelDto {
    fn from(ch: &VirtualChannel) -> Self {
        Self {
            channel_id: ch.channel_id.clone(),
            name: ch.name.clone(),
            status: ch.status.as_str().to_string(),
        }
    }
}
