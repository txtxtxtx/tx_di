#![allow(dead_code)]

//! 纯 Rust 媒体流生成模块
//!
//! 流程：
//! 1. 背景图片 + 时间/通道叠加 → RGB 帧
//! 2. RGB → YUV420 转换
//! 3. YUV420 → H264 编码（openh264）
//! 4. H264 NALU → PES → PS 封装
//! 5. PS → RTP 封包 → UDP 发送

pub mod frame_gen;
pub mod ps_mux;
pub mod rtp_sender;

use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// 媒体流配置
#[derive(Debug, Clone)]
pub struct MediaStreamConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub target_ip: String,
    pub target_port: u16,
    pub ssrc: String,
    pub channel_id: String,
    pub background_path: String,
}

/// 媒体流实例
pub struct MediaStream {
    config: MediaStreamConfig,
    cancel: CancellationToken,
}

impl MediaStream {
    pub fn new(config: MediaStreamConfig, cancel: CancellationToken) -> Self {
        Self { config, cancel }
    }

    /// 启动媒体流发送（后台任务）
    pub fn start(self) {
        let config = self.config.clone();
        let cancel = self.cancel.clone();

        tokio::spawn(async move {
            if let Err(e) = run_media_stream(config, cancel).await {
                error!(error = %e, "媒体流异常退出");
            }
        });
    }
}

/// 媒体流主循环
async fn run_media_stream(
    config: MediaStreamConfig,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    info!(
        channel = %config.channel_id,
        target = %format!("{}:{}", config.target_ip, config.target_port),
        "🎬 媒体流启动"
    );

    // 加载背景图片
    let bg_image = image::open(&config.background_path)
        .map_err(|e| anyhow::anyhow!("加载背景图片失败 '{}': {}", config.background_path, e))?;
    let bg_rgba = bg_image
        .resize_exact(config.width, config.height, image::imageops::FilterType::Nearest)
        .to_rgba8();

    // 创建 RTP 发送器
    let mut rtp_sender = rtp_sender::RtpSender::new(
        &config.target_ip,
        config.target_port,
        &config.ssrc,
    )
    .await?;

    let frame_duration = std::time::Duration::from_millis(1000 / config.fps as u64);
    let mut frame_count: u64 = 0;
    let mut timestamp: u32 = 0;

    // H264 编码上下文
    // 使用简单方案：每帧独立 JPEG 编码后封装为 PS 流
    // 这样避免 openh264 的复杂编译依赖
    // GB28181 的 PS 流可以承载 MPEG-4 video (PT=96)
    let mut interval = tokio::time::interval(frame_duration);
    interval.tick().await; // 跳过首次

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!(channel = %config.channel_id, "媒体流收到停止信号");
                break;
            }
            _ = interval.tick() => {
                frame_count += 1;
                timestamp += 90000 / config.fps; // 90kHz 时钟

                // 生成当前帧
                let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                let frame = frame_gen::generate_frame(
                    &bg_rgba,
                    config.width,
                    config.height,
                    &now,
                    &config.channel_id,
                );

                // 将 RGBA 帧转为 JPEG bytes
                let jpeg_bytes = encode_frame_to_jpeg(&frame, config.width, config.height)?;

                // 封装为 PS 包
                let ps_data = ps_mux::wrap_as_ps(
                    &jpeg_bytes,
                    timestamp,
                    &config.ssrc,
                );

                // 发送 RTP
                if let Err(e) = rtp_sender.send_ps(&ps_data, timestamp, frame_count as u32).await {
                    warn!(error = %e, "RTP 发送失败");
                }
            }
        }
    }

    info!(channel = %config.channel_id, frames = frame_count, "🎬 媒体流停止");
    Ok(())
}

/// 将 RGBA 帧编码为 JPEG
fn encode_frame_to_jpeg(
    rgba_data: &[u8],
    width: u32,
    height: u32,
) -> anyhow::Result<Vec<u8>> {
    use image::ImageEncoder;
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 75);
    encoder.write_image(
        rgba_data,
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(buf.into_inner())
}
