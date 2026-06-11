//! 应用服务集合
//!
//! 使用 Mock 仓库组装 admin_app 各模块服务，
//! 供 HTTP / gRPC handler 通过 OnceLock 获取。

use std::sync::{Arc, OnceLock};

// ── admin_app 应用服务 ──
use admin_app::auth::app_service::AuthAppService;
use admin_app::user::app_service::UserAppService;
use admin_app::role::app_service::RoleAppService;
use admin_app::menu::app_service::MenuAppService;
use admin_app::department::app_service::DepartmentAppService;
use admin_app::permission::app_service::PermissionAppService;
use admin_app::config::app_service::ConfigAppService;
use admin_app::dictionary::app_service::{DictTypeAppService, DictDataAppService};
use admin_app::log::app_service::{OperateLogAppService, LoginLogAppService};
use admin_app::file::app_service::FileAppService;

// ── admin_domain 领域服务 ──
use admin_domain::user::service::UserService;
use admin_domain::role::service::RoleService;
use admin_domain::menu::service::MenuService;
use admin_domain::department::service::DepartmentService;
use admin_domain::permission::service::PermissionService;
use admin_domain::config::service::ConfigService;
use admin_domain::dictionary::service::{DictTypeService, DictDataService};
use admin_domain::log::service::{OperateLogService, LoginLogService};
use admin_domain::file::service::FileService;

// ── Mock 仓库 ──
use admin_app::mock::user_repo::MockUserRepository;
use admin_app::mock::role_repo::MockRoleRepository;
use admin_app::mock::menu_repo::MockMenuRepository;
use admin_app::mock::department_repo::MockDepartmentRepository;
use admin_app::mock::permission_repo::MockPermissionRepository;
use admin_app::mock::config_repo::MockConfigRepository;
use admin_app::mock::dict_repo::{MockDictTypeRepository, MockDictDataRepository};
use admin_app::mock::log_repo::{MockOperateLogRepository, MockLoginLogRepository};
use admin_app::mock::file_repo::{MockFileRepository, MockFileConfigRepository};

/// 所有应用服务的集合
pub struct Svc {
    pub auth:       AuthAppService,
    pub user:       UserAppService,
    pub role:       RoleAppService,
    pub menu:       MenuAppService,
    pub dept:       DepartmentAppService,
    pub perm:       PermissionAppService,
    pub config:     ConfigAppService,
    pub dict_type:  DictTypeAppService,
    pub dict_data:  DictDataAppService,
    pub oper_log:   OperateLogAppService,
    pub login_log:  LoginLogAppService,
    pub file:       FileAppService,
}

static SERVICES: OnceLock<Arc<Svc>> = OnceLock::new();

/// 初始化服务（plugin init 时调用一次）
pub fn init_services() {
    // ── 创建 Mock 仓库 ──
    let user_repo       = Arc::new(MockUserRepository::new());
    let role_repo       = Arc::new(MockRoleRepository::new());
    let menu_repo       = Arc::new(MockMenuRepository::new());
    let dept_repo       = Arc::new(MockDepartmentRepository::new());
    let perm_repo       = Arc::new(MockPermissionRepository::new());
    let config_repo     = Arc::new(MockConfigRepository::new());
    let dict_type_repo  = Arc::new(MockDictTypeRepository::new());
    let dict_data_repo  = Arc::new(MockDictDataRepository::new());
    let oper_log_repo   = Arc::new(MockOperateLogRepository::new());
    let login_log_repo  = Arc::new(MockLoginLogRepository::new());
    let file_repo        = Arc::new(MockFileRepository::new());
    let file_config_repo = Arc::new(MockFileConfigRepository::new());

    // ── 领域服务 ──
    let user_svc       = Arc::new(UserService::new(user_repo.clone(), perm_repo.clone()));
    let role_svc       = Arc::new(RoleService::new(role_repo.clone()));
    let menu_svc       = Arc::new(MenuService::new(menu_repo.clone()));
    let dept_svc       = Arc::new(DepartmentService::new(dept_repo.clone()));
    let perm_svc       = Arc::new(PermissionService::new(perm_repo.clone()));
    let config_svc     = Arc::new(ConfigService::new(config_repo.clone()));
    let dict_type_svc  = Arc::new(DictTypeService::new(dict_type_repo.clone()));
    let dict_data_svc  = Arc::new(DictDataService::new(dict_data_repo.clone()));
    let oper_log_svc   = Arc::new(OperateLogService::new(oper_log_repo.clone()));
    let login_log_svc  = Arc::new(LoginLogService::new(login_log_repo.clone()));
    let file_svc       = Arc::new(FileService::new(file_repo.clone(), file_config_repo.clone()));

    // ── 应用服务 ──
    let svc = Svc {
        auth:       AuthAppService::new(user_svc.clone(), role_svc.clone(), perm_svc.clone()),
        user:       UserAppService::new(user_svc.clone()),
        role:       RoleAppService::new(role_svc.clone()),
        menu:       MenuAppService::new(menu_svc.clone()),
        dept:       DepartmentAppService::new(dept_svc.clone()),
        perm:       PermissionAppService::new(perm_svc.clone()),
        config:     ConfigAppService::new(config_svc.clone()),
        dict_type:  DictTypeAppService::new(dict_type_svc.clone()),
        dict_data:  DictDataAppService::new(dict_data_svc.clone()),
        oper_log:   OperateLogAppService::new(oper_log_svc.clone()),
        login_log:  LoginLogAppService::new(login_log_svc.clone()),
        file:       FileAppService::new(file_svc.clone()),
    };

    let _ = SERVICES.set(Arc::new(svc));
}

/// 获取全局服务引用
pub fn get() -> &'static Arc<Svc> {
    SERVICES.get().expect("services not initialized, call init_services() first")
}
