//! Common test helpers — shared by all integration test files.
//!
//! 使用真实 toasty 内存数据库进行测试。

use std::sync::{Arc, OnceLock, RwLock};

use tx_di_toasty::ToastyPlugin;
use toasty::ModelSet;

use admin_app::auth::app_service::AuthAppService;
use admin_app::config::app_service::ConfigAppService;
use admin_app::department::app_service::DepartmentAppService;
use admin_app::dictionary::app_service::{DictDataAppService, DictTypeAppService};
use admin_app::file::app_service::FileAppService;
use admin_app::log::app_service::{LoginLogAppService, OperateLogAppService};
use admin_app::menu::app_service::MenuAppService;
use admin_app::role::app_service::RoleAppService;
use admin_app::user::app_service::UserAppService;

use admin_domain::config::service::ConfigService;
use admin_domain::department::service::DepartmentService;
use admin_domain::dictionary::service::{DictDataService, DictTypeService};
use admin_domain::file::service::FileService;
use admin_domain::log::service::{LoginLogService, OperateLogService};
use admin_domain::menu::service::MenuService;
use admin_domain::role::service::RoleService;
use admin_domain::user::service::UserService;

use admin_infra::user::repository::ToastyUserRepository;
use admin_infra::role::repository::ToastyRoleRepository;
use admin_infra::menu::repository::ToastyMenuRepository;
use admin_infra::department::repository::ToastyDepartmentRepository;
use admin_infra::config::repository::ToastyConfigRepository;
use admin_infra::dictionary::repository::{ToastyDictTypeRepository, ToastyDictDataRepository};
use admin_infra::file::repository::{ToastyFileRepository, ToastyFileConfigRepository};
use admin_infra::log::repository::{ToastyOperateLogRepository, ToastyLoginLogRepository};

use admin_infra::user::model::{SysUser, SysUserRole, SysUserDept};
use admin_infra::role::model::{SysRole, SysRoleMenu};
use admin_infra::menu::model::SysMenu;
use admin_infra::department::model::SysDepartment;
use admin_infra::file::model::{SysFile, SysFileConfig};
use admin_infra::config::model::SysConfig;
use admin_infra::dictionary::model::{SysDictType, SysDictData};
use admin_infra::log::model::{SysOperateLog, SysLoginLog};

/// 创建内存 SQLite 数据库插件
pub async fn create_db_plugin() -> Arc<ToastyPlugin> {
    let mut builder = toasty::Db::builder();
    builder.models(toasty::models!(
        SysUser, SysUserRole, SysUserDept,
        SysRole, SysRoleMenu,
        SysMenu,
        SysDepartment,
        SysFile, SysFileConfig,
        SysConfig,
        SysDictType, SysDictData,
        SysOperateLog, SysLoginLog
    ));
    let db = builder.connect("sqlite::memory:").await.unwrap();
    db.push_schema().await.unwrap();

    Arc::new(ToastyPlugin {
        config: Arc::new(tx_di_toasty::ToastyConfig::default()),
        db: OnceLock::from(db),
        models: Arc::new(RwLock::new(ModelSet::new())),
    })
}

// ── User helpers ───────────────────────────────────────────────────────────

pub async fn create_user_service() -> (Arc<UserService>, Arc<ToastyUserRepository>) {
    let plugin = create_db_plugin().await;
    let user_repo = Arc::new(ToastyUserRepository::new(plugin.clone()));
    let role_repo = Arc::new(ToastyRoleRepository::new(plugin.clone()));
    let dept_repo = Arc::new(ToastyDepartmentRepository::new(plugin.clone()));
    let menu_repo = Arc::new(ToastyMenuRepository::new(plugin));
    let user_service = Arc::new(UserService::new(user_repo.clone(), role_repo, dept_repo, menu_repo));
    (user_service, user_repo)
}

