use admin_proto::admin::auth::auth_service_server::AuthServiceServer;
use admin_proto::admin::config::config_service_server::ConfigServiceServer;
use admin_proto::admin::dept::department_service_server::DepartmentServiceServer;
use admin_proto::admin::dict::dict_service_server::DictServiceServer;
use admin_proto::admin::file::file_service_server::FileServiceServer;
use admin_proto::admin::log::log_service_server::LogServiceServer;
use admin_proto::admin::menu::menu_service_server::MenuServiceServer;
use admin_proto::admin::permission::permission_service_server::PermissionServiceServer;
use admin_proto::admin::role::role_service_server::RoleServiceServer;
use admin_proto::admin::user::user_service_server::UserServiceServer;
use std::sync::Arc;
use tracing::info;
use tx_di_axum::WebPlugin;
use tx_di_core::{App, CancellationToken, CompInit, RIE, async_method, tx_comp};

use crate::interfaces::api;
use crate::services;

#[tx_comp(init)]
pub struct AdminPlugin;

impl CompInit for AdminPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            // 注册 HTTP 路由（通过 WebPlugin 全局注册表）
            WebPlugin::add_router(api::router(ctx));
            info!("admin HTTP 路由已注册");
            Ok(())
        }
    );
    fn init_sort() -> i32 {
        i32::MAX - 100
    }
}
