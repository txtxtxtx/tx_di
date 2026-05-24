//! API 路由注册组件
//!
//! 在 DI 框架的 inner_init 阶段（早于 WebPlugin 的 i32::MAX）完成所有 API 路由注册，
//! 确保 WebPlugin::inner_init 调用 merge_routers() 时路由已经就位。

use std::sync::Arc;
use tracing::info;
use tx_di_axum::{ WebPlugin};
use tx_di_core::{BuildContext, CompInit, InnerContext, RIE, tx_comp, App, CancellationToken, BoxFuture};
use tx_di_gb28181::Gb28181Server;
use tx_di_sa_token::SaTokenPlugin;
use tx_di_toasty::ToastyPlugin;

use crate::{api, models};

/// API 路由注册组件
///
/// init_sort = i32::MAX - 100，早于 WebPlugin（i32::MAX）执行，
/// 确保路由在 WebPlugin::merge_routers() 之前注册到 ROUTER_REGISTRY。
#[tx_comp(init)]
pub struct ApiRegisterComponent {}

impl CompInit for ApiRegisterComponent {

    fn inner_init(&mut self, ctx: &InnerContext) -> RIE<()>{
        let ctx: BuildContext = ctx.into();
        let toasty_plugin = ctx.inject::<ToastyPlugin>();
        // 1. 在 build 之前注册事件监听器（用于 SSE 推送）
        Gb28181Server::on_event(|event| async move {
            api::sse::broadcast_event(event);
            Ok(())
        });

        // 2. 【关键】在 BuildContext::new() 之后、build() 之前注册数据库模型
        //    可以多次调用 register_models()，模型会合并（重复 ModelId 自动覆盖）
        toasty_plugin.register_models(toasty::models!(
            models::User,
            models::GbDeviceRecord,
            models::GbSessionRecord,
            models::GbAlarmRecord,
            models::GbAuditLog,
            models::GbDeviceGroup,
            models::GbDeviceGroupMember,
            models::GbRegisterAudit,
        ));
        Ok(())
    }
    fn async_init(ctx: Arc<App>, _token: CancellationToken) -> BoxFuture {
        let toasty_plugin = ctx.inject::<ToastyPlugin>();

        let db = toasty_plugin.db().clone();

        // 5. 获取 sa_token 插件实例，提取 SaTokenState
        let sa_plugin = ctx.inject::<SaTokenPlugin>();
        let sa_state = sa_plugin.state().clone();

        // 6. 注册带 State 的 API 路由
        WebPlugin::add_router(api::router(db, sa_state));
        info!("gb28181_admin 初始化完成");

        Box::pin(async {
            Ok(())
        })
    }

    fn init_sort() -> i32 {
        i32::MAX - 100
    }
}