pub async fn create_user_app() -> (UserAppService, Arc<UserService>, Arc<ToastyUserRepository>) {
    let (svc, repo) = create_user_service().await;
    let app = UserAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Role helpers ───────────────────────────────────────────────────────────

pub async fn create_role_service() -> (Arc<RoleService>, Arc<ToastyRoleRepository>) {
    let plugin = create_db_plugin().await;
    let role_repo = Arc::new(ToastyRoleRepository::new(plugin.clone()));
    let user_repo = Arc::new(ToastyUserRepository::new(plugin));
    let role_service = Arc::new(RoleService::new(role_repo.clone(), user_repo));
    (role_service, role_repo)
}

pub async fn create_role_app() -> (RoleAppService, Arc<RoleService>, Arc<ToastyRoleRepository>) {
    let (svc, repo) = create_role_service().await;
    let app = RoleAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Menu helpers ───────────────────────────────────────────────────────────

pub async fn create_menu_service() -> (Arc<MenuService>, Arc<ToastyMenuRepository>) {
    let plugin = create_db_plugin().await;
    let menu_repo = Arc::new(ToastyMenuRepository::new(plugin));
    let menu_service = Arc::new(MenuService::new(menu_repo.clone()));
    (menu_service, menu_repo)
}

pub async fn create_menu_app() -> (MenuAppService, Arc<MenuService>, Arc<ToastyMenuRepository>) {
    let (svc, repo) = create_menu_service().await;
    let app = MenuAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Department helpers ─────────────────────────────────────────────────────

pub async fn create_dept_service() -> (Arc<DepartmentService>, Arc<ToastyDepartmentRepository>) {
    let plugin = create_db_plugin().await;
    let dept_repo = Arc::new(ToastyDepartmentRepository::new(plugin));
    let dept_service = Arc::new(DepartmentService::new(dept_repo.clone()));
    (dept_service, dept_repo)
}

pub async fn create_dept_app() -> (DepartmentAppService, Arc<DepartmentService>, Arc<ToastyDepartmentRepository>) {
    let (svc, repo) = create_dept_service().await;
    let app = DepartmentAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Config helpers ─────────────────────────────────────────────────────────

pub async fn create_config_service() -> (Arc<ConfigService>, Arc<ToastyConfigRepository>) {
    let plugin = create_db_plugin().await;
    let config_repo = Arc::new(ToastyConfigRepository::new(plugin));
    let config_service = Arc::new(ConfigService::new(config_repo.clone()));
    (config_service, config_repo)
}

pub async fn create_config_app() -> (ConfigAppService, Arc<ConfigService>, Arc<ToastyConfigRepository>) {
    let (svc, repo) = create_config_service().await;
    let app = ConfigAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Dict helpers ───────────────────────────────────────────────────────────

pub async fn create_dict_type_service() -> (Arc<DictTypeService>, Arc<ToastyDictTypeRepository>) {
    let plugin = create_db_plugin().await;
    let dict_type_repo = Arc::new(ToastyDictTypeRepository::new(plugin));
    let dict_type_service = Arc::new(DictTypeService::new(dict_type_repo.clone()));
    (dict_type_service, dict_type_repo)
}

pub async fn create_dict_type_app() -> (DictTypeAppService, Arc<DictTypeService>, Arc<ToastyDictTypeRepository>) {
    let (svc, repo) = create_dict_type_service().await;
    let app = DictTypeAppService::new(svc.clone());
    (app, svc, repo)
}

pub async fn create_dict_data_service() -> (Arc<DictDataService>, Arc<ToastyDictDataRepository>) {
    let plugin = create_db_plugin().await;
    let dict_data_repo = Arc::new(ToastyDictDataRepository::new(plugin));
    let dict_data_service = Arc::new(DictDataService::new(dict_data_repo.clone()));
    (dict_data_service, dict_data_repo)
}

pub async fn create_dict_data_app() -> (DictDataAppService, Arc<DictDataService>, Arc<ToastyDictDataRepository>) {
    let (svc, repo) = create_dict_data_service().await;
    let app = DictDataAppService::new(svc.clone());
    (app, svc, repo)
}

// ── File helpers ───────────────────────────────────────────────────────────

pub async fn create_file_service() -> (Arc<FileService>, Arc<ToastyFileRepository>) {
    let plugin = create_db_plugin().await;
    let file_repo = Arc::new(ToastyFileRepository::new(plugin.clone()));
    let file_config_repo = Arc::new(ToastyFileConfigRepository::new(plugin));
    let file_service = Arc::new(FileService::new(file_repo.clone(), file_config_repo));
    (file_service, file_repo)
}

pub async fn create_file_app() -> (FileAppService, Arc<FileService>, Arc<ToastyFileRepository>) {
    let (svc, repo) = create_file_service().await;
    let app = FileAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Log helpers ────────────────────────────────────────────────────────────

pub async fn create_operate_log_service() -> (Arc<OperateLogService>, Arc<ToastyOperateLogRepository>) {
    let plugin = create_db_plugin().await;
    let log_repo = Arc::new(ToastyOperateLogRepository::new(plugin));
    let log_service = Arc::new(OperateLogService::new(log_repo.clone()));
    (log_service, log_repo)
}

pub async fn create_operate_log_app() -> (OperateLogAppService, Arc<OperateLogService>, Arc<ToastyOperateLogRepository>) {
    let (svc, repo) = create_operate_log_service().await;
    let app = OperateLogAppService::new(svc.clone());
    (app, svc, repo)
}

pub async fn create_login_log_service() -> (Arc<LoginLogService>, Arc<ToastyLoginLogRepository>) {
    let plugin = create_db_plugin().await;
    let log_repo = Arc::new(ToastyLoginLogRepository::new(plugin));
    let log_service = Arc::new(LoginLogService::new(log_repo.clone()));
    (log_service, log_repo)
}

pub async fn create_login_log_app() -> (LoginLogAppService, Arc<LoginLogService>, Arc<ToastyLoginLogRepository>) {
    let (svc, repo) = create_login_log_service().await;
    let app = LoginLogAppService::new(svc.clone());
    (app, svc, repo)
}
