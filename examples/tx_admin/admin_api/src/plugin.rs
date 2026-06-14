use std::sync::Arc;
use tracing::info;
use tx_di_axum::WebPlugin;
use tx_di_core::{App, CancellationToken, CompInit, RIE, async_method, tx_comp};
use tx_di_toasty::ToastyPlugin;

use crate::interfaces::api;

// 导入 toasty 模型
use admin_infra::user::model::{SysUser, SysUserRole, SysUserDept};
use admin_infra::role::model::{SysRole, SysRoleMenu};
use admin_infra::permission::model::SysPermission;
use admin_infra::menu::model::SysMenu;
use admin_infra::department::model::SysDepartment;
use admin_infra::file::model::{SysFile, SysFileConfig};
use admin_infra::config::model::SysConfig;
use admin_infra::dictionary::model::{SysDictType, SysDictData};
use admin_infra::log::model::{SysOperateLog, SysLoginLog};

#[tx_comp(init)]
pub struct AdminPlugin;

impl CompInit for AdminPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            // 注册 toasty 数据库模型（必须在数据库连接之前）
            let toasty_plugin = ctx.inject::<ToastyPlugin>();
            toasty_plugin.register_models(toasty::models!(
                SysUser, SysUserRole, SysUserDept,
                SysRole, SysRoleMenu,
                SysPermission,
                SysMenu,
                SysDepartment,
                SysFile, SysFileConfig,
                SysConfig,
                SysDictType, SysDictData,
                SysOperateLog, SysLoginLog
            ));
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
