pub mod nano4sp;
pub mod gqb200a7u;

use serde::{Deserialize, Serialize};

/// 设备数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "device_model")]
pub enum DeviceModel {
    #[serde(rename = "Nano4SP")]
    Nano4SP(nano4sp::Nano4SPModel),

    #[serde(rename = "GQB200A7U")]
    GQB200A7U(gqb200a7u::GQB200A7UModel),
}

/// 通用设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// 设备类型
    pub device_model: String,
    /// 设备编号
    pub device_code: String,
    /// 信号强度
    pub rssi: String,
    /// 传感器数据
    pub sensors: Sensors,
    /// GPS数据
    pub gps: GpsData,
    /// 报警信息
    pub alarm: AlarmInfo,
    /// 电量信息
    pub soc: Option<String>,
    /// 时间戳
    pub timestamp: String,
}

/// 传感器数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sensors {
    pub sensor1: String,
    pub sensor2: String,
    pub sensor3: String,
    pub sensor4: String,
}

/// GPS数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsData {
    pub longitude: String,
    pub latitude: String,
}

/// 报警信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmInfo {
    pub levels: Vec<u8>,
    pub level_descriptions: Vec<String>,
    pub special: Vec<String>,
}

impl From<nano4sp::Nano4SPModel> for DeviceInfo {
    fn from(model: nano4sp::Nano4SPModel) -> Self {
        DeviceInfo {
            device_model: model.device_model,
            device_code: model.device_code,
            rssi: model.rssi,
            sensors: Sensors {
                sensor1: model.sensor1,
                sensor2: model.sensor2,
                sensor3: model.sensor3,
                sensor4: model.sensor4,
            },
            gps: GpsData {
                longitude: model.lng,
                latitude: model.lat,
            },
            alarm: AlarmInfo {
                levels: model.alarm,
                level_descriptions: model.level,
                special: model.alarm_sp,
            },
            soc: Some(model.soc),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl From<gqb200a7u::GQB200A7UModel> for DeviceInfo {
    fn from(model: gqb200a7u::GQB200A7UModel) -> Self {
        DeviceInfo {
            device_model: model.device_model,
            device_code: model.device_code,
            rssi: model.rssi,
            sensors: Sensors {
                sensor1: model.sensor1,
                sensor2: model.sensor2,
                sensor3: model.sensor3,
                sensor4: model.sensor4,
            },
            gps: GpsData {
                longitude: model.lng,
                latitude: model.lat,
            },
            alarm: AlarmInfo {
                levels: model.alarm,
                level_descriptions: model.level,
                special: model.alarm_sp,
            },
            soc: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}