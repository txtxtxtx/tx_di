//! GB28181 设备与通道数据类型
//!
//! 定义 GB28181-2022 / GB28181-2016 标准中的核心域模型，并提供版本间双向转换能力。
//!
//! ## 核心类型
//! - [`GbDevice`]：统一设备/子设备描述结构体，支持 2022 全部节点类型 + SIP 运行时状态
//! - [`GbDeviceType`]：节点类型枚举（设备 / 子设备 / 区域 / 系统 / 业务分组 / 虚拟组织）
//! - [`ChannelInfo`]：2016 版通道信息（保持向后兼容）
//! - [`ChannelStatus`]：通道在线状态
//! - [`DeviceInfo`]：2016 版设备注册信息（SIP 通信层 + 运行时状态，保持向后兼容）
//!
//! ## 版本兼容
//! | 转换方向 | 方法 |
//! |---------|------|
//! | 2016 `DeviceInfo` → `GbDevice` | `GbDevice::from_device_info()` |
//! | `GbDevice` → 2016 `DeviceInfo` | `gb_device.to_device_info()` |
//! | 2016 `ChannelInfo` → `GbDevice` | `GbDevice::from_channel_info()` |
//! | `GbDevice` → 2016 `ChannelInfo` | `gb_device.to_channel_info()` |
//! | 2022 `CatalogItem` → `GbDevice` | `GbDevice::from_catalog_item()` |
//! | `GbDevice` → 2022 `CatalogItem` | `gb_device.to_catalog_item()` |

use crate::xml::CatalogItem;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;
use tokio::time::{Duration, Instant};


// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ChannelStatus
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 通道在线状态
#[derive(Debug, Clone, PartialEq, Serialize)]
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GbDeviceType — 2022 节点类型枚举
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// GB28181-2022 节点类型
///
/// 2022 版统一了设备和通道的概念，形成树形目录结构，
/// 每个节点有一个类型来标识其在层级中的角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GbDeviceType {
    /// 设备（NVR/DVR/IPC 等父节点设备）
    Device,
    /// 子设备（原 2016 版的"通道"，叶节点，可点播视频）
    SubDevice,
    /// 区域（Area）
    Area,
    /// 系统（System）
    System,
    /// 业务分组（BusinessGroup）
    BusinessGroup,
    /// 虚拟组织（VirtualOrg）
    VirtualOrg,
}

impl GbDeviceType {
    /// 转为 GB28181 标准中的字符串标识
    pub fn as_str(&self) -> &'static str {
        match self {
            GbDeviceType::Device => "Device",
            GbDeviceType::SubDevice => "SubDevice",
            GbDeviceType::Area => "Area",
            GbDeviceType::System => "System",
            GbDeviceType::BusinessGroup => "BusinessGroup",
            GbDeviceType::VirtualOrg => "VirtualOrg",
        }
    }
}

impl FromStr for GbDeviceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Device" => Ok(GbDeviceType::Device),
            "SubDevice" => Ok(GbDeviceType::SubDevice),
            "Area" => Ok(GbDeviceType::Area),
            "System" => Ok(GbDeviceType::System),
            "BusinessGroup" => Ok(GbDeviceType::BusinessGroup),
            "VirtualOrg" => Ok(GbDeviceType::VirtualOrg),
            other => Err(format!("未知的 GbDeviceType: {other}")),
        }
    }
}

