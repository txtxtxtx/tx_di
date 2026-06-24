use crate::config::AppConfig;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tracing::{error, info};
use tx_di_core::{async_method, App, CancellationToken, CompInit, RIE, tx_comp};

/// MQTT 客户端组件
///
/// 负责连接 MQTT broker 并发布消息。
/// 在 `async_init_impl` 阶段创建客户端，在 `async_run_impl` 阶段运行事件循环。
#[tx_comp(init)]
pub struct MqttClient {
    /// 应用配置
    config: Arc<AppConfig>,
    /// MQTT 异步客户端（延迟初始化）
    #[tx_cst(OnceLock::new())]
    client: OnceLock<AsyncClient>,
    /// MQTT 事件循环（从 init 传递到 run）
    #[tx_cst(Mutex::new(None))]
    eventloop: Mutex<Option<EventLoop>>,
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
            Err("MQTT客户端未初始化")?
        }
        Ok(())
    }
}

impl CompInit for MqttClient {
    fn init_sort() -> i32 {
        10000 // 默认顺序
    }

    async_method!(fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
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

        let (client, eventloop) = AsyncClient::new(mqttoptions, 100);

        // 存储客户端和事件循环到组件实例
        let this = ctx.inject::<MqttClient>();
        this.client.set(client)
            .map_err(|_| anyhow::anyhow!("MQTT客户端已经初始化"))?;
        *this.eventloop.lock().unwrap() = Some(eventloop);

        info!(
            "MQTT客户端已创建，正在连接: {}:{}",
            config.mqtt_broker, config.mqtt_port
        );

        Ok(())
    });

    async_method!(fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
        let this = ctx.inject::<MqttClient>();

        // 取出事件循环（从 init 阶段传递过来）
        let mut eventloop = this.eventloop.lock().unwrap().take()
            .ok_or_else(|| anyhow::anyhow!("MQTT事件循环未初始化"))?;

        info!("MQTT事件循环启动");

        loop {
            tokio::select! {
                // 处理 MQTT 事件
                result = eventloop.poll() => {
                    match result {
                        Ok(notification) => {
                            match notification {
                                Event::Incoming(Packet::PubAck(ack)) => {
                                    info!("MQTT消息已确认送达broker, pkid: {}", ack.pkid);
                                }
                                _ => {
                                    // 其他通知，忽略
                                }
                            }
                        }
                        Err(e) => {
                            error!("MQTT错误: {}", e);
                            // 重连延迟
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                }
                // 收到取消信号，退出事件循环
                _ = token.cancelled() => {
                    info!("MQTT事件循环收到退出信号，正在关闭...");
                    break;
                }
            }
        }

        info!("MQTT事件循环已停止");
        Ok(())
    });
}
