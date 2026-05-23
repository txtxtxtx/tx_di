//! # GB28181 管理后台
//!
//! 启动 GB28181 服务端 + 管理 HTTP API + Vue3 前端静态服务 + 数据库持久化 + 用户认证。
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
mod models;

use tx_di_core::BuildContext;
use tx_di_gb28181::Gb28181Server;
use tx_di_axum::WebPlugin;
use tx_di_toasty::ToastyPlugin;
use tx_di_sa_token::SaTokenPlugin;
use tracing::info;

#[allow(unused_imports)]
use tx_di_log;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 在 build 之前注册事件监听器（用于 SSE 推送）
    Gb28181Server::on_event(|event| async move {
        api::sse::broadcast_event(event);
        Ok(())
    });

    // 2. 【关键】在 BuildContext::new() 之后、build() 之前注册数据库模型
    //    可以多次调用 register_models()，模型会合并（重复 ModelId 自动覆盖）
    ToastyPlugin::register_models(toasty::models!(
        models::User,
        models::GbDeviceRecord,
        models::GbSessionRecord,
        models::GbAlarmRecord,
        models::GbAuditLog,
        models::GbDeviceGroup,
        models::GbDeviceGroupMember,
        models::GbRegisterAudit,
    ));
    // 如果其他插件也有模型要注册，可以在这里继续追加：
    // ToastyPlugin::register_models(toasty::models!(OtherModel));

    // 3. 启动 DI 框架（加载配置 → 初始化所有组件 → async_init 自动读取全局模型）
    let app = BuildContext::new(Some("examples/gb28181_admin/config/config.toml"))
        .build()?
        .ins_run()
        .await?;

    // 4. 获取已初始化的 Db（由 ToastyPlugin::async_init 自动构建）
    let toasty_plugin = app.inject::<ToastyPlugin>();
    let db = toasty_plugin.db().clone();

    // 5. 获取 sa_token 插件实例，提取 SaTokenState
    let sa_plugin = app.inject::<SaTokenPlugin>();
    let sa_state = sa_plugin.state().clone();

    // 6. 注册带 State 的 API 路由
    WebPlugin::add_router(api::router(db, sa_state));

    app.waiting_exit().await;
    Ok(())
}
