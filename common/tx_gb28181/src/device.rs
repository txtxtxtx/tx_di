//! GB28181 设备与通道数据类型 todo 统一设备和通道数据结构，提供兼容2016的数据结构以及转换方法
//!
//! 定义 GB28181-2022 标准中的核心域模型：
//! - [`ChannelInfo`]：设备/通道信息（目录结构 §8.1）
//! - [`ChannelStatus`]：通道在线状态
//! - [`DeviceInfo`]：单个设备的注册信息（SIP 通信层 + 运行时状态）
//!
//! 这些类型被 `tx_di_gb28181`（服务端插件）、`tx_di_gb28181_client`（设备客户端插件）
//! 以及 `gb28181_admin`、`gb_cams` 等示例程序共同使用。

use chrono::{DateTime, Utc};
use std::str::FromStr;
use tokio::time::{Duration, Instant};
use serde::{Deserialize, Deserializer, Serialize};

/// 通道在线状态
#[derive(Debug, Clone, PartialEq,Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[serde(untagged)]
pub enum ChannelStatus {
    On,
    Off,
    Unknown(String),
}
impl<'de> Deserialize<'de> for ChannelStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // 使用 FromStr 实现，支持大小写不敏感
        s.parse::<ChannelStatus>()
            .map_err(serde::de::Error::custom)
    }
}
impl FromStr for ChannelStatus {
    /// 永远不会发生的错误
    type Err = std::convert::Infallible;

    /// 从字符串解析通道状态
    /// 永远不会发生错误，可直接 unwrap
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "ON" => ChannelStatus::On,
            "OFF" => ChannelStatus::Off,
            other => ChannelStatus::Unknown(other.to_string()),
        })
    }
}

impl ChannelStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ChannelStatus::On => "ON",
            ChannelStatus::Off => "OFF",
            ChannelStatus::Unknown(s) => s.as_str(),
        }
    }
}

/// 设备通道信息（GB28181-2022 §8.1 目录结构）
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    /// 通道 ID（20 位）
    pub channel_id: String,
    /// 通道名称
    pub name: String,
    /// 厂商
    pub manufacturer: String,
    /// 设备型号
    pub model: String,
    /// 通道状态
    pub status: ChannelStatus,
    /// 地址描述
    pub address: String,
    /// 父设备 ID
    pub parent_id: String,
    /// 是否父节点（1=父节点/设备，0=叶节点/通道）
    pub parental: u8,
    /// 注册方式（1=主动注册，2=被动注册）
    pub register_way: u8,
    /// 保密属性（0=不涉密）
    pub secrecy: u8,
    /// 设备 IP
    pub ip_address: String,
    /// 设备端口
    pub port: u16,
    /// 经度
    pub longitude: Option<f64>,
    /// 纬度
    pub latitude: Option<f64>,
    /// 行政区划代码
    pub civil_code: String,
}

/// 单个设备的注册信息（含运行时状态）
///
/// 存储设备通过 SIP REGISTER 注册后的完整信息，包括：
/// - **协议层数据**：device_id / contact / channels / manufacturer 等
/// - **运行时状态**：注册时间、最后心跳时间、在线标记
///
/// `DeviceRegistry` 内部存储此类型，外部通过 `get()` / `online_devices()` 获取。
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 设备 ID（20 位）
    pub device_id: String,

    /// 设备的 SIP 联系地址（Contact URI），后续请求的直接目的地地址
    /// - 指定后续通信的地址
    /// - 实现 NAT（网络地址转换）穿透
    /// - 支持移动性和分机
    ///
    /// case
    /// - Contact: <sip:34020000001320000001@192.168.1.100:5060>
    /// - Contact: <sip:34020000001320000001@192.168.1.100:5060>;expires=3600
    pub contact: String,

    /// 注册时间
    pub registered_at: DateTime<Utc>,

    /// 最后一次心跳时间
    pub last_heartbeat: Instant,

    /// 注册有效期（秒）
    pub expires: u32,

    /// 设备的 IP 地址（来自 Via 头）
    pub remote_addr: String,

    /// 设备通道列表（目录查询后填充）
    pub channels: Vec<ChannelInfo>,

    /// 是否在线（由心跳超时检测更新）
    pub online: bool,

    /// 制造商（DeviceInfo 查询后填充）
    pub manufacturer: String,

    /// 型号（DeviceInfo 查询后填充）
    pub model: String,

    /// 固件版本
    pub firmware: String,
}

impl DeviceInfo {
    /// 创建新的设备信息
    pub fn new(device_id: String, contact: String, expires: u32, remote_addr: String) -> Self {
        Self {
            device_id,
            contact,
            registered_at: Utc::now(),
            last_heartbeat: Instant::now(),
            expires,
            remote_addr,
            channels: Vec::new(),
            online: true,
            manufacturer: String::new(),
            model: String::new(),
            firmware: String::new(),
        }
    }

    /// 是否已超时（未在规定时间内收到心跳）
    pub fn is_timeout(&self, timeout_secs: u64) -> bool {
        self.last_heartbeat.elapsed() > Duration::from_secs(timeout_secs)
    }

    /// 刷新心跳时间戳
    pub fn refresh_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 单元测试
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_status_from_str() {
        assert_eq!(
            "ON".parse::<ChannelStatus>().unwrap(),
            ChannelStatus::On
        );
        assert_eq!(
            "off".parse::<ChannelStatus>().unwrap(),
            ChannelStatus::Off
        );
        assert_eq!(
            "Maintenance"
                .parse::<ChannelStatus>()
                .unwrap(),
            ChannelStatus::Unknown("MAINTENANCE".to_string())
        );
    }

    #[test]
    fn channel_status_as_str() {
        assert_eq!(ChannelStatus::On.as_str(), "ON");
        assert_eq!(ChannelStatus::Off.as_str(), "OFF");
        assert_eq!(
            ChannelStatus::Unknown("X".into()).as_str(),
            "X"
        );
    }

    #[test]
    fn device_info_new() {
        let dev = DeviceInfo::new(
            "34020000001320000001".into(),
            "<sip:34020000001320000001@192.168.1.100:5060>".into(),
            3600,
            "192.168.1.100:5060".into(),
        );
        assert_eq!(dev.device_id, "34020000001320000001");
        assert!(dev.online);
        assert!(!dev.is_timeout(60)); // 刚创建，不应超时
    }
}
