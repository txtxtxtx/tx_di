use std::sync::Arc;
use tracing::info;
use tx_di_axum::{WebPlugin, WebConfig};
use tx_di_core::{App, CancellationToken, CompInit, RIE, async_method, tx_comp};
use tx_di_sa_token::{SaTokenPlugin, SaTokenLayer, SaCheckLoginLayer};

use crate::interfaces::api;

/// 确保 infra 层插件被编译引入
#[allow(unused_imports)]
use admin_infra::plugin::InfraPlugin;

#[tx_comp(init)]
pub struct AdminPlugin;

impl CompInit for AdminPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            // 获取 sa-token 状态
            let sa_plugin = ctx.inject::<SaTokenPlugin>();
            let sa_state = sa_plugin.state().clone();

            // 获取 WebConfig 的 max_body_size，用于文件上传 Content-Length 提前拦截
            let web_config = ctx.inject::<WebConfig>();
            let max_body_size = web_config.max_body_size as u64;

            // 构建路由：登录接口公开，其他接口需要认证
            let open = api::auth_api::open_router();
            let protected = api::router(max_body_size);

            let router = tx_di_axum::Router::new()
                .merge(open)
                .merge(
                    protected
                        .layer(SaCheckLoginLayer::new())
                        .layer(SaTokenLayer::new(sa_state))
                );

            WebPlugin::add_router(router);
            info!("admin HTTP 路由已注册（含认证）");
            Ok(())
        }
    );
    fn init_sort() -> i32 {
        // 在 InfraPlugin 之后初始化（确保数据库已连接且数据已初始化）
        i32::MAX - 100
    }
}
