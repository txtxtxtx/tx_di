//! # GB28181 多设备模拟器
#![allow(dead_code)]

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = "examples/gb_cams/config/gb_cams.toml";

    WebPlugin::add_router(api::router().into());

    let ctx = BuildContext::new(Some(config_path));
    let cam_config = ctx.inject::<GbCamsConfig>();


    // 初始化全局设备管理器（业务单例，需在 build 之前，仅依赖业务配置）
    DeviceManager::init(cam_config);

    // 启动 DI 框架（加载日志/配置/Web/SIP/设备端组件等所有组件）
    // - tx_di_sip::SipPlugin 提供 SIP 端点
    // - tx_di_gb_dev::Gb28181Device 自动注册/心跳/响应（注入本 crate 的 GbCamsHandler）
    let app = ctx.build()?;

    let app = app.ins_run()
        .await?;

    app.waiting_exit().await;
    DeviceManager::instance().shutdown();
    Ok(())
}
