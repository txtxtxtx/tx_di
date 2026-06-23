use crate::config::AppConfig;
use crate::mqtt::MqttClient;
use crate::protocol::ProtocolParser;
use crate::util::convert;
use bytes::{Buf, BytesMut};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};
use tx_di_core::{async_method, App, CancellationToken, CompInit, RIE, tx_comp};

/// TCP 服务器组件
///
/// 负责监听 TCP 连接，接收设备数据，解析协议并转发到 MQTT。
/// 在 `async_run_impl` 阶段启动监听循环。
#[tx_comp(init)]
pub struct TcpServer {
    /// 应用配置
    config: Arc<AppConfig>,
    /// MQTT 客户端
    mqtt_client: Arc<MqttClient>,
    /// 协议解析器
    parser: Arc<ProtocolParser>,
}

impl TcpServer {
    /// 处理单个 TCP 连接
    async fn handle_connection(
        mut stream: TcpStream,
        mqtt_client: Arc<MqttClient>,
        config: Arc<AppConfig>,
    ) -> RIE<()> {
        let mut buffer = BytesMut::with_capacity(1024);
        let mut frame_buffer = BytesMut::with_capacity(4096);
        let read_timeout = Duration::from_secs(config.tcp_timeout_secs);

        loop {
            // 读取数据（带超时）
            let n = match timeout(read_timeout, stream.read_buf(&mut buffer)).await {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
                    warn!("读取数据错误: {}", e);
                    return Err(e.into());
                }
                Err(_) => {
                    warn!("读取超时，关闭连接");
                    return Ok(());
                }
            };

            if n == 0 {
                info!("连接关闭");
                return Ok(());
            }

            // 将数据添加到帧缓冲区
            frame_buffer.extend_from_slice(&buffer);
            buffer.clear();

            // 尝试解析帧
            while let Some(frame) = Self::extract_frame(&mut frame_buffer)? {
                // 处理帧
                if let Err(e) = Self::process_frame(&frame, &mqtt_client, &config).await {
                    error!("处理帧错误: {}", e);
                }
            }
        }
    }

    /// 从缓冲区提取完整的帧
    fn extract_frame(buffer: &mut BytesMut) -> RIE<Option<String>> {
        if buffer.len() < 2 {
            return Ok(None);
        }

        // 查找开始标志
        let start_pos = buffer.iter().position(|&b| b == 0x55 || b == 0xAA);
        let start_pos = match start_pos {
            Some(pos) => pos,
            None => {
                // 没有找到开始标志，清空缓冲区
                buffer.clear();
                return Ok(None);
            }
        };

        // 跳过开始标志之前的数据
        if start_pos > 0 {
            buffer.advance(start_pos);
        }

        if buffer.len() < 2 {
            return Ok(None);
        }

        // 获取长度字节
        let length_byte = buffer[1];
        let frame_length = Self::calculate_frame_length(length_byte)?;

        // 检查是否有足够的数据
        if buffer.len() < frame_length {
            return Ok(None);
        }

        // 提取帧数据
        let frame_data = buffer.split_to(frame_length);
        let frame_hex = convert::bytes_to_hex(&frame_data);

        Ok(Some(frame_hex))
    }

    /// 计算帧长度
    fn calculate_frame_length(length_byte: u8) -> RIE<usize> {
        let actual_length = if length_byte > 128 {
            ((length_byte - 128) as usize) / 2 + 128
        } else {
            length_byte as usize
        };

        // 加上开始标志(1字节)、长度字节(1字节)、RSSI(1字节)、CRC(2字节)
        Ok(actual_length + 5)
    }

    /// 处理单个帧
    async fn process_frame(
        frame: &str,
        mqtt_client: &MqttClient,
        config: &AppConfig,
    ) -> RIE<()> {
        info!("收到帧: {}", frame);

        // 解析消息
        let device_data = ProtocolParser::parse(frame)?;

        // 转换为设备信息
        let device_info = match device_data {
            crate::protocol::DeviceData::Nano4SP(model) => {
                info!("解析Nano4SP数据: {}", model);
                model.to_device_info()
            }
            crate::protocol::DeviceData::GQB200A7U(model) => {
                info!("解析GQB200A7U数据: {}", model);
                model.to_device_info()
            }
        };

        // 序列化为JSON
        let json = serde_json::to_string(&device_info)?;

        // 生成MQTT主题
        let topic = format!(
            "{}{}/{}",
            config.mqtt_topic_prefix, device_info.device_model, device_info.device_code
        );

        // 发送到MQTT
        mqtt_client.publish(&topic, json.as_bytes()).await?;

        info!("消息已发送到MQTT: {}", topic);

        Ok(())
    }
}

impl CompInit for TcpServer {
    /// 最后启动 TCP 服务器
    fn init_sort() -> i32 {
        i32::MAX
    }

    /// 在 async_run 阶段启动 TCP 监听循环
    async_method!(fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
        let config = ctx.inject::<AppConfig>();
        let mqtt_client = ctx.inject::<MqttClient>();

        let addr = format!("0.0.0.0:{}", config.tcp_port);
        let listener = TcpListener::bind(&addr).await?;
        info!("TCP服务器启动在: {}", addr);

        // 使用 select! 同时监听连接和取消信号
        loop {
            tokio::select! {
                // 接受新连接
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            info!("新的连接: {}", addr);
                            let mqtt_client = mqtt_client.clone();
                            let config = config.clone();

                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(stream, mqtt_client, config).await {
                                    error!("处理连接错误: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("接受连接错误: {}", e);
                        }
                    }
                }
                // 收到取消信号，优雅退出
                _ = token.cancelled() => {
                    info!("收到退出信号，TCP服务器正在关闭...");
                    break;
                }
            }
        }

        info!("TCP服务器已关闭");
        Ok(())
    });
}
