use serde::{Deserialize, Serialize};
use crate::config::SensorConfig;

/// Nano4SP设备数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nano4SPModel {
    /// 设备类型
    pub device_model: String,

    /// 设备编号
    pub device_code: String,

    /// 信号强度
    pub rssi: String,

    /// 第一路传感器
    pub sensor1: String,

    /// 第二路传感器
    pub sensor2: String,

    /// 第三路传感器
    pub sensor3: String,

    /// 第四路传感器
    pub sensor4: String,

    /// 经度
    pub lng: String,

    /// 纬度
    pub lat: String,

    /// 报警数据
    pub alarm: Vec<u8>,

    /// 报警描述
    pub level: Vec<String>,

    /// 特殊报警数据
    pub alarm_sp: Vec<String>,

    /// 电量信息
    pub soc: String,
}

impl Nano4SPModel {
    /// 转换为JSON字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 从JSON字符串解析
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 转换为 MQTT 发送的设备数据载荷
    pub fn to_payload(&self, sensor_configs: &[SensorConfig]) -> crate::model::DevicePayload {
        let sensor_values = vec![
            self.sensor1.clone(),
            self.sensor2.clone(),
            self.sensor3.clone(),
            self.sensor4.clone(),
        ];

        let now = chrono::Utc::now();

        crate::model::DevicePayload {
            seq: now.timestamp_millis(),
            timestamp: now.timestamp(),
            params: crate::model::build_params(&sensor_values, sensor_configs),
            device_model: self.device_model.clone(),
            device_code: self.device_code.clone(),
            rssi: self.rssi.clone(),
            gps: crate::model::GpsData {
                longitude: self.lng.clone(),
                latitude: self.lat.clone(),
            },
        }
    }
}

impl std::fmt::Display for Nano4SPModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Nano4SPModel {{ device_model: {}, device_code: {}, rssi: {}, sensors: [{}, {}, {}, {}], gps: ({}, {}), alarm: {:?}, level: {:?}, alarm_sp: {:?}, soc: {} }}",
            self.device_model,
            self.device_code,
            self.rssi,
            self.sensor1,
            self.sensor2,
            self.sensor3,
            self.sensor4,
            self.lng,
            self.lat,
            self.alarm,
            self.level,
            self.alarm_sp,
            self.soc
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let model = Nano4SPModel {
            device_model: "Nano4SP".to_string(),
            device_code: "12345678".to_string(),
            rssi: "-75dBm".to_string(),
            sensor1: "100".to_string(),
            sensor2: "200".to_string(),
            sensor3: "300".to_string(),
            sensor4: "4.5".to_string(),
            lng: "116.397128".to_string(),
            lat: "39.916527".to_string(),
            alarm: vec![1, 0, 2, 0],
            level: vec!["通道1一级报警".to_string(), "通道3二级报警".to_string()],
            alarm_sp: vec!["SOS报警".to_string()],
            soc: "85".to_string(),
        };

        let json = model.to_json().unwrap();
        let deserialized = Nano4SPModel::from_json(&json).unwrap();

        assert_eq!(model.device_model, deserialized.device_model);
        assert_eq!(model.device_code, deserialized.device_code);
        assert_eq!(model.sensor1, deserialized.sensor1);
    }
}