impl std::fmt::Display for GbDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ChannelInfo — 2016 版通道信息（保持向后兼容）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 设备通道信息（GB28181-2016 目录结构，保持向后兼容）
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DeviceInfo — 2016 版设备注册信息（保持向后兼容）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
// GbDevice — 统一设备/子设备描述结构体
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 国标设备/子设备统一描述结构体（GB/T 28181-2022 完整目录属性 + 运行时状态）
///
/// 2022 版统一了设备和通道的概念，用一棵目录树描述所有节点。
/// 本结构体既能表达 2022 版的所有节点类型，也能与 2016 版的
/// [`DeviceInfo`] + [`ChannelInfo`] 互相转换。
///
/// # 节点类型
/// - `device_type == GbDeviceType::Device && parental == 1`：2022 版父设备（等同 2016 版 `DeviceInfo`）
/// - `device_type == GbDeviceType::SubDevice && parental == 0`：2022 版子设备（等同 2016 版 `ChannelInfo`）
/// - 其他 `device_type`：2022 版特有的节点类型（区域/系统/分组/虚拟组织）
///
/// # 转换
/// 通过 `from_device_info` / `to_device_info` 与 2016 版互相转换，
/// 通过 `from_catalog_item` / `to_catalog_item` 与 2022 版 Catalog 响应互相转换。
#[derive(Debug, Clone)]
pub struct GbDevice {
    // ==================== 统一标识与层级 ====================
    /// 节点类型（2022 版新增，区分设备/子设备/区域/系统/分组/虚拟组织）
    pub device_type: GbDeviceType,

    /// 设备/子设备 20 位编码（全局唯一）
    pub device_id: String,

    /// 设备/子设备名称
    pub name: String,

    /// 父节点设备编码（根节点为空字符串）
    pub parent_id: String,

    /// 是否为父节点：
    /// - 1：父节点（设备/NVR/平台/分组）
    /// - 0：叶节点（子设备/原通道）
    pub parental: u8,

    // ==================== 设备属性（目录响应字段） ====================
    /// 设备厂商（Manufacturer）
    pub manufacturer: String,

    /// 设备型号（Model）
    pub model: String,

    /// 固件版本（Firmware）
    pub firmware: String,

    /// 归属平台编码（Owner），20 位，表示设备归属的上级 SIP 服务域，可为空
    pub owner: String,

    /// 设备/子设备状态（Status）
    pub status: ChannelStatus,

    /// 经度（Longitude），单位：度，取值范围 -180.0 ~ 180.0
    pub longitude: Option<f64>,

    /// 纬度（Latitude），单位：度，取值范围 -90.0 ~ 90.0
    pub latitude: Option<f64>,

    /// 行政区划代码（CivilCode），6 位数字，如 330100
    pub civil_code: String,

    /// 安装地址（Address）
    pub address: String,

    /// 安全传输方式（SafetyWay）：
    /// - 0：不采用 TLS
    /// - 1：采用 TLS
    pub safety_way: u8,

    /// 注册方式（RegisterWay）：
    /// - 1：符合 IETF RFC 3261 标准的 SIP 注册
    /// - 2：基于数字摘要认证的注册
    /// - 3：双向认证注册
    pub register_way: u8,

    /// 保密属性（Secrecy）：
    /// - 0：不涉密
    /// - 1：涉密
    pub secrecy: u8,

    /// 是否移动设备（Mobile）：
    /// - 0：非移动设备
    /// - 1：移动设备
    pub mobile: u8,

    /// 业务分组编码（BusinessGroupID），可空
    pub business_group_id: Option<String>,

    /// 下载倍速范围（DownloadSpeed），如 "1/4-1/2-1-2-4"，可空
    pub download_speed: Option<String>,

    /// 空域编码能力（SVCSpaceDomainMode），如 "1,2,3"，可空
    pub svc_space_domain_mode: Option<String>,

    /// 时域编码能力（SVCTimeDomainMode），如 "1,2,3"，可空
    pub svc_time_domain_mode: Option<String>,

    /// SSIM 编码能力（SVCSSIMode），如 "1,2"，可空
    pub svc_ssim_mode: Option<String>,

    // ==================== 网络与 SIP 协议层 ====================
    /// 设备 IP 地址（优先来自 Contact 或 Via，也可目录查询填充）
    pub ip_address: String,

    /// 设备端口号
    pub port: u16,

    /// SIP 联系地址（Contact URI）
    /// 示例: sip:34020000001320000001@192.168.1.100:5060
    pub contact: String,

