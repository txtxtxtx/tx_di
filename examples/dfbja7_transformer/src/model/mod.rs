pub mod nano4sp;
pub mod gqb200a7u;

use serde::Serialize;
use crate::config::SensorConfig;

/// 设备数据模型
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "device_model")]
pub enum DeviceModel {
    #[serde(rename = "Nano4SP")]
    Nano4SP(nano4sp::Nano4SPModel),

    #[serde(rename = "GQB200A7U")]
    GQB200A7U(gqb200a7u::GQB200A7UModel),
}

/// MQTT 发送的设备数据载荷
#[derive(Debug, Clone, Serialize)]
pub struct DevicePayload {
    /// 序列号（Unix 毫秒时间戳）
    pub seq: i64,
    /// 时间戳（Unix 秒）
    pub timestamp: i64,
    /// 传感器参数列表
    pub params: Vec<Param>,
    /// 设备类型
    pub device_model: String,
    /// 设备编号
    pub device_code: String,
    /// 信号强度
    pub rssi: String,
    /// GPS数据
    pub gps: GpsData,
}

/// 传感器参数
#[derive(Debug, Clone, Serialize)]
pub struct Param {
    /// 传感器名称
    pub name: String,
    /// 单位
    pub unit: String,
    /// 最小值
    pub min: f64,
    /// 最大值
    pub max: f64,
    /// 当前值
    pub value: f64,
}

/// GPS数据
#[derive(Debug, Clone, Serialize)]
pub struct GpsData {
    pub longitude: String,
    pub latitude: String,
}

/// 将传感器值字符串转换为 f64
pub fn parse_sensor_value(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(-1.0)
}

/// 根据传感器配置和值列表生成 Param 列表
pub fn build_params(sensor_values: &[String], sensor_configs: &[SensorConfig]) -> Vec<Param> {
    sensor_configs
        .iter()
        .zip(sensor_values.iter())
        .map(|(config, value)| Param {
            name: config.name.clone(),
            unit: config.unit.clone(),
            min: config.min,
            max: config.max,
            value: parse_sensor_value(value),
        })
        .collect()
}