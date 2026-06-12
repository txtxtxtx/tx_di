//! Common test helpers — shared by all integration test files.

use std::sync::Arc;

use admin_app::mock::*;
use admin_app::auth::app_service::AuthAppService;
use admin_app::config::app_service::ConfigAppService;
use admin_app::department::app_service::DepartmentAppService;
use admin_app::dictionary::app_service::{DictDataAppService, DictTypeAppService};
use admin_app::file::app_service::FileAppService;
use admin_app::log::app_service::{LoginLogAppService, OperateLogAppService};
use admin_app::menu::app_service::MenuAppService;
use admin_app::permission::app_service::PermissionAppService;
use admin_app::role::app_service::RoleAppService;
use admin_app::user::app_service::UserAppService;

use admin_domain::config::service::ConfigService;
use admin_domain::department::service::DepartmentService;
use admin_domain::dictionary::service::{DictDataService, DictTypeService};
use admin_domain::file::service::FileService;
use admin_domain::log::service::{LoginLogService, OperateLogService};
use admin_domain::menu::service::MenuService;
use admin_domain::permission::service::PermissionService;
use admin_domain::role::service::RoleService;
use admin_domain::user::service::UserService;

// ── User helpers ───────────────────────────────────────────────────────────

pub fn create_user_service() -> (Arc<UserService>, Arc<user_repo::MockUserRepository>) {
    let user_repo = Arc::new(user_repo::MockUserRepository::new());
    let permission_repo = Arc::new(permission_repo::MockPermissionRepository::new());
    let user_service = Arc::new(UserService::new(user_repo.clone(), permission_repo));
    (user_service, user_repo)
}

pub fn create_user_app(
) -> (UserAppService, Arc<UserService>, Arc<user_repo::MockUserRepository>) {
    let (svc, repo) = create_user_service();
    let app = UserAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Role helpers ───────────────────────────────────────────────────────────

pub fn create_role_service() -> (Arc<RoleService>, Arc<role_repo::MockRoleRepository>) {
    let role_repo = Arc::new(role_repo::MockRoleRepository::new());
    let role_service = Arc::new(RoleService::new(role_repo.clone()));
    (role_service, role_repo)
}

pub fn create_role_app(
) -> (RoleAppService, Arc<RoleService>, Arc<role_repo::MockRoleRepository>) {
    let (svc, repo) = create_role_service();
    let app = RoleAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Menu helpers ───────────────────────────────────────────────────────────

pub fn create_menu_service() -> (Arc<MenuService>, Arc<menu_repo::MockMenuRepository>) {
    let menu_repo = Arc::new(menu_repo::MockMenuRepository::new());
    let menu_service = Arc::new(MenuService::new(menu_repo.clone()));
    (menu_service, menu_repo)
}

pub fn create_menu_app(
) -> (MenuAppService, Arc<MenuService>, Arc<menu_repo::MockMenuRepository>) {
    let (svc, repo) = create_menu_service();
    let app = MenuAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Department helpers ─────────────────────────────────────────────────────

pub fn create_dept_service(
) -> (Arc<DepartmentService>, Arc<department_repo::MockDepartmentRepository>) {
    let dept_repo = Arc::new(department_repo::MockDepartmentRepository::new());
    let dept_service = Arc::new(DepartmentService::new(dept_repo.clone()));
    (dept_service, dept_repo)
}

pub fn create_dept_app(
) -> (
    DepartmentAppService,
    Arc<DepartmentService>,
    Arc<department_repo::MockDepartmentRepository>,
) {
    let (svc, repo) = create_dept_service();
    let app = DepartmentAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Config helpers ─────────────────────────────────────────────────────────

pub fn create_config_service(
) -> (Arc<ConfigService>, Arc<config_repo::MockConfigRepository>) {
    let config_repo = Arc::new(config_repo::MockConfigRepository::new());
    let config_service = Arc::new(ConfigService::new(config_repo.clone()));
    (config_service, config_repo)
}

pub fn create_config_app(
) -> (ConfigAppService, Arc<ConfigService>, Arc<config_repo::MockConfigRepository>) {
    let (svc, repo) = create_config_service();
    let app = ConfigAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Dict helpers ───────────────────────────────────────────────────────────

pub fn create_dict_type_service(
) -> (Arc<DictTypeService>, Arc<dict_repo::MockDictTypeRepository>) {
    let dict_type_repo = Arc::new(dict_repo::MockDictTypeRepository::new());
    let dict_type_service = Arc::new(DictTypeService::new(dict_type_repo.clone()));
    (dict_type_service, dict_type_repo)
}

pub fn create_dict_type_app(
) -> (
    DictTypeAppService,
    Arc<DictTypeService>,
    Arc<dict_repo::MockDictTypeRepository>,
) {
    let (svc, repo) = create_dict_type_service();
    let app = DictTypeAppService::new(svc.clone());
    (app, svc, repo)
}

pub fn create_dict_data_service(
) -> (Arc<DictDataService>, Arc<dict_repo::MockDictDataRepository>) {
    let dict_data_repo = Arc::new(dict_repo::MockDictDataRepository::new());
    let dict_data_service = Arc::new(DictDataService::new(dict_data_repo.clone()));
    (dict_data_service, dict_data_repo)
}

pub fn create_dict_data_app(
) -> (
    DictDataAppService,
    Arc<DictDataService>,
    Arc<dict_repo::MockDictDataRepository>,
) {
    let (svc, repo) = create_dict_data_service();
    let app = DictDataAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Permission helpers ─────────────────────────────────────────────────────

pub fn create_permission_service(
) -> (Arc<PermissionService>, Arc<permission_repo::MockPermissionRepository>) {
    let permission_repo = Arc::new(permission_repo::MockPermissionRepository::new());
    let permission_service = Arc::new(PermissionService::new(permission_repo.clone()));
    (permission_service, permission_repo)
}

pub fn create_permission_app(
) -> (
    PermissionAppService,
    Arc<PermissionService>,
    Arc<permission_repo::MockPermissionRepository>,
) {
    let (svc, repo) = create_permission_service();
    let app = PermissionAppService::new(svc.clone());
    (app, svc, repo)
}

// ── File helpers ───────────────────────────────────────────────────────────

pub fn create_file_service() -> (Arc<FileService>, Arc<file_repo::MockFileRepository>) {
    let file_repo = Arc::new(file_repo::MockFileRepository::new());
    let file_config_repo = Arc::new(file_repo::MockFileConfigRepository::new());
    let file_service = Arc::new(FileService::new(file_repo.clone(), file_config_repo));
    (file_service, file_repo)
}

pub fn create_file_app(
) -> (FileAppService, Arc<FileService>, Arc<file_repo::MockFileRepository>) {
    let (svc, repo) = create_file_service();
    let app = FileAppService::new(svc.clone());
    (app, svc, repo)
}

// ── Log helpers ────────────────────────────────────────────────────────────

pub fn create_operate_log_service(
) -> (Arc<OperateLogService>, Arc<log_repo::MockOperateLogRepository>) {
    let log_repo = Arc::new(log_repo::MockOperateLogRepository::new());
    let log_service = Arc::new(OperateLogService::new(log_repo.clone()));
    (log_service, log_repo)
}

pub fn create_operate_log_app(
) -> (
    OperateLogAppService,
    Arc<OperateLogService>,
    Arc<log_repo::MockOperateLogRepository>,
) {
    let (svc, repo) = create_operate_log_service();
    let app = OperateLogAppService::new(svc.clone());
    (app, svc, repo)
}

pub fn create_login_log_service(
) -> (Arc<LoginLogService>, Arc<log_repo::MockLoginLogRepository>) {
    let log_repo = Arc::new(log_repo::MockLoginLogRepository::new());
    let log_service = Arc::new(LoginLogService::new(log_repo.clone()));
    (log_service, log_repo)
}

pub fn create_login_log_app(
) -> (
    LoginLogAppService,
    Arc<LoginLogService>,
    Arc<log_repo::MockLoginLogRepository>,
) {
    let (svc, repo) = create_login_log_service();
    let app = LoginLogAppService::new(svc.clone());
    (app, svc, repo)
}