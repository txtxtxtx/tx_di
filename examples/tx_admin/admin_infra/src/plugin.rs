//! 基础设施层插件 - 模型注册与数据初始化
//!
//! 职责：
//! 1. `InfraPlugin` — 注册所有 toasty 模型（在 DB 连接之前）
//! 2. `DbInitPlugin` — 检测首次启动，执行种子数据初始化（在 DB 连接之后）

use std::sync::Arc;
use tracing::{debug, info};
use tx_di_core::{App, CancellationToken, CompInit, RIE, async_method, tx_comp};
use tx_di_toasty::{ToastyConfig, ToastyDb, ToastyPlugin};

/// 模型注册插件
///
/// 在 `ToastyPlugin` 连接数据库之前执行，将模型注册到 `ModelSet`。
#[tx_comp(init)]
pub struct InfraPlugin;

impl CompInit for InfraPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let toasty_plugin = ctx.inject::<ToastyPlugin>();
            toasty_plugin.register_models(crate::register_models());
            info!("infra: toasty 模型已注册");
            Ok(())
        }
    );

    fn init_sort() -> i32 {
        // 必须在 ToastyPlugin（MAX-50）之前，确保模型在 DB 连接前注册
        i32::MAX - 200
    }
}

/// 数据库初始化插件
///
/// 在 `ToastyPlugin` 连接数据库之后执行，检测空数据库并初始化种子数据。
/// 仅在 `auto_schema = true` 时执行种子初始化。
#[tx_comp(init)]
pub struct DbInitPlugin;

impl CompInit for DbInitPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            // auto_schema = false 时跳过种子初始化（用户自行管理表结构）
            let toasty_config = ctx.inject::<ToastyConfig>();
            if !toasty_config.auto_schema {
                debug!("infra: auto_schema=false，跳过种子数据初始化");
                return Ok(());
            }

            let toasty_plugin = ctx.inject::<ToastyPlugin>();
            let db = toasty_plugin.db();
            crate::seed::seed_data(db).await?;
            info!("infra: 种子数据初始化完成");
            Ok(())
        }
    );

    fn init_sort() -> i32 {
        // 在 ToastyPlugin（MAX-50）之后，确保 DB 已连接
        i32::MAX - 25
    }
}
