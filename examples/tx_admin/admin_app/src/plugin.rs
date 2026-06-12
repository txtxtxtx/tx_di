use std::any::Any;
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
use std::sync::Arc;
use tx_di_core::{App, CancellationToken, CompInit, RIE, async_method, tx_comp, CompRef};

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

            // ── 领域服务 ──
            let user_svc = UserService::new(user_repo.clone(), perm_repo.clone());
            let role_svc = RoleService::new(role_repo.clone());
            let menu_svc = MenuService::new(menu_repo.clone());
            let dept_svc = DepartmentService::new(dept_repo.clone());
            let perm_svc = PermissionService::new(perm_repo.clone());
            let config_svc = ConfigService::new(config_repo.clone());
            let dict_type_svc = DictTypeService::new(dict_type_repo.clone());
            let dict_data_svc = DictDataService::new(dict_data_repo.clone());
            let oper_log_svc = OperateLogService::new(oper_log_repo.clone());
            let login_log_svc = LoginLogService::new(login_log_repo.clone());
            let file_svc = FileService::new(file_repo.clone(), file_config_repo.clone());

            ctx.store.insert(user_svc.type_id(), CompRef::Cached(Arc::new(user_svc)));
            ctx.store.insert(role_svc.type_id(), CompRef::Cached(Arc::new(role_svc)));
            ctx.store.insert(menu_svc.type_id(), CompRef::Cached(Arc::new(menu_svc)));
            ctx.store.insert(dept_svc.type_id(), CompRef::Cached(Arc::new(dept_svc)));
            ctx.store.insert(perm_svc.type_id(), CompRef::Cached(Arc::new(perm_svc)));
            ctx.store.insert(config_svc.type_id(), CompRef::Cached(Arc::new(config_svc)));
            ctx.store.insert(dict_type_svc.type_id(), CompRef::Cached(Arc::new(dict_type_svc)));
            ctx.store.insert(dict_data_svc.type_id(), CompRef::Cached(Arc::new(dict_data_svc)));
            ctx.store.insert(oper_log_svc.type_id(), CompRef::Cached(Arc::new(oper_log_svc)));
            ctx.store.insert(login_log_svc.type_id(), CompRef::Cached(Arc::new(login_log_svc)));
            ctx.store.insert(file_svc.type_id(), CompRef::Cached(Arc::new(file_svc)));
            Ok(())
        }
    );
    fn init_sort() -> i32 {
        i32::MAX - 101
    }
}
