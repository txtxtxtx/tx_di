//! GB_CAMS 全局配置

use serde::Deserialize;
use tx_di_core::{tx_comp, BuildContext, CompInit, InnerContext, RIE};

/// GB28181 多设备模拟器全局配置
///
/// 对应 TOML 中的 `[gb_cams_config]` 段。
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
pub struct GbCamsConfig {
    /// 上级平台 ID（20 位）
    pub platform_id: String,

    /// 上级平台 IP
    pub platform_ip: String,

    /// 上级平台 SIP 端口
    #[serde(default = "default_platform_port")]
    pub platform_port: u16,

    /// SIP 认证域
    #[serde(default = "default_realm")]
    pub realm: String,

    /// 注册密码（所有设备共用）
    #[serde(default = "default_password")]
    pub password: String,

    /// 心跳间隔（秒）
    #[serde(default = "default_heartbeat")]
    pub heartbeat_secs: u64,

    /// 注册有效期（秒）
    #[serde(default = "default_ttl")]
    pub register_ttl: u32,

    /// SIP 基础端口（每个设备递增）
    #[serde(default = "default_sip_base_port")]
    pub sip_base_port: u16,

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
            platform_id: default_platform_id(),
            platform_ip: default_platform_ip(),
            platform_port: default_platform_port(),
            realm: default_realm(),
            password: default_password(),
            heartbeat_secs: default_heartbeat(),
            register_ttl: default_ttl(),
            sip_base_port: default_sip_base_port(),
            video_width: default_width(),
            video_height: default_height(),
            video_fps: default_fps(),
            background_image: default_bg_image(),
        }
    }
}

impl GbCamsConfig {
    /// 上级平台 SIP URI
    pub fn platform_uri(&self) -> String {
        format!("sip:{}@{}:{}", self.platform_id, self.platform_ip, self.platform_port)
    }
}

impl CompInit for GbCamsConfig {
    fn inner_init(&mut self, _ctx: &InnerContext) -> RIE<()> {
        Ok(())
    }
    fn init_sort() -> i32 {
        10002
    }
}

fn default_platform_id() -> String { "34020000002000000001".to_string() }
fn default_platform_ip() -> String { "127.0.0.1".to_string() }
fn default_platform_port() -> u16 { 5060 }
fn default_realm() -> String { "3402000000".to_string() }
fn default_password() -> String { "12345678".to_string() }
fn default_heartbeat() -> u64 { 60 }
fn default_ttl() -> u32 { 3600 }
fn default_sip_base_port() -> u16 { 6100 }
fn default_width() -> u32 { 1280 }
fn default_height() -> u32 { 720 }
fn default_fps() -> u32 { 25 }
fn default_bg_image() -> String { "examples/gb_cams/public/qj.jpg".to_string() }
