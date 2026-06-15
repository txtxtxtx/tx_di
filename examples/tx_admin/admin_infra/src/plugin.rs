//! 基础设施层插件 - 模型注册与数据初始化
//!
//! 职责：
//! 1. `InfraPlugin` — 注册所有 toasty 模型（在 DB 连接之前）
//! 2. `DbInitPlugin` — 检测首次启动，执行种子数据初始化（在 DB 连接之后）

use tx_di_core::{tx_comp, App, CancellationToken, CompInit, RIE, async_method};
use tx_di_toasty::{ToastyPlugin, ToastyDb};
use std::sync::Arc;
use tracing::{info, debug};

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
#[tx_comp(init)]
pub struct DbInitPlugin;

impl CompInit for DbInitPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let toasty_plugin = ctx.inject::<ToastyPlugin>();
            let db = toasty_plugin.db();

            if needs_init(db).await {
                info!("infra: 检测到空数据库，开始初始化种子数据...");
                crate::seed::seed_data(db).await?;
                info!("infra: 种子数据初始化完成");
            } else {
                debug!("infra: 数据库已有数据，跳过初始化");
            }

            Ok(())
        }
    );

    fn init_sort() -> i32 {
        // 在 ToastyPlugin（MAX-50）之后，确保 DB 已连接
        i32::MAX - 25
    }
}

/// 检测数据库是否需要初始化
///
/// 通过查询 sys_user 表是否有数据来判断
async fn needs_init(db: &ToastyDb) -> bool {
    use crate::user::model::SysUser;

    let mut db = db.clone();
    match SysUser::all().count().exec(&mut db).await {
        Ok(count) => count == 0,
        Err(_) => true,
    }
}
