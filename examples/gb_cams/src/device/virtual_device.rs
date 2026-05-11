//! 虚拟设备 & 通道数据结构

use std::sync::atomic::AtomicBool;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// 通道状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelStatus {
    Online,
    Offline,
}

impl std::fmt::Display for ChannelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelStatus::Online => write!(f, "ON"),
            ChannelStatus::Offline => write!(f, "OFF"),
        }
    }
}

/// 虚拟通道
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualChannel {
    pub channel_id: String,
    pub name: String,
    pub status: ChannelStatus,
}

/// 设备注册状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    /// 未注册
    Idle,
    /// 注册中
    Registering,
    /// 已注册
    Registered,
    /// 注册失败
    Failed,
    /// 已注销
    Unregistered,
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceStatus::Idle => write!(f, "idle"),
            DeviceStatus::Registering => write!(f, "registering"),
            DeviceStatus::Registered => write!(f, "registered"),
            DeviceStatus::Failed => write!(f, "failed"),
            DeviceStatus::Unregistered => write!(f, "unregistered"),
        }
    }
}

/// 虚拟设备实例
#[derive(Debug,Clone, Serialize, Deserialize)]
pub struct VirtualDevice {
    /// 设备 ID（20 位）
    pub device_id: String,

    /// 设备名称
    pub name: String,

    /// 通道列表
    pub channels: Vec<VirtualChannel>,

    /// SIP 端口号
    pub sip_port: u16,

    /// 注册状态
    pub status: DeviceStatus,

    /// 上级平台 username（通常等于 device_id）
    pub username: String,

    /// 注册失败原因
    pub error: Option<String>,

    /// 上次心跳时间
    #[serde(skip)]
    pub last_keepalive: Option<Instant>,

    /// 心跳计数
    pub keepalive_count: u64,

    pub running: bool,
}

impl VirtualDevice {
    /// 创建新虚拟设备
    pub fn new(device_id: String, channels: Vec<VirtualChannel>, name: String, sip_port: u16) -> Self {
        Self {
            device_id: device_id.clone(),
            name,
            channels,
            sip_port,
            status: DeviceStatus::Idle,
            username: device_id,
            error: None,
            last_keepalive: None,
            keepalive_count: 0,
            running: false,
        }
    }

    /// 是否在线（已注册）
    pub fn is_online(&self) -> bool {
        self.status == DeviceStatus::Registered
    }

    /// 通道数量
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
}
