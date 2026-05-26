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
//! | `GbDevice` → 2022 `ItemType` | `gb_device.to_item_type()` |

use crate::enums::{DeviceIDType, ItemType, StatusType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::time::{Duration, Instant};
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
// GbDevice — 统一设备/子设备描述结构体
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 国标设备/子设备统一描述结构体（GB/T 28181-2022 完整目录属性 + 运行时状态）
#[derive(Debug, Clone)]
pub struct GbDevice {
    // ==================== 协议层（嵌入 ItemType）====================
    /// GB28181-2022 目录项协议字段
    pub item: ItemType,

    // ==================== 运行时节点属性 ====================
    /// 节点类型（2022 版新增，区分设备/子设备/区域/系统/分组/虚拟组织）
    pub device_type: GbDeviceType,
    pub device_id: String,
    /// 固件版本（Firmware）
    pub firmware: String,

    /// 子设备数
    pub channel: u32,

    // ==================== SIP 协议层 ====================
    /// SIP 联系地址（Contact URI）
    pub contact: String,

    /// 远端地址
    pub remote_addr: String,

    // ==================== 运行时状态 ====================
    /// 注册时间
    pub registered_at: DateTime<Utc>,

    /// 最后一次心跳时间
    pub last_heartbeat: Instant,

    /// 注册有效期（秒），来自 SIP Expires 头
    pub expires: u32,

    /// 是否在线（心跳超时检测后的内部标记）
    pub online: bool,
}

impl Default for GbDevice {
    fn default() -> Self {
        Self {
            item: ItemType::default(),
            device_type: GbDeviceType::Device,
            device_id: String::new(),
            firmware: String::new(),
            channel: 0,
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
        self.item.parental == 0
    }

    /// 判断是否为父节点（容器/设备）
    pub fn is_parent(&self) -> bool {
        self.item.parental == 1
    }

    /// 是否在线
    pub fn is_online(&self) -> bool {
        self.online
    }

    /// 是否为指定父级的子节点
    pub fn is_son(&self, id: &str) -> bool {
        self.item.parent_id == id
    }
    /// 是否已超时（超过指定秒数未收到心跳）
    pub fn is_timeout(&self, timeout_secs: u64) -> bool {
        self.last_heartbeat.elapsed() > Duration::from_secs(timeout_secs)
    }

    // ==================== 运行时状态维护 ====================

    /// 刷新心跳时间戳
    pub fn refresh_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    /// 设置为在线
    pub fn set_online(&mut self) {
        self.online = true;
        self.refresh_heartbeat();
    }

    /// 设置为离线
    pub fn set_offline(&mut self) {
        self.online = false;
    }

    // ==================== 工厂方法 — 2022 版节点类型 ====================

    /// 创建设备节点（2022 版父设备）
    pub fn new_device(device_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            item: ItemType {
                device_id: DeviceIDType::Len20(device_id.into()),
                name: name.into(),
                parental: 1,
                ..ItemType::default()
            },
            device_type: GbDeviceType::Device,
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
            item: ItemType {
                device_id: DeviceIDType::Len20(device_id.into()),
                name: name.into(),
                parent_id: parent_id.into(),
                parental: 0,
                ..ItemType::default()
            },
            device_type: GbDeviceType::SubDevice,
            ..Default::default()
        }
    }

    /// 创建区域节点
    pub fn new_area(device_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            item: ItemType {
                device_id: DeviceIDType::Len20(device_id.into()),
                name: name.into(),
                parental: 1,
                ..ItemType::default()
            },
            device_type: GbDeviceType::Area,
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
            item: ItemType {
                device_id: DeviceIDType::Len20(device_id.into()),
                name: name.into(),
                parent_id: parent_id.into(),
                parental: 1,
                ..ItemType::default()
            },
            device_type: GbDeviceType::BusinessGroup,
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
            item: ItemType {
                device_id: DeviceIDType::Len20(device_id.into()),
                name: name.into(),
                parent_id: parent_id.into(),
                parental: 1,
                ..ItemType::default()
            },
            device_type: GbDeviceType::VirtualOrg,
            ..Default::default()
        }
    }

    /// 创建系统节点
    pub fn new_system(device_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            item: ItemType {
                device_id: DeviceIDType::Len20(device_id.into()),
                name: name.into(),
                parental: 1,
                ..ItemType::default()
            },
            device_type: GbDeviceType::System,
            ..Default::default()
        }
    }
    
    // ==================== 2022 版 CatalogItem 转换 ====================

    /// 从 2022 版 [`ItemType`] 构建
    pub fn from_item_type(item: &ItemType) -> Self {
        let device_type = if item.parental == 0 {
            GbDeviceType::SubDevice
        } else {
            GbDeviceType::Device
        };

        Self {
            item: item.clone(),
            device_type,
            device_id: item.device_id.to_string(),
            firmware: String::new(),
            channel: 0,
            contact: String::new(),
            remote_addr: String::new(),
            registered_at: Utc::now(),
            last_heartbeat: Instant::now(),
            expires: 3600,
            online: item.status == StatusType::ON,
        }
    }

    /// 转为 2022 版 [`ItemType`]
    pub fn to_item_type(&self) -> ItemType {
        self.item.clone()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 单元测试
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    // ── StatusType ──────────────────────────────────────────────────────────────

    #[test]
    fn status_type_from_str() {
        assert_eq!(StatusType::try_from("ON".to_string()).unwrap(), StatusType::ON);
        assert_eq!(StatusType::try_from("OFF".to_string()).unwrap(), StatusType::OFF);
        assert!(StatusType::try_from("Unknown".to_string()).is_err());
    }

    #[test]
    fn status_type_as_str() {
        assert_eq!(StatusType::ON.as_str(), "ON");
        assert_eq!(StatusType::OFF.as_str(), "OFF");
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
        assert_eq!(
            "Device".parse::<GbDeviceType>().unwrap(),
            GbDeviceType::Device
        );
        assert_eq!(
            "SubDevice".parse::<GbDeviceType>().unwrap(),
            GbDeviceType::SubDevice
        );
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
        assert_eq!(dev.item.device_id, DeviceIDType::Len2("34020000001320000001".into()));
        assert_eq!(dev.item.name, "TestDevice");
        assert_eq!(dev.item.parental, 1);
        assert!(dev.is_parent());
    }

    #[test]
    fn gb_device_new_sub_device() {
        let sub =
            GbDevice::new_sub_device("34020000001320000001", "Camera-01", "34020000001320000000");
        assert_eq!(sub.device_type, GbDeviceType::SubDevice);
        assert_eq!(sub.item.parent_id, "34020000001320000000");
        assert_eq!(sub.item.parental, 0);
        assert!(sub.is_leaf());
    }

    #[test]
    fn gb_device_new_area() {
        let area = GbDevice::new_area("330100", "杭州");
        assert_eq!(area.device_type, GbDeviceType::Area);
        assert_eq!(area.item.device_id, DeviceIDType::Len2("330100".into()));
        assert!(area.is_parent());
    }
    
    // ── 2022 ItemType <-> GbDevice 往返 ────────────────────────────────────

    #[test]
    fn catalog_item_to_gb_device_and_back() {
        let original = ItemType {
            device_id: DeviceIDType::Len20("34020000001320000001".into()),
            name: "Camera-01".into(),
            manufacturer: "Hikvision".into(),
            model: "DS-2CD2143G2-I".into(),
            status: StatusType::ON,
            address: "杭州市".into(),
            parent_id: "34020000001320000000".into(),
            parental: 0,
            register_way: 1,
            secrecy: 0,
            ip_address: Some("192.168.1.200".into()),
            port: Some(5060),
            longitude: Some(120.15),
            latitude: Some(30.28),
            block: Some("警区1".into()),
            civil_code: "330100".into(),
            password: None,
            security_level_code: None,
            business_group_id: None,
            info: None,
        };

        // ItemType (parental=0) -> GbDevice
        let gb = GbDevice::from_item_type(&original);
        assert_eq!(gb.item.device_id, DeviceIDType::Len20("34020000001320000001".into()));
        assert_eq!(gb.device_type, GbDeviceType::SubDevice);
        assert_eq!(gb.item.parent_id, "34020000001320000000");
        assert_eq!(gb.item.parental, 0);
        assert_eq!(gb.item.status, StatusType::ON);
        assert!(gb.online);

        // GbDevice -> ItemType
        let back = gb.to_item_type();
        assert_eq!(back.device_id, DeviceIDType::Len20("34020000001320000001".into()));
        assert_eq!(back.name, "Camera-01");
        assert_eq!(back.manufacturer, "Hikvision");
        assert_eq!(back.status, StatusType::ON);
        assert_eq!(back.parental, 0);
        assert_eq!(back.civil_code, "330100");
    }

    #[test]
    fn catalog_item_parental_device() {
        let item = ItemType {
            device_id: DeviceIDType::Len20("34020000001320000000".into()),
            name: "NVR".into(),
            manufacturer: "Hikvision".into(),
            model: "DS-7804N-K1".into(),
            status: StatusType::ON,
            address: String::new(),
            parent_id: String::new(),
            parental: 1,
            register_way: 1,
            secrecy: 0,
            ip_address: Some("192.168.1.1".into()),
            port: Some(5060),
            longitude: None,
            latitude: None,
            block: Some(String::new()),
            civil_code: "330100".into(),
            password: None,
            security_level_code: None,
            business_group_id: None,
            info: None,
        };

        let gb = GbDevice::from_item_type(&item);
        assert_eq!(gb.device_type, GbDeviceType::Device);
        assert_eq!(gb.item.parental, 1);
    }
}
