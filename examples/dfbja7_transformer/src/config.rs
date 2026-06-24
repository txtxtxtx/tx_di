use serde::Deserialize;
use std::collections::HashMap;
use tx_di_core::{tx_comp, CompInit};

/// 传感器配置
#[derive(Debug, Clone, Deserialize)]
pub struct SensorConfig {
    /// 传感器名称（如 O2、CO、H2S）
    pub name: String,
    /// 单位（如 %Vol、ppm）
    pub unit: String,
    /// 最小值
    pub min: f64,
    /// 最大值
    pub max: f64,
}

/// 设备型号配置
#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    /// 传感器配置列表
    pub sensors: Vec<SensorConfig>,
}

/// 应用配置
///
/// 从 TOML 配置文件的 `[app_config]` section 自动反序列化。
///
/// # 配置文件示例
///
/// ```toml
/// [app_config]
/// tcp_port = 10080
/// tcp_timeout_secs = 150
/// mqtt_broker = "localhost"
/// mqtt_port = 1883
/// mqtt_client_id = "dfbja7_transformer"
/// mqtt_username = ""
/// mqtt_password = ""
/// mqtt_topic_prefix = "/device/"
/// ```
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
pub struct AppConfig {
    /// TCP服务器端口
    #[serde(default = "default_tcp_port")]
    pub tcp_port: u16,

    /// TCP超时时间（秒）
    #[serde(default = "default_tcp_timeout")]
    pub tcp_timeout_secs: u64,

    /// MQTT broker地址
    #[serde(default = "default_mqtt_broker")]
    pub mqtt_broker: String,

    /// MQTT broker端口
    #[serde(default = "default_mqtt_port")]
    pub mqtt_port: u16,

    /// MQTT客户端ID
    #[serde(default = "default_mqtt_client_id")]
    pub mqtt_client_id: String,

    /// MQTT用户名（可选）
    #[serde(default)]
    pub mqtt_username: Option<String>,

    /// MQTT密码（可选）
    #[serde(default)]
    pub mqtt_password: Option<String>,

    /// MQTT主题前缀
    #[serde(default = "default_mqtt_topic_prefix")]
    pub mqtt_topic_prefix: String,

    /// 设备型号配置（按设备型号名称索引）
    #[serde(default)]
    pub model: HashMap<String, ModelConfig>,
}

fn default_tcp_port() -> u16 {
    10080
}
fn default_tcp_timeout() -> u64 {
    150
}
fn default_mqtt_broker() -> String {
    "localhost".to_string()
}
fn default_mqtt_port() -> u16 {
    1883
}
fn default_mqtt_client_id() -> String {
    "dfbja7_transformer".to_string()
}
fn default_mqtt_topic_prefix() -> String {
    "/device/".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tcp_port: default_tcp_port(),
            tcp_timeout_secs: default_tcp_timeout(),
            mqtt_broker: default_mqtt_broker(),
            mqtt_port: default_mqtt_port(),
            mqtt_client_id: default_mqtt_client_id(),
            mqtt_username: None,
            mqtt_password: None,
            mqtt_topic_prefix: default_mqtt_topic_prefix(),
            model: HashMap::new(),
        }
    }
}

impl CompInit for AppConfig {
    /// 在日志之后初始化
    fn init_sort() -> i32 {
        i32::MIN + 1
    }
}
