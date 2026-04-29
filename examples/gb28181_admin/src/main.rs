//! # GB28181 管理后台示例
//!
//! 启动 GB28181 服务端 + 管理 HTTP API + Vue3 前端静态服务。
//!
//! ## 启动步骤
//!
//! ```bash
//! # 1. 先构建前端（首次或代码变更时）
//! cd examples/gb28181_admin/ui && npm install && npm run build
//!
//! # 2. 启动后端
//! cargo run -p gb28181_admin
//! # 打开浏览器访问 http://localhost:8080/admin/
//! ```

mod api;
mod dto;

use tx_di_core::BuildContext;
use tx_di_gb28181::Gb28181Server;
use tx_di_axum::WebPlugin;
use tracing::info;
use tx_di_log;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 在 build 之前注册事件监听器（用于 SSE 推送）
    Gb28181Server::on_event(|event| async move {
        api::sse::broadcast_event(event);
        Ok(())
    });

    // 2. 注册 REST API 路由
    WebPlugin::add_router(api::router());

    // 3. 启动 DI 框架（加载配置 → 初始化所有组件）
    let mut ctx = BuildContext::new(Some("examples/gb28181_admin/config/config.toml"));
    ctx.build_and_run().await?;

    info!("✅ GB28181 管理后台启动完成");
    info!("📡 API:      http://localhost:8080/api/gb28181/");
    info!("🖥️  前端:     http://localhost:8080/admin/");

    // 4. 等待 Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("正在关闭...");

    Ok(())
}