    /// 远端地址（通常为 Via 头或实际接收到的 IP:Port）
    pub remote_addr: String,

    // ==================== 运行时状态（由注册和心跳维护） ====================
    /// 注册时间
    pub registered_at: DateTime<Utc>,

    /// 最后一次心跳时间（基于本地单调时钟，用于超时检测）
    pub last_heartbeat: Instant,

    /// 注册有效期（秒），来自 SIP Expires 头
    pub expires: u32,

    /// 是否在线（心跳超时检测后的内部标记，不直接对应 Status）
    pub online: bool,
}

impl Default for GbDevice {
    fn default() -> Self {
        Self {
            device_type: GbDeviceType::Device,
            device_id: String::new(),
            name: String::new(),
            parent_id: String::new(),
            parental: 0,
            manufacturer: String::new(),
            model: String::new(),
            firmware: String::new(),
            owner: String::new(),
            status: ChannelStatus::On,
            longitude: None,
            latitude: None,
            civil_code: String::new(),
            address: String::new(),
            safety_way: 0,
            register_way: 1,
            secrecy: 0,
            mobile: 0,
            business_group_id: None,
            download_speed: None,
            svc_space_domain_mode: None,
            svc_time_domain_mode: None,
            svc_ssim_mode: None,
            ip_address: String::new(),
            port: 0,
            contact: String::new(),
            remote_addr: String::new(),
            registered_at: Utc::now(),
            last_heartbeat: Instant::now(),
            expires: 3600,
            online: true,
        }
    }
}

impl GbDevice {
    // ==================== 判断方法 ====================

    /// 判断是否为叶节点（可直接点播视频）
    pub fn is_leaf(&self) -> bool {
        self.parental == 0
    }

    /// 判断是否为父节点（容器/设备）
    pub fn is_parent(&self) -> bool {
        self.parental == 1
    }

    // ==================== 工厂方法 — 2022 版节点类型 ====================

