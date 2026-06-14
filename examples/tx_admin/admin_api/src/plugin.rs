use std::sync::Arc;
use tracing::info;
use tx_di_axum::WebPlugin;
use tx_di_core::{App, CancellationToken, CompInit, RIE, async_method, tx_comp};

use crate::interfaces::api;

/// 确保 infra 层插件被编译引入
#[allow(unused_imports)]
use admin_infra::plugin::InfraPlugin;

#[tx_comp(init)]
pub struct AdminPlugin;

impl CompInit for AdminPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            // 注册 HTTP 路由（通过 WebPlugin 全局注册表）
            WebPlugin::add_router(api::router());
            info!("admin HTTP 路由已注册");
            Ok(())
        }
    );
    fn init_sort() -> i32 {
        // 在 InfraPlugin 之后初始化（确保数据库已连接且数据已初始化）
        i32::MAX - 100
    }
}
