pub mod tcp;

use crate::config::Config;
use crate::error::AppResult;
use crate::mqtt::MqttClient;
use std::sync::Arc;
use tokio::sync::Mutex;

/// TCP服务器
pub struct Server {
    config: Config,
    mqtt_client: Arc<Mutex<MqttClient>>,
}

impl Server {
    /// 创建新的服务器实例
    pub fn new(config: Config, mqtt_client: MqttClient) -> Self {
        Server {
            config,
            mqtt_client: Arc::new(Mutex::new(mqtt_client)),
        }
    }

    /// 启动服务器
    pub async fn run(&self) -> AppResult<()> {
        tcp::run_tcp_server(&self.config, self.mqtt_client.clone()).await
    }
}