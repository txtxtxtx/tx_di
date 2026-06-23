use crate::config::Config;
use crate::error::{AppError, AppResult};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;
use tracing::{error, info};

/// MQTT客户端
pub struct MqttClient {
    client: AsyncClient,
}

impl MqttClient {
    /// 创建新的MQTT客户端
    pub async fn new(config: &Config) -> AppResult<Self> {
        let mut mqttoptions = MqttOptions::new(
            &config.mqtt_client_id,
            &config.mqtt_broker,
            config.mqtt_port,
        );

        // 设置连接超时
        mqttoptions.set_keep_alive(Duration::from_secs(60));

        // 设置认证（如果提供）
        if let (Some(username), Some(password)) = (&config.mqtt_username, &config.mqtt_password) {
            mqttoptions.set_credentials(username, password);
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 100);

        // 启动事件循环处理
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(notification) => {
                        info!("MQTT通知: {:?}", notification);
                    }
                    Err(e) => {
                        error!("MQTT错误: {}", e);
                        // 重连延迟
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        info!(
            "MQTT客户端已连接: {}:{}",
            config.mqtt_broker, config.mqtt_port
        );

        Ok(MqttClient { client })
    }

    /// 发布消息
    pub async fn publish(&self, topic: &str, payload: &[u8]) -> AppResult<()> {
        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(AppError::Mqtt)?;

        info!("消息已发布到主题: {}", topic);
        Ok(())
    }

    /// 订阅主题
    pub async fn subscribe(&self, topic: &str) -> AppResult<()> {
        self.client
            .subscribe(topic, QoS::AtLeastOnce)
            .await
            .map_err(AppError::Mqtt)?;

        info!("已订阅主题: {}", topic);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_client_creation() {
        // 这个测试需要实际的MQTT broker
        // let config = Config::from_env().unwrap();
        // let client = MqttClient::new(&config).await;
        // assert!(client.is_ok());
    }
}