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

/// `#[component(app_async_init)]` 回调：初始化种子数据
async fn app_async_init(_comp: Arc<DbInitPlugin>, app: Arc<App>) -> RIE<()> {
    // auto_schema = false 时跳过种子初始化（用户自行管理表结构）
    let toasty_config = app.inject::<ToastyConfig>();
    if !toasty_config.auto_schema {
        debug!("infra: auto_schema=false，跳过种子数据初始化");
        return Ok(());
    }

    let toasty_plugin = app.inject::<ToastyPlugin>();
    let db = toasty_plugin.db();
    crate::seed::seed_data(db).await?;
    info!("infra: 种子数据初始化完成");
    Ok(())
}