    /// 创建设备节点（2022 版父设备）
    pub fn new_device(device_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            device_type: GbDeviceType::Device,
            device_id: device_id.into(),
            name: name.into(),
            parental: 1,
            ..Default::default()
        }
    }

    /// 创建子设备/通道节点（2022 版叶节点，兼容 2016 版 ChannelInfo）
    pub fn new_sub_device(
        device_id: impl Into<String>,
        name: impl Into<String>,
        parent_id: impl Into<String>,
    ) -> Self {
        Self {
            device_type: GbDeviceType::SubDevice,
            device_id: device_id.into(),
            name: name.into(),
            parent_id: parent_id.into(),
            parental: 0,
            ..Default::default()
        }
    }

    /// 创建区域节点
    pub fn new_area(device_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            device_type: GbDeviceType::Area,
            device_id: device_id.into(),
            name: name.into(),
            parental: 1,
            ..Default::default()
        }
    }

    /// 创建业务分组节点
    pub fn new_business_group(
        device_id: impl Into<String>,
        name: impl Into<String>,
        parent_id: impl Into<String>,
    ) -> Self {
        Self {
            device_type: GbDeviceType::BusinessGroup,
            device_id: device_id.into(),
            name: name.into(),
            parent_id: parent_id.into(),
            parental: 1,
            ..Default::default()
        }
    }

    /// 创建虚拟组织节点
    pub fn new_virtual_org(
        device_id: impl Into<String>,
        name: impl Into<String>,
        parent_id: impl Into<String>,
    ) -> Self {
        Self {
            device_type: GbDeviceType::VirtualOrg,
            device_id: device_id.into(),
            name: name.into(),
            parent_id: parent_id.into(),
            parental: 1,
            ..Default::default()
        }
    }

    /// 创建系统节点
    pub fn new_system(device_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            device_type: GbDeviceType::System,
            device_id: device_id.into(),
            name: name.into(),
            parental: 1,
            ..Default::default()
        }
    }

    // ==================== 2016 版转换 ====================

    /// 从 2016 版 [`DeviceInfo`] 构建
    ///
    /// 映射 SIP 运行时状态字段（contact / registered_at / last_heartbeat / expires / remote_addr / online），
    /// 以及设备属性字段（manufacturer / model / firmware）。
    /// 注意：`DeviceInfo.channels` 不在此映射中（channels 是子节点列表，需要单独转换）。
    pub fn from_device_info(info: &DeviceInfo) -> Self {
        Self {
            device_type: GbDeviceType::Device,
            device_id: info.device_id.clone(),
            name: info.device_id.clone(), // 2016 DeviceInfo 无 name 字段，用 device_id 兜底
            parent_id: String::new(),
            parental: 1, // DeviceInfo 始终是父节点
            manufacturer: info.manufacturer.clone(),
            model: info.model.clone(),
            firmware: info.firmware.clone(),
            owner: String::new(),
            status: if info.online {
                ChannelStatus::On
            } else {
                ChannelStatus::Off
            },
            longitude: None,
            latitude: None,
            civil_code: String::new(),
            address: String::new(),
            safety_way: 0,
            register_way: 1,
            secrecy: 0,
            mobile: 0,
            business_group_id: None,
            download_speed: None,
            svc_space_domain_mode: None,
            svc_time_domain_mode: None,
            svc_ssim_mode: None,
            ip_address: info.remote_addr.clone(),
            port: 0,
            contact: info.contact.clone(),
            remote_addr: info.remote_addr.clone(),
            registered_at: info.registered_at,
            last_heartbeat: info.last_heartbeat,
            expires: info.expires,
            online: info.online,
        }
    }

    /// 转为 2016 版 [`DeviceInfo`]
    ///
    /// `channels` 返回空 Vec，需要外部根据子节点列表单独填充。
    pub fn to_device_info(&self) -> DeviceInfo {
        DeviceInfo {
            device_id: self.device_id.clone(),
            contact: self.contact.clone(),
            registered_at: self.registered_at,
            last_heartbeat: self.last_heartbeat,
            expires: self.expires,
            remote_addr: self.remote_addr.clone(),
            channels: Vec::new(), // 需要外部填充
            online: self.online,
            manufacturer: self.manufacturer.clone(),
            model: self.model.clone(),
            firmware: self.firmware.clone(),
        }
    }

    /// 从 2016 版 [`ChannelInfo`] 构建
    pub fn from_channel_info(info: &ChannelInfo) -> Self {
        Self {
            device_type: GbDeviceType::SubDevice,
            device_id: info.channel_id.clone(),
            name: info.name.clone(),
            parent_id: info.parent_id.clone(),
            parental: info.parental,
            manufacturer: info.manufacturer.clone(),
            model: info.model.clone(),
            firmware: String::new(),
            owner: String::new(),
            status: info.status.clone(),
            longitude: info.longitude,
            latitude: info.latitude,
            civil_code: info.civil_code.clone(),
            address: info.address.clone(),
            safety_way: 0,
            register_way: info.register_way,
            secrecy: info.secrecy,
            mobile: 0,
            business_group_id: None,
            download_speed: None,
            svc_space_domain_mode: None,
            svc_time_domain_mode: None,
            svc_ssim_mode: None,
            ip_address: info.ip_address.clone(),
            port: info.port,
            contact: String::new(),
            remote_addr: String::new(),
            registered_at: Utc::now(),
            last_heartbeat: Instant::now(),
            expires: 3600,
            online: info.status == ChannelStatus::On,
        }
    }

    /// 转为 2016 版 [`ChannelInfo`]
    pub fn to_channel_info(&self) -> ChannelInfo {
        ChannelInfo {
            channel_id: self.device_id.clone(),
            name: self.name.clone(),
            manufacturer: self.manufacturer.clone(),
            model: self.model.clone(),
            status: self.status.clone(),
            address: self.address.clone(),
            parent_id: self.parent_id.clone(),
            parental: self.parental,
            register_way: self.register_way,
            secrecy: self.secrecy,
            ip_address: self.ip_address.clone(),
            port: self.port,
            longitude: self.longitude,
            latitude: self.latitude,
            civil_code: self.civil_code.clone(),
        }
    }

    // ==================== 2022 版 CatalogItem 转换 ====================

    /// 从 2022 版 [`CatalogItem`] 构建
    ///
    /// `CatalogItem` 是 GB28181-2022 目录查询响应解析后的条目结构（定义在 [`crate::xml`]）。
    pub fn from_catalog_item(item: &CatalogItem) -> Self {
        // 根据 parental 字段推断节点类型
        let device_type = if item.parental == 0 {
            GbDeviceType::SubDevice
        } else {
            GbDeviceType::Device
        };

        Self {
            device_type,
            device_id: item.device_id.clone(),
            name: item.name.clone(),
            parent_id: item.parent_id.clone(),
            parental: item.parental,
            manufacturer: item.manufacturer.clone(),
            model: item.model.clone(),
            firmware: String::new(),
            owner: String::new(),
            status: ChannelStatus::from_str(&item.status).unwrap(),
            longitude: item.longitude,
            latitude: item.latitude,
            civil_code: item.civil_code.clone(),
            address: item.address.clone(),
            safety_way: 0,
            register_way: item.register_way,
            secrecy: item.secrecy,
            mobile: 0,
            business_group_id: None,
            download_speed: None,
            svc_space_domain_mode: None,
            svc_time_domain_mode: None,
            svc_ssim_mode: None,
            ip_address: item.ip_address.clone(),
            port: item.port,
            contact: String::new(),
            remote_addr: String::new(),
            registered_at: Utc::now(),
            last_heartbeat: Instant::now(),
            expires: 3600,
            online: item.status.to_uppercase() == "ON",
        }
    }

    /// 转为 2022 版 [`CatalogItem`]
    pub fn to_catalog_item(&self) -> CatalogItem {
        CatalogItem {
            device_id: self.device_id.clone(),
            name: self.name.clone(),
            manufacturer: self.manufacturer.clone(),
            model: self.model.clone(),
            status: self.status.as_str().to_string(),
            address: self.address.clone(),
            parent_id: self.parent_id.clone(),
            parental: self.parental,
            register_way: self.register_way,
            secrecy: self.secrecy,
            ip_address: self.ip_address.clone(),
            port: self.port,
            longitude: self.longitude,
            latitude: self.latitude,
            block: String::new(),
            civil_code: self.civil_code.clone(),
            channel_num: 0,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 单元测试
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    // ── ChannelStatus ──────────────────────────────────────────────────────────

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
            "Maintenance".parse::<ChannelStatus>().unwrap(),
            ChannelStatus::Unknown("MAINTENANCE".to_string())
        );
    }

    #[test]
    fn channel_status_as_str() {
        assert_eq!(ChannelStatus::On.as_str(), "ON");
        assert_eq!(ChannelStatus::Off.as_str(), "OFF");
        assert_eq!(ChannelStatus::Unknown("X".into()).as_str(), "X");
    }

    // ── DeviceInfo ─────────────────────────────────────────────────────────────

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

    // ── GbDeviceType ───────────────────────────────────────────────────────────

    #[test]
    fn gb_device_type_as_str() {
        assert_eq!(GbDeviceType::Device.as_str(), "Device");
        assert_eq!(GbDeviceType::SubDevice.as_str(), "SubDevice");
        assert_eq!(GbDeviceType::Area.as_str(), "Area");
        assert_eq!(GbDeviceType::System.as_str(), "System");
        assert_eq!(GbDeviceType::BusinessGroup.as_str(), "BusinessGroup");
        assert_eq!(GbDeviceType::VirtualOrg.as_str(), "VirtualOrg");
    }

    #[test]
    fn gb_device_type_from_str() {
        assert_eq!("Device".parse::<GbDeviceType>().unwrap(), GbDeviceType::Device);
        assert_eq!("SubDevice".parse::<GbDeviceType>().unwrap(), GbDeviceType::SubDevice);
        assert_eq!("Area".parse::<GbDeviceType>().unwrap(), GbDeviceType::Area);
        assert!("Unknown".parse::<GbDeviceType>().is_err());
    }

    #[test]
    fn gb_device_type_display() {
        assert_eq!(format!("{}", GbDeviceType::Device), "Device");
    }

    // ── GbDevice 工厂方法 ──────────────────────────────────────────────────────

    #[test]
    fn gb_device_new_device() {
        let dev = GbDevice::new_device("34020000001320000001", "TestDevice");
        assert_eq!(dev.device_type, GbDeviceType::Device);
        assert_eq!(dev.device_id, "34020000001320000001");
        assert_eq!(dev.name, "TestDevice");
        assert_eq!(dev.parental, 1);
        assert!(dev.is_parent());
    }

    #[test]
    fn gb_device_new_sub_device() {
        let sub = GbDevice::new_sub_device("34020000001320000001", "Camera-01", "34020000001320000000");
        assert_eq!(sub.device_type, GbDeviceType::SubDevice);
        assert_eq!(sub.parent_id, "34020000001320000000");
        assert_eq!(sub.parental, 0);
        assert!(sub.is_leaf());
    }

    #[test]
    fn gb_device_new_area() {
        let area = GbDevice::new_area("330100", "杭州");
        assert_eq!(area.device_type, GbDeviceType::Area);
        assert_eq!(area.device_id, "330100");
        assert!(area.is_parent());
    }

    // ── 2016 DeviceInfo <-> GbDevice 往返 ──────────────────────────────────────

    #[test]
    fn device_info_to_gb_device_and_back() {
        let original = DeviceInfo::new(
            "34020000001320000001".into(),
            "<sip:34020000001320000001@192.168.1.100:5060>".into(),
            3600,
            "192.168.1.100:5060".into(),
        );
        let mut original = original;
        original.manufacturer = "Hikvision".into();
        original.model = "DS-2CD2143G2-I".into();
        original.firmware = "V5.7.10".into();

        // DeviceInfo -> GbDevice
        let gb = GbDevice::from_device_info(&original);
        assert_eq!(gb.device_id, "34020000001320000001");
        assert_eq!(gb.device_type, GbDeviceType::Device);
        assert_eq!(gb.parental, 1);
        assert_eq!(gb.manufacturer, "Hikvision");
        assert_eq!(gb.model, "DS-2CD2143G2-I");
        assert_eq!(gb.firmware, "V5.7.10");
        assert_eq!(gb.contact, "<sip:34020000001320000001@192.168.1.100:5060>");
        assert_eq!(gb.remote_addr, "192.168.1.100:5060");
        assert!(gb.online);

        // GbDevice -> DeviceInfo
        let back = gb.to_device_info();
        assert_eq!(back.device_id, "34020000001320000001");
        assert_eq!(back.manufacturer, "Hikvision");
        assert_eq!(back.model, "DS-2CD2143G2-I");
        assert_eq!(back.firmware, "V5.7.10");
        assert_eq!(back.contact, "<sip:34020000001320000001@192.168.1.100:5060>");
        assert_eq!(back.remote_addr, "192.168.1.100:5060");
        assert!(back.online);
        assert_eq!(back.expires, 3600);
        assert!(back.channels.is_empty()); // channels 需要外部填充
    }

    // ── 2016 ChannelInfo <-> GbDevice 往返 ─────────────────────────────────────

    #[test]
    fn channel_info_to_gb_device_and_back() {
        let original = ChannelInfo {
            channel_id: "34020000001320000001".into(),
            name: "Camera-01".into(),
            manufacturer: "Dahua".into(),
            model: "IPC-HDW5442T".into(),
            status: ChannelStatus::On,
            address: "杭州市".into(),
            parent_id: "34020000001320000000".into(),
            parental: 0,
            register_way: 1,
            secrecy: 0,
            ip_address: "192.168.1.200".into(),
            port: 5060,
            longitude: Some(120.15),
            latitude: Some(30.28),
            civil_code: "330100".into(),
        };

        // ChannelInfo -> GbDevice
        let gb = GbDevice::from_channel_info(&original);
        assert_eq!(gb.device_id, "34020000001320000001");
        assert_eq!(gb.device_type, GbDeviceType::SubDevice);
        assert_eq!(gb.parent_id, "34020000001320000000");
        assert_eq!(gb.parental, 0);
        assert!(gb.is_leaf());
        assert_eq!(gb.manufacturer, "Dahua");
        assert_eq!(gb.longitude, Some(120.15));
        assert_eq!(gb.latitude, Some(30.28));
        assert_eq!(gb.civil_code, "330100");

        // GbDevice -> ChannelInfo
        let back = gb.to_channel_info();
        assert_eq!(back.channel_id, "34020000001320000001");
        assert_eq!(back.name, "Camera-01");
        assert_eq!(back.manufacturer, "Dahua");
        assert_eq!(back.model, "IPC-HDW5442T");
        assert_eq!(back.status, ChannelStatus::On);
        assert_eq!(back.parent_id, "34020000001320000000");
        assert_eq!(back.parental, 0);
        assert_eq!(back.ip_address, "192.168.1.200");
        assert_eq!(back.port, 5060);
        assert_eq!(back.longitude, Some(120.15));
        assert_eq!(back.latitude, Some(30.28));
        assert_eq!(back.civil_code, "330100");
    }

    // ── 2022 CatalogItem <-> GbDevice 往返 ────────────────────────────────────

    #[test]
    fn catalog_item_to_gb_device_and_back() {
        let original = CatalogItem {
            device_id: "34020000001320000001".into(),
            name: "Camera-01".into(),
            manufacturer: "Hikvision".into(),
            model: "DS-2CD2143G2-I".into(),
            status: "ON".into(),
            address: "杭州市".into(),
            parent_id: "34020000001320000000".into(),
            parental: 0,
            register_way: 1,
            secrecy: 0,
            ip_address: "192.168.1.200".into(),
            port: 5060,
            longitude: Some(120.15),
            latitude: Some(30.28),
            block: "警区1".into(),
            civil_code: "330100".into(),
            channel_num: 0,
        };

        // CatalogItem (parental=0) -> GbDevice
        let gb = GbDevice::from_catalog_item(&original);
        assert_eq!(gb.device_id, "34020000001320000001");
        assert_eq!(gb.device_type, GbDeviceType::SubDevice);
        assert_eq!(gb.parent_id, "34020000001320000000");
        assert_eq!(gb.parental, 0);
        assert_eq!(gb.status, ChannelStatus::On);
        assert!(gb.online);

        // GbDevice -> CatalogItem
        let back = gb.to_catalog_item();
        assert_eq!(back.device_id, "34020000001320000001");
        assert_eq!(back.name, "Camera-01");
        assert_eq!(back.manufacturer, "Hikvision");
        assert_eq!(back.status, "ON");
        assert_eq!(back.parental, 0);
        assert_eq!(back.civil_code, "330100");
    }

    #[test]
    fn catalog_item_parental_device() {
        let item = CatalogItem {
            device_id: "34020000001320000000".into(),
            name: "NVR".into(),
            manufacturer: "Hikvision".into(),
            model: "DS-7804N-K1".into(),
            status: "ON".into(),
            address: String::new(),
            parent_id: String::new(),
            parental: 1, // 父节点
            register_way: 1,
            secrecy: 0,
            ip_address: "192.168.1.1".into(),
            port: 5060,
            longitude: None,
            latitude: None,
            block: String::new(),
            civil_code: "330100".into(),
            channel_num: 4,
        };

        let gb = GbDevice::from_catalog_item(&item);
        assert_eq!(gb.device_type, GbDeviceType::Device);
        assert_eq!(gb.parental, 1);
    }
}
