//! # GB28181 多设备模拟器
//!
//! 带 Vue3 Web 管理界面的 GB28181 摄像头集群模拟器：
//!
//! 1. **Web 管理界面** — 配置设备、批量生成、查看通道
//! 2. **自动注册** — 所有虚拟设备向上级平台注册
//! 3. **心跳保活** — 定时发送 Keepalive
//! 4. **目录响应** — 响应平台 Catalog 查询
//! 5. **点播响应** — 收到 INVITE 后生成媒体流（图片+时间叠加 → RTP）

mod config;
mod device;
mod media;
mod generator;
mod api;
mod dto;

use config::GbCamsConfig;
use device::DeviceManager;
use tx_di_axum::WebPlugin;
use tx_di_core::BuildContext;
use tracing::info;

use tx_di_log;
use tx_di_axum;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = "examples/gb_cams/config/gb_cams.toml";

    // 1. 预读取配置文件，初始化全局设备管理器
    // （WebPlugin::add_router 必须在 ctx.build() 之前调用）
    let toml_content = std::fs::read_to_string(config_path)
        .map_err(|e| anyhow::anyhow!("读取配置文件失败: {}", e))?;
    let full_config: toml::Value = toml::from_str(&toml_content)
        .map_err(|e| anyhow::anyhow!("解析配置文件失败: {}", e))?;

    // 提取 [gb_cams_config] 段
    let gb_cfg: GbCamsConfig = full_config
        .get("gb_cams_config")
        .and_then(|v| v.clone().try_into().ok())
        .unwrap_or_default();

    info!("📂 配置加载完成: platform={}, base_port={}", gb_cfg.platform_uri(), gb_cfg.sip_base_port);

    // 2. 初始化全局设备管理器（单例，所有 API handler 可访问）
    DeviceManager::init(std::sync::Arc::new(gb_cfg));

    // 3. 注册 REST API 路由（必须在 build 前）
    WebPlugin::add_router(api::router());

    // 4. 启动 DI 框架（加载日志/配置/Web 服务等所有组件）
    let mut ctx = BuildContext::new(Some(config_path));
    ctx.build_and_run().await?;

    info!("✅ GB28181 多设备模拟器启动完成");
    info!("📡 API: http://localhost:8889/api/gb_cams/");
    info!("🖥️  管理界面: http://localhost:8889/cam/");
    info!("");
    info!("📌 快速开始:");
    info!("   1. 批量生成设备: POST /api/gb_cams/devices/generate");
    info!("      body: {{\"count\": 10, \"channels_per_device\": 4, \"auto_register\": true}}");
    info!("   2. 查看设备列表: GET /api/gb_cams/devices");
    info!("   3. 查看统计: GET /api/gb_cams/stats");
    info!("");

    // 5. 等待 Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("正在关闭...");
    DeviceManager::instance().stop_all();

    Ok(())
}
