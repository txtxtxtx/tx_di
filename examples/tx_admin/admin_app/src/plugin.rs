use std::any::Any;
use std::any::TypeId;
use std::sync::Arc;
use dashmap::DashMap;
use tx_di_core::{App, CancellationToken, CompInit, CompRef, ComponentDescriptor, RIE, Scope, async_method, tx_comp};

use crate::mock::config_repo::MockConfigRepository;
use crate::mock::department_repo::MockDepartmentRepository;
use crate::mock::dict_repo::{MockDictDataRepository, MockDictTypeRepository};
use crate::mock::file_repo::{MockFileConfigRepository, MockFileRepository};
use crate::mock::log_repo::{MockLoginLogRepository, MockOperateLogRepository};
use crate::mock::menu_repo::MockMenuRepository;
use crate::mock::permission_repo::MockPermissionRepository;
use crate::mock::role_repo::MockRoleRepository;
use crate::mock::user_repo::MockUserRepository;

use admin_domain::config::service::ConfigService;
use admin_domain::department::service::DepartmentService;
use admin_domain::dictionary::service::{DictDataService, DictTypeService};
use admin_domain::file::service::FileService;
use admin_domain::log::service::{LoginLogService, OperateLogService};
use admin_domain::menu::service::MenuService;
use admin_domain::permission::service::PermissionService;
use admin_domain::role::service::RoleService;
use admin_domain::user::service::UserService;

use crate::auth::app_service::AuthAppService;
use crate::user::app_service::UserAppService;
use crate::role::app_service::RoleAppService;
use crate::menu::app_service::MenuAppService;
use crate::department::app_service::DepartmentAppService;
use crate::permission::app_service::PermissionAppService;
use crate::config::app_service::ConfigAppService;
use crate::dictionary::app_service::{DictTypeAppService, DictDataAppService};
use crate::log::app_service::{OperateLogAppService, LoginLogAppService};
use crate::file::app_service::FileAppService;

// 为 AppService 实现 ComponentDescriptor，使其可通过 DiComp<T> 注入 axum handler
macro_rules! make_injectable {
    ($t:ty) => {
        impl CompInit for $t {}
        impl ComponentDescriptor for $t {
            const DEP_IDS: &'static [fn() -> std::any::TypeId] = &[];
            const SCOPE: Scope = Scope::Singleton;
            fn build(_store: &DashMap<TypeId, CompRef>) -> Self { panic!("use ctx.store.insert()") }
        }
    };
}

make_injectable!(AuthAppService);
make_injectable!(UserAppService);
make_injectable!(RoleAppService);
make_injectable!(MenuAppService);
make_injectable!(DepartmentAppService);
make_injectable!(PermissionAppService);
make_injectable!(ConfigAppService);
make_injectable!(DictTypeAppService);
make_injectable!(DictDataAppService);
make_injectable!(OperateLogAppService);
make_injectable!(LoginLogAppService);
make_injectable!(FileAppService);

#[tx_comp(init)]
pub struct AppPlugin;

