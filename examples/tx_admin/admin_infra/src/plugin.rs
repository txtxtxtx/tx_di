//! 基础设施层插件 - 模型注册与数据初始化
//!
//! 职责：
//! 1. 模型注册 — `init` 中注册所有 toasty 模型（在 DB 连接之前）
//! 2. 种子数据 — `app_async_init` 中检测首次启动，执行种子数据初始化

use std::sync::Arc;
use tracing::{debug, info};
use tx_di_core::{App, Component, DepsTuple, RIE, Store};
use tx_di_toasty::{ToastyConfig, ToastyPlugin};

/// 数据库初始化插件
///
/// `init`：注册所有 toasty 模型（在 ToastyPlugin 连接数据库之前）
/// `app_async_init`：检测空数据库并初始化种子数据（仅在 `auto_schema = true` 时）
#[derive(Component)]
#[component(init, app_async_init, init_sort = i32::MAX - 200)]
pub struct DbInitPlugin;

/// `#[component(init)]` 回调：注册所有 toasty 模型
fn init(_this: &mut DbInitPlugin, _store: &Store) -> RIE<()> {
    let toasty_plugin = tx_di_core::inject_from_store::<ToastyPlugin>(_store);
    toasty_plugin.register_models(crate::register_models());
    toasty_plugin.register_models(tx_di_job::register_models());
    info!("infra: toasty 模型已注册");
    Ok(())
}

/// `#[component(app_async_init)]` 回调：Schema 迁移 + 初始化种子数据
async fn app_async_init(_comp: Arc<DbInitPlugin>, app: Arc<App>) -> RIE<()> {
    let toasty_config = app.inject::<ToastyConfig>();
    let toasty_plugin = app.inject::<ToastyPlugin>();

    // 生产迁移模式：受控 push_schema + 版本审计（`migrate_on_start=true`）
    if !toasty_config.auto_schema {
        if toasty_config.migrate_on_start {
            toasty_plugin.migrate().await?;
            info!("infra: Schema 迁移完成 (migrate_on_start)");
        } else {
            debug!("infra: auto_schema=false 且 migrate_on_start=false，跳过 Schema 管理");
            return Ok(());
        }
    }

    // 种子数据（内部检测空库，非空自动跳过）
    let db = toasty_plugin.db();
    crate::seed::seed_data(db).await?;
    info!("infra: 种子数据初始化完成");
    Ok(())
}
