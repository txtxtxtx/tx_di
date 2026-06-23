use crate::config::AppConfig;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::{error, info};
use tx_di_core::{async_method, App, CancellationToken, CompInit, RIE, tx_comp};

/// MQTT 客户端组件
///
/// 负责连接 MQTT broker 并发布消息。
/// 在 `async_init_impl` 阶段建立连接，在应用生命周期内保持连接。
#[tx_comp(init)]
pub struct MqttClient {
    /// 应用配置
    config: Arc<AppConfig>,
    /// MQTT 异步客户端（延迟初始化）
    /// 使用 OnceLock 实现一次性初始化，线程安全且无锁开销
    #[tx_cst(OnceLock::new())]
    client: OnceLock<AsyncClient>,
}

impl MqttClient {
    /// 发布消息到指定主题
    ///
    /// # Arguments
    /// * `topic` - MQTT 主题
    /// * `payload` - 消息内容
    pub async fn publish(&self, topic: &str, payload: &[u8]) -> RIE<()> {
        if let Some(client) = self.client.get() {
            client
                .publish(topic, QoS::AtLeastOnce, false, payload)
                .await
                .map_err(|e| anyhow::anyhow!("MQTT publish error: {}", e))?;
            info!("消息已发布到主题: {}", topic);
        } else {
            error!("MQTT客户端未初始化");
        }
        Ok(())
    }
}

impl CompInit for MqttClient {
    fn init_sort() -> i32 {
        10000 // 默认顺序
    }

    async_method!(fn async_init_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
        let config = ctx.inject::<AppConfig>();

        let mut mqttoptions = MqttOptions::new(
            &config.mqtt_client_id,
            &config.mqtt_broker,
            config.mqtt_port,
        );
        mqttoptions.set_keep_alive(Duration::from_secs(60));

        // 设置认证（如果提供）
        if let (Some(username), Some(password)) = (&config.mqtt_username, &config.mqtt_password) {
            if !username.is_empty() {
                mqttoptions.set_credentials(username, password);
            }
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 100);

        // 启动事件循环处理（支持优雅退出）
        let token_clone = token.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // 处理 MQTT 事件
                    result = eventloop.poll() => {
                        match result {
                            Ok(_notification) => {
                                // 正常通知，忽略
                            }
                            Err(e) => {
                                error!("MQTT错误: {}", e);
                                // 重连延迟
                                tokio::time::sleep(Duration::from_secs(5)).await;
                            }
                        }
                    }
                    // 收到取消信号，退出事件循环
                    _ = token_clone.cancelled() => {
                        info!("MQTT事件循环收到退出信号，正在关闭...");
                        break;
                    }
                }
            }
        });

        // 存储客户端到组件实例（一次性初始化）
        let this = ctx.inject::<MqttClient>();
        this.client.set(client)
            .map_err(|_| anyhow::anyhow!("MQTT客户端已经初始化"))?;

        info!(
            "MQTT客户端已连接: {}:{}",
            config.mqtt_broker, config.mqtt_port
        );

        Ok(())
    });
}