impl CompInit for AppPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            // ── 创建 Mock 仓库 ──
            let user_repo = Arc::new(MockUserRepository::new());
            let role_repo = Arc::new(MockRoleRepository::new());
            let menu_repo = Arc::new(MockMenuRepository::new());
            let dept_repo = Arc::new(MockDepartmentRepository::new());
            let perm_repo = Arc::new(MockPermissionRepository::new());
            let config_repo = Arc::new(MockConfigRepository::new());
            let dict_type_repo = Arc::new(MockDictTypeRepository::new());
            let dict_data_repo = Arc::new(MockDictDataRepository::new());
            let oper_log_repo = Arc::new(MockOperateLogRepository::new());
            let login_log_repo = Arc::new(MockLoginLogRepository::new());
            let file_repo = Arc::new(MockFileRepository::new());
            let file_config_repo = Arc::new(MockFileConfigRepository::new());

            // ── 领域服务（Arc 化，后续可 clone）──
            let user_svc = Arc::new(UserService::new(user_repo.clone(), perm_repo.clone()));
            let role_svc = Arc::new(RoleService::new(role_repo.clone()));
            let menu_svc = Arc::new(MenuService::new(menu_repo.clone()));
            let dept_svc = Arc::new(DepartmentService::new(dept_repo.clone()));
            let perm_svc = Arc::new(PermissionService::new(perm_repo.clone()));
            let config_svc = Arc::new(ConfigService::new(config_repo.clone()));
            let dict_type_svc = Arc::new(DictTypeService::new(dict_type_repo.clone()));
            let dict_data_svc = Arc::new(DictDataService::new(dict_data_repo.clone()));
            let oper_log_svc = Arc::new(OperateLogService::new(oper_log_repo.clone()));
            let login_log_svc = Arc::new(LoginLogService::new(login_log_repo.clone()));
            let file_svc = Arc::new(FileService::new(file_repo.clone(), file_config_repo.clone()));

            // ── 应用服务 ──
            let auth_app = AuthAppService::new(user_svc.clone(), role_svc.clone(), perm_svc.clone());
            let user_app = UserAppService::new(user_svc.clone());
            let role_app = RoleAppService::new(role_svc.clone());
            let menu_app = MenuAppService::new(menu_svc.clone());
            let dept_app = DepartmentAppService::new(dept_svc.clone());
            let perm_app = PermissionAppService::new(perm_svc.clone());
            let config_app = ConfigAppService::new(config_svc.clone());
            let dict_type_app = DictTypeAppService::new(dict_type_svc.clone());
            let dict_data_app = DictDataAppService::new(dict_data_svc.clone());
            let oper_log_app = OperateLogAppService::new(oper_log_svc.clone());
            let login_log_app = LoginLogAppService::new(login_log_svc.clone());
            let file_app = FileAppService::new(file_svc.clone());

            // 注册到 DI 容器
            // ctx.store.insert(user_svc.type_id(), CompRef::Cached(user_svc));
            // ctx.store.insert(role_svc.type_id(), CompRef::Cached(role_svc));
            // ctx.store.insert(menu_svc.type_id(), CompRef::Cached(menu_svc));
            // ctx.store.insert(dept_svc.type_id(), CompRef::Cached(dept_svc));
            // ctx.store.insert(perm_svc.type_id(), CompRef::Cached(perm_svc));
            // ctx.store.insert(config_svc.type_id(), CompRef::Cached(config_svc));
            // ctx.store.insert(dict_type_svc.type_id(), CompRef::Cached(dict_type_svc));
            // ctx.store.insert(dict_data_svc.type_id(), CompRef::Cached(dict_data_svc));
            // ctx.store.insert(oper_log_svc.type_id(), CompRef::Cached(oper_log_svc));
            // ctx.store.insert(login_log_svc.type_id(), CompRef::Cached(login_log_svc));
            // ctx.store.insert(file_svc.type_id(), CompRef::Cached(file_svc));

            ctx.store.insert(auth_app.type_id(), CompRef::Cached(Arc::new(auth_app)));
            ctx.store.insert(user_app.type_id(), CompRef::Cached(Arc::new(user_app)));
            ctx.store.insert(role_app.type_id(), CompRef::Cached(Arc::new(role_app)));
            ctx.store.insert(menu_app.type_id(), CompRef::Cached(Arc::new(menu_app)));
            ctx.store.insert(dept_app.type_id(), CompRef::Cached(Arc::new(dept_app)));
            ctx.store.insert(perm_app.type_id(), CompRef::Cached(Arc::new(perm_app)));
            ctx.store.insert(config_app.type_id(), CompRef::Cached(Arc::new(config_app)));
            ctx.store.insert(dict_type_app.type_id(), CompRef::Cached(Arc::new(dict_type_app)));
            ctx.store.insert(dict_data_app.type_id(), CompRef::Cached(Arc::new(dict_data_app)));
            ctx.store.insert(oper_log_app.type_id(), CompRef::Cached(Arc::new(oper_log_app)));
            ctx.store.insert(login_log_app.type_id(), CompRef::Cached(Arc::new(login_log_app)));
            ctx.store.insert(file_app.type_id(), CompRef::Cached(Arc::new(file_app)));

            Ok(())
        }
    );
    fn init_sort() -> i32 {
        i32::MAX - 101
    }
}
