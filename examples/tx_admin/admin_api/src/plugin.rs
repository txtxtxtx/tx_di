use std::sync::Arc;
use tracing::info;
use tx_di_axum::WebPlugin;
use tx_di_core::{App, CancellationToken, CompInit, RIE, async_method, tx_comp};
use tx_di_toasty::ToastyPlugin;

use crate::interfaces::api;

#[tx_comp(init)]
pub struct AdminPlugin;

impl CompInit for AdminPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            // 注册 admin_infra 中所有 toasty 模型
            let toasty_plugin = ctx.inject::<ToastyPlugin>();
            toasty_plugin.register_models(admin_infra::register_models());
            info!("toasty 模型已注册");

            // 注册 HTTP 路由（通过 WebPlugin 全局注册表）
            WebPlugin::add_router(api::router());
            info!("admin HTTP 路由已注册");
            Ok(())
        }
    );
    fn init_sort() -> i32 {
        // 在 ToastyPlugin 之后初始化（确保数据库已连接）
        i32::MAX - 100
    }
}
