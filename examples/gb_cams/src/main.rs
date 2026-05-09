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

    // 注册 REST API 路由（必须在 build 前）
    WebPlugin::add_router(api::router());

    let mut ctx = BuildContext::new(Some(config_path));
    let cam_config = ctx.inject::<GbCamsConfig>();


    // 启动 DI 框架（加载日志/配置/Web 服务等所有组件）
    let app = ctx.build()?;
    // 初始化全局设备管理器（单例，所有 API handler 可访问）
    DeviceManager::init(cam_config,app.shutdown_token.clone());

    let app = app.ins_run()
        .await?;

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

    app.waiting_exit().await;
    Ok(())
}
