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

    // 2. 注册 REST API 路由（在 DI build 之后设置，需要 db 和 sa_plugin）
    //    这里先占位，实际路由在 DI 完成后通过 WebPlugin::add_router 设置
    //    注意：router 需要 (Db, DiComp<SaTokenPlugin>) 作为 State

    // 3. 使用 toasty::models! 注册所有数据库模型
    //    模型注册必须在 Db::builder() 阶段完成
    let models = toasty::models!(
        models::User,
        models::GbDeviceRecord,
        models::GbSessionRecord,
        models::GbAlarmRecord,
        models::GbAuditLog,
        models::GbDeviceGroup,
        models::GbDeviceGroupMember,
        models::GbRegisterAudit,
    );

    // 4. 启动 DI 框架（加载配置 → 初始化所有组件）
    let app = BuildContext::new(Some("examples/gb28181_admin/config/config.toml"))
        .build()?
        .ins_run()
        .await?;

    // 5. 用注册的模型构建数据库（在 DI 完成后）
    let toasty_plugin = app.inject::<ToastyPlugin>();
    let config = toasty_plugin.config.clone();
    let db = if toasty_plugin.try_db().is_none() {
        let d = ToastyPlugin::build_db_with_models(models, &config).await?;
        let _ = toasty_plugin.db.set(d.clone());
        info!("数据库已初始化（带模型注册）");
        d
    } else {
        toasty_plugin.db.get().cloned().expect("db 已初始化")
    };

    // 6. 获取 sa_token 插件实例，提取 SaTokenState
    let sa_plugin = app.inject::<SaTokenPlugin>();
    let sa_state = sa_plugin.state().clone();

    // 7. 注册带 State 的 API 路由
    WebPlugin::add_router(api::router(db, sa_state));

    app.waiting_exit().await;
    Ok(())
}
