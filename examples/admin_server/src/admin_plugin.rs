use std::sync::Arc;
use tracing::info;
use tx_di_axum::WebPlugin;
use tx_di_core::{App, CancellationToken, CompInit, RIE, async_method, tx_comp};

use crate::interfaces::api;

/// 管理后台路由注册组件
///
/// init_sort = i32::MAX - 100，早于 WebPlugin（i32::MAX），
/// 确保路由在 WebPlugin::merge_routers() 之前注册。
#[tx_comp(init)]
pub struct AdminPlugin;

impl CompInit for AdminPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            // 注册所有后台管理 API 路由
            WebPlugin::add_router(api::router(ctx));
            info!("admin_server 路由已注册");
            Ok(())
        }
    );

    fn init_sort() -> i32 {
        i32::MAX - 100
    }
}
