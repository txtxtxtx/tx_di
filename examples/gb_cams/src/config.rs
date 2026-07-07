//! GB_CAMS 业务配置（视频 / 媒体参数）
//!
//! SIP 连接信息已交由 `tx_di_gb_dev`（TOML `[gb_dev]` 段）与
//! `tx_di_sip`（TOML `[sip_config]` 段）承载，本配置仅保留模拟器自身的
//! 媒体生成参数，避免重复配置。

use serde::Deserialize;
use tx_di_core::Component;

/// GB_CAMS 业务配置（TOML `[gb_cams_config]` 段）
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf)]
pub struct GbCamsConfig {
    /// 视频宽度
    #[serde(default = "default_width")]
    pub video_width: u32,

    /// 视频高度
    #[serde(default = "default_height")]
    pub video_height: u32,

    /// 视频帧率
    #[serde(default = "default_fps")]
    pub video_fps: u32,

    /// 背景图片路径
    #[serde(default = "default_bg_image")]
    pub background_image: String,
}

impl Default for GbCamsConfig {
    fn default() -> Self {
        Self {
            video_width: default_width(),
            video_height: default_height(),
            video_fps: default_fps(),
            background_image: default_bg_image(),
        }
    }
}

fn default_width() -> u32 { 1280 }
fn default_height() -> u32 { 720 }
fn default_fps() -> u32 { 25 }
fn default_bg_image() -> String { "examples/gb_cams/public/qj.jpg".to_string() }
