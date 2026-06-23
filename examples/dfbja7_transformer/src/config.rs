use std::env;

/// 应用配置
#[derive(Debug, Clone)]
pub struct Config {
    /// TCP服务器端口
    pub tcp_port: u16,
    /// MQTT broker地址
    pub mqtt_broker: String,
    /// MQTT broker端口
    pub mqtt_port: u16,
    /// MQTT客户端ID
    pub mqtt_client_id: String,
    /// MQTT用户名（可选）
    pub mqtt_username: Option<String>,
    /// MQTT密码（可选）
    pub mqtt_password: Option<String>,
    /// MQTT主题前缀
    pub mqtt_topic_prefix: String,
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> anyhow::Result<Self> {
        // 加载.env文件（如果存在）
        dotenvy::dotenv().ok();

        let tcp_port = env::var("TCP_PORT")
            .unwrap_or_else(|_| "10080".to_string())
            .parse::<u16>()?;

        let mqtt_broker = env::var("MQTT_BROKER")
            .unwrap_or_else(|_| "localhost".to_string());

        let mqtt_port = env::var("MQTT_PORT")
            .unwrap_or_else(|_| "1883".to_string())
            .parse::<u16>()?;

        let mqtt_client_id = env::var("MQTT_CLIENT_ID")
            .unwrap_or_else(|_| "dfbja7_transformer".to_string());

        let mqtt_username = env::var("MQTT_USERNAME").ok();
        let mqtt_password = env::var("MQTT_PASSWORD").ok();

        let mqtt_topic_prefix = env::var("MQTT_TOPIC")
            .unwrap_or_else(|_| "/device/".to_string());

        Ok(Config {
            tcp_port,
            mqtt_broker,
            mqtt_port,
            mqtt_client_id,
            mqtt_username,
            mqtt_password,
            mqtt_topic_prefix,
        })
    }
}