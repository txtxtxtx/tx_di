//! # GB28181 设备模拟器入口
//!
//! 演示如何使用 `tx_di_sip` 插件实现国标 GB28181 设备侧能力：
//!
//! 1. **注册**  — 向上级平台注册，自动处理 401 摘要认证
//! 2. **心跳**  — 定时发送 `MESSAGE Keepalive` 维持在线状态
//! 3. **点播**  — 响应平台下发的 `INVITE`，回复 SDP 应答开始推流
//! 4. **注销**  — 进程退出前发送 `Expires: 0` 的 REGISTER 注销

mod gb;

use gb::{Gb28181Config, Gb28181Manager};
use tx_di_core::BuildContext;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 注册 SIP 消息处理器（必须在 build 之前完成）
    gb::register_handlers();

    // 2. 启动 DI 框架（加载配置、初始化 SIP 传输层）
    let mut ctx = BuildContext::new(Some("configs/gb28181.toml"));
    ctx.build().await?;

    info!("✅ GB28181 设备模拟器启动完成");

    // 3. 获取平台配置并启动 GB28181 业务逻辑
    let gb_cfg = ctx.inject::<Gb28181Config>();
    let manager = Gb28181Manager::new(gb_cfg);

    // 4. 执行注册 → 保持心跳 → 等待点播（主循环，Ctrl+C 退出）
    manager.run().await?;

    // 5. 程序退出前注销
    manager.unregister().await?;
    tx_di_sip::SipPlugin::shutdown();

    Ok(())
}
