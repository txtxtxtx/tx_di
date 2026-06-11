//! Integration tests for DDD domain and application layers
//! Uses mock repositories to verify functionality without real database

use std::collections::HashSet;
use std::sync::Arc;

use admin_app::mock::*;
use admin_app::user::dto::*;
use admin_app::user::app_service::UserAppService;
use admin_app::role::dto::*;
use admin_app::role::app_service::RoleAppService;
use admin_app::menu::dto::*;
use admin_app::menu::app_service::MenuAppService;
use admin_app::department::dto::*;
use admin_app::department::app_service::DepartmentAppService;
use admin_app::config::dto::*;
use admin_app::config::app_service::ConfigAppService;
use admin_app::dictionary::dto::*;
use admin_app::dictionary::app_service::{DictTypeAppService, DictDataAppService};
use admin_app::permission::dto::*;
use admin_app::permission::app_service::PermissionAppService;
use admin_app::auth::dto::*;
use admin_app::auth::app_service::AuthAppService;

use admin_domain::user::service::UserService;
use admin_domain::role::service::RoleService;
use admin_domain::menu::service::MenuService;
use admin_domain::department::service::DepartmentService;
use admin_domain::config::service::ConfigService;
use admin_domain::dictionary::service::{DictDataService, DictTypeService};
use admin_domain::permission::service::PermissionService;
use admin_domain::user::model::aggregate::User;
use admin_domain::user::model::value_object::{Sex, UserStatus};
use admin_domain::role::model::aggregate::Role;
use admin_domain::menu::model::aggregate::Menu;
use admin_domain::department::model::aggregate::Department;

use tx_common::id;

/// Helper to create user service with mock repos
fn create_user_service() -> (Arc<UserService>, Arc<user_repo::MockUserRepository>) {
    let user_repo = Arc::new(user_repo::MockUserRepository::new());
    let permission_repo = Arc::new(permission_repo::MockPermissionRepository::new());
    let user_service = Arc::new(UserService::new(user_repo.clone(), permission_repo));
    (user_service, user_repo)
}

/// Helper to create role service with mock repos
fn create_role_service() -> (Arc<RoleService>, Arc<role_repo::MockRoleRepository>) {
    let role_repo = Arc::new(role_repo::MockRoleRepository::new());
    let role_service = Arc::new(RoleService::new(role_repo.clone()));
    (role_service, role_repo)
}

/// Helper to create menu service with mock repos
fn create_menu_service() -> (Arc<MenuService>, Arc<menu_repo::MockMenuRepository>) {
    let menu_repo = Arc::new(menu_repo::MockMenuRepository::new());
    let menu_service = Arc::new(MenuService::new(menu_repo.clone()));
    (menu_service, menu_repo)
}

/// Helper to create department service with mock repos
fn create_dept_service() -> (Arc<DepartmentService>, Arc<department_repo::MockDepartmentRepository>) {
    let dept_repo = Arc::new(department_repo::MockDepartmentRepository::new());
    let dept_service = Arc::new(DepartmentService::new(dept_repo.clone()));
    (dept_service, dept_repo)
}

/// Helper to create config service with mock repos
fn create_config_service() -> (Arc<ConfigService>, Arc<config_repo::MockConfigRepository>) {
    let config_repo = Arc::new(config_repo::MockConfigRepository::new());
    let config_service = Arc::new(ConfigService::new(config_repo.clone()));
    (config_service, config_repo)
}

/// Helper to create dict type service with mock repos
fn create_dict_type_service() -> (Arc<DictTypeService>, Arc<dict_repo::MockDictTypeRepository>) {
    let dict_type_repo = Arc::new(dict_repo::MockDictTypeRepository::new());
    let dict_type_service = Arc::new(DictTypeService::new(dict_type_repo.clone()));
    (dict_type_service, dict_type_repo)
}

/// Helper to create dict data service with mock repos
fn create_dict_data_service() -> (Arc<DictDataService>, Arc<dict_repo::MockDictDataRepository>) {
    let dict_data_repo = Arc::new(dict_repo::MockDictDataRepository::new());
    let dict_data_service = Arc::new(DictDataService::new(dict_data_repo.clone()));
    (dict_data_service, dict_data_repo)
}

// ============================================================================
// User Tests
// ============================================================================

#[tokio::test]
async fn test_create_user() {
    let (user_service, _) = create_user_service();
    let app_service = UserAppService::new(user_service);

    let cmd = CreateUserCommand {
        username: "testuser".to_string(),
        password: "password123".to_string(),
        nickname: "Test User".to_string(),
        email: Some("test@example.com".to_string()),
        mobile: Some("13800138000".to_string()),
        sex: Some(Sex::Male),
        remark: None,
        role_ids: None,
        dept_ids: None,
    };

    let result = app_service.create_user(cmd, Some("admin".to_string())).await;
    assert!(result.is_ok());

    let user = result.unwrap();
    assert_eq!(user.username, "testuser");
    assert_eq!(user.nickname, "Test User");
    assert_eq!(user.email, Some("test@example.com".to_string()));
    assert_eq!(user.mobile, Some("13800138000".to_string()));
    assert_eq!(user.sex, Sex::Male);
}

#[tokio::test]
async fn test_create_duplicate_user() {
    let (user_service, _) = create_user_service();
    let app_service = UserAppService::new(user_service);

    let cmd1 = CreateUserCommand {
        username: "testuser".to_string(),
        password: "password123".to_string(),
        nickname: "Test User 1".to_string(),
        email: None,
        mobile: None,
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };

    let cmd2 = CreateUserCommand {
        username: "testuser".to_string(),
        password: "password456".to_string(),
        nickname: "Test User 2".to_string(),
        email: None,
        mobile: None,
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };

    // First creation should succeed
    let result1 = app_service.create_user(cmd1, Some("admin".to_string())).await;
    assert!(result1.is_ok());

    // Second creation with same username should fail
    let result2 = app_service.create_user(cmd2, Some("admin".to_string())).await;
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_update_user() {
    let (user_service, _) = create_user_service();
    let app_service = UserAppService::new(user_service);

    // Create user
    let create_cmd = CreateUserCommand {
        username: "testuser".to_string(),
        password: "password123".to_string(),
        nickname: "Test User".to_string(),
        email: None,
        mobile: None,
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };

    let user = app_service.create_user(create_cmd, Some("admin".to_string())).await.unwrap();

    // Update user
    let update_cmd = UpdateUserCommand {
        user_id: user.id,
        nickname: "Updated User".to_string(),
        email: Some("updated@example.com".to_string()),
        mobile: Some("13900139000".to_string()),
        sex: Sex::Female,
        remark: Some("Updated remark".to_string()),
    };

    let result = app_service.update_user(update_cmd, Some("admin".to_string())).await;
    assert!(result.is_ok());

    let updated = result.unwrap();
    assert_eq!(updated.nickname, "Updated User");
    assert_eq!(updated.email, Some("updated@example.com".to_string()));
    assert_eq!(updated.sex, Sex::Female);
}

#[tokio::test]
async fn test_delete_user() {
    let (user_service, _) = create_user_service();
    let app_service = UserAppService::new(user_service);

    // Create user
    let create_cmd = CreateUserCommand {
        username: "testuser".to_string(),
        password: "password123".to_string(),
        nickname: "Test User".to_string(),
        email: None,
        mobile: None,
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };

    let user = app_service.create_user(create_cmd, Some("admin".to_string())).await.unwrap();

    // Delete user
    let result = app_service.delete_user(user.id, Some("admin".to_string())).await;
    assert!(result.is_ok());

    // Try to get deleted user should fail
    let get_result = app_service.get_user(user.id).await;
    assert!(get_result.is_err());
}

#[tokio::test]
async fn test_change_user_status() {
    let (user_service, _) = create_user_service();
    let app_service = UserAppService::new(user_service);

    // Create user
    let create_cmd = CreateUserCommand {
        username: "testuser".to_string(),
        password: "password123".to_string(),
        nickname: "Test User".to_string(),
        email: None,
        mobile: None,
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };

    let user = app_service.create_user(create_cmd, Some("admin".to_string())).await.unwrap();
    assert_eq!(user.status, UserStatus::Active); // Active

    // Change to disabled
    let result = app_service.change_status(user.id, UserStatus::Disabled, Some("admin".to_string())).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, UserStatus::Disabled);
}

#[tokio::test]
async fn test_user_pagination() {
    let (user_service, _) = create_user_service();
    let app_service = UserAppService::new(user_service);

    // Create 5 users
    for i in 0..5 {
        let cmd = CreateUserCommand {
            username: format!("user{}", i),
            password: "password".to_string(),
            nickname: format!("User {}", i),
            email: None,
            mobile: None,
            sex: None,
            remark: None,
            role_ids: None,
            dept_ids: None,
        };
        app_service.create_user(cmd, Some("admin".to_string())).await.unwrap();
    }

    // Get page 1 with size 2
    let query = UserQueryRequest {
        username: None,
        nickname: None,
        mobile: None,
        status: None,
        dept_id: None,
        page: 1,
        page_size: 2,
    };

    let result = app_service.get_user_page(query).await.unwrap();
    assert_eq!(result.list.len(), 2);
    assert_eq!(result.total, 5);

    // Get page 2 with size 2
    let query2 = UserQueryRequest {
        username: None,
        nickname: None,
        mobile: None,
        status: None,
        dept_id: None,
        page: 2,
        page_size: 2,
    };

    let result2 = app_service.get_user_page(query2).await.unwrap();
    assert_eq!(result2.list.len(), 2);

    // Get page 3 with size 2
    let query3 = UserQueryRequest {
        username: None,
        nickname: None,
        mobile: None,
        status: None,
        dept_id: None,
        page: 3,
        page_size: 2,
    };

    let result3 = app_service.get_user_page(query3).await.unwrap();
    assert_eq!(result3.list.len(), 1);
}

#[tokio::test]
async fn test_assign_roles_to_user() {
    let (user_service, _) = create_user_service();
    let app_service = UserAppService::new(user_service);

    // Create user
    let create_cmd = CreateUserCommand {
        username: "testuser".to_string(),
        password: "password123".to_string(),
        nickname: "Test User".to_string(),
        email: None,
        mobile: None,
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };

    let user = app_service.create_user(create_cmd, Some("admin".to_string())).await.unwrap();

    // Assign roles
    let assign_cmd = AssignRolesCommand {
        user_id: user.id,
        role_ids: vec![1, 2, 3],
    };

    let result = app_service.assign_roles(assign_cmd).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_assign_departments_to_user() {
    let (user_service, _) = create_user_service();
    let app_service = UserAppService::new(user_service);

    // Create user
    let create_cmd = CreateUserCommand {
        username: "testuser".to_string(),
        password: "password123".to_string(),
        nickname: "Test User".to_string(),
        email: None,
        mobile: None,
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };

    let user = app_service.create_user(create_cmd, Some("admin".to_string())).await.unwrap();

    // Assign departments
    let assign_cmd = AssignDeptsCommand {
        user_id: user.id,
        dept_ids: vec![100, 200],
    };

    let result = app_service.assign_departments(assign_cmd).await;
    assert!(result.is_ok());
}

// ============================================================================
// Role Tests
// ============================================================================

#[tokio::test]
async fn test_create_role() {
    let (role_service, _) = create_role_service();
    let app_service = RoleAppService::new(role_service);

    let cmd = CreateRoleCommand {
        name: "管理员".to_string(),
        code: "admin".to_string(),
        sort: 1,
        remark: Some("系统管理员角色".to_string()),
        menu_ids: None,
    };

    let result = app_service.create_role(cmd, Some("admin".to_string())).await;
    assert!(result.is_ok());

    let role = result.unwrap();
    assert_eq!(role.name, "管理员");
    assert_eq!(role.code, "admin");
    assert_eq!(role.sort, 1);
}

#[tokio::test]
async fn test_create_duplicate_role_code() {
    let (role_service, _) = create_role_service();
    let app_service = RoleAppService::new(role_service);

    let cmd1 = CreateRoleCommand {
        name: "管理员".to_string(),
        code: "admin".to_string(),
        sort: 1,
        remark: None,
        menu_ids: None,
    };

    let cmd2 = CreateRoleCommand {
        name: "超级管理员".to_string(),
        code: "admin".to_string(),
        sort: 2,
        remark: None,
        menu_ids: None,
    };

    // First creation should succeed
    let result1 = app_service.create_role(cmd1, Some("admin".to_string())).await;
    assert!(result1.is_ok());

    // Second creation with same code should fail
    let result2 = app_service.create_role(cmd2, Some("admin".to_string())).await;
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_update_role() {
    let (role_service, _) = create_role_service();
    let app_service = RoleAppService::new(role_service);

    // Create role
    let create_cmd = CreateRoleCommand {
        name: "管理员".to_string(),
        code: "admin".to_string(),
        sort: 1,
        remark: None,
        menu_ids: None,
    };

    let role = app_service.create_role(create_cmd, Some("admin".to_string())).await.unwrap();

    // Update role
    let update_cmd = UpdateRoleCommand {
        role_id: role.id,
        name: "超级管理员".to_string(),
        code: "super_admin".to_string(),
        sort: 2,
        data_scope: 1,
        remark: Some("超级管理员角色".to_string()),
    };

    let result = app_service.update_role(update_cmd, Some("admin".to_string())).await;
    assert!(result.is_ok());

    let updated = result.unwrap();
    assert_eq!(updated.name, "超级管理员");
    assert_eq!(updated.code, "super_admin");
    assert_eq!(updated.data_scope, 1);
}

#[tokio::test]
async fn test_assign_menus_to_role() {
    let (role_service, _) = create_role_service();
    let app_service = RoleAppService::new(role_service);

    // Create role
    let create_cmd = CreateRoleCommand {
        name: "管理员".to_string(),
        code: "admin".to_string(),
        sort: 1,
        remark: None,
        menu_ids: None,
    };

    let role = app_service.create_role(create_cmd, Some("admin".to_string())).await.unwrap();

    // Assign menus
    let assign_cmd = AssignMenusCommand {
        role_id: role.id,
        menu_ids: vec![1, 2, 3, 4, 5],
    };

    let result = app_service.assign_menus(assign_cmd).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().menu_ids, vec![1, 2, 3, 4, 5]);
}

// ============================================================================
// Menu Tests
// ============================================================================

#[tokio::test]
async fn test_create_menu() {
    let (menu_service, _) = create_menu_service();
    let app_service = MenuAppService::new(menu_service);

    let cmd = CreateMenuCommand {
        name: "用户管理".to_string(),
        permission: "system:user:list".to_string(),
        types: 1,
        sort: 1,
        parent_id: 0,
        path: Some("/system/user".to_string()),
        icon: Some("user".to_string()),
        component: Some("system/user/index".to_string()),
        component_name: Some("UserManagement".to_string()),
    };

    let result = app_service.create_menu(cmd, Some("admin".to_string())).await;
    assert!(result.is_ok());

    let menu = result.unwrap();
    assert_eq!(menu.name, "用户管理");
    assert_eq!(menu.permission, "system:user:list");
    assert_eq!(menu.types, 1);
    assert_eq!(menu.parent_id, 0);
}

#[tokio::test]
async fn test_create_menu_hierarchy() {
    let (menu_service, _) = create_menu_service();
    let app_service = MenuAppService::new(menu_service);

    // Create parent menu (directory)
    let parent_cmd = CreateMenuCommand {
        name: "系统管理".to_string(),
        permission: "system".to_string(),
        types: 0,
        sort: 1,
        parent_id: 0,
        path: Some("/system".to_string()),
        icon: Some("setting".to_string()),
        component: None,
        component_name: Some("System".to_string()),
    };

    let parent = app_service.create_menu(parent_cmd, Some("admin".to_string())).await.unwrap();

    // Create child menu (page)
    let child_cmd = CreateMenuCommand {
        name: "用户管理".to_string(),
        permission: "system:user:list".to_string(),
        types: 1,
        sort: 1,
        parent_id: parent.id,
        path: Some("/system/user".to_string()),
        icon: Some("user".to_string()),
        component: Some("system/user/index".to_string()),
        component_name: Some("UserManagement".to_string()),
    };

    let child = app_service.create_menu(child_cmd, Some("admin".to_string())).await.unwrap();
    assert_eq!(child.parent_id, parent.id);
}

#[tokio::test]
async fn test_get_menu_tree() {
    let (menu_service, _) = create_menu_service();
    let app_service = MenuAppService::new(menu_service);

    // Create menu hierarchy
    let parent_cmd = CreateMenuCommand {
        name: "系统管理".to_string(),
        permission: "system".to_string(),
        types: 0,
        sort: 1,
        parent_id: 0,
        path: Some("/system".to_string()),
        icon: Some("setting".to_string()),
        component: None,
        component_name: Some("System".to_string()),
    };

    let parent = app_service.create_menu(parent_cmd, Some("admin".to_string())).await.unwrap();

    let child1_cmd = CreateMenuCommand {
        name: "用户管理".to_string(),
        permission: "system:user:list".to_string(),
        types: 1,
        sort: 1,
        parent_id: parent.id,
        path: Some("/system/user".to_string()),
        icon: Some("user".to_string()),
        component: Some("system/user/index".to_string()),
        component_name: Some("UserManagement".to_string()),
    };

    let child2_cmd = CreateMenuCommand {
        name: "角色管理".to_string(),
        permission: "system:role:list".to_string(),
        types: 1,
        sort: 2,
        parent_id: parent.id,
        path: Some("/system/role".to_string()),
        icon: Some("peoples".to_string()),
        component: Some("system/role/index".to_string()),
        component_name: Some("RoleManagement".to_string()),
    };

    app_service.create_menu(child1_cmd, Some("admin".to_string())).await.unwrap();
    app_service.create_menu(child2_cmd, Some("admin".to_string())).await.unwrap();

    // Get menu tree
    let query = MenuQueryRequest {
        name: None,
        status: None,
        types: None,
    };

    let tree = app_service.get_menu_tree(query).await.unwrap();
    assert_eq!(tree.len(), 1); // One root menu
    assert_eq!(tree[0].children.len(), 2); // Two child menus
}

// ============================================================================
// Department Tests
// ============================================================================

#[tokio::test]
async fn test_create_department() {
    let (dept_service, _) = create_dept_service();
    let app_service = DepartmentAppService::new(dept_service);

    let cmd = CreateDeptCommand {
        name: "技术部".to_string(),
        parent_id: 0,
        sort: 1,
        leader_user_id: None,
        phone: None,
        email: None,
    };

    let result = app_service.create_dept(cmd, Some("admin".to_string())).await;
    assert!(result.is_ok());

    let dept = result.unwrap();
    assert_eq!(dept.name, "技术部");
    assert_eq!(dept.parent_id, 0);
}

#[tokio::test]
async fn test_create_dept_hierarchy() {
    let (dept_service, _) = create_dept_service();
    let app_service = DepartmentAppService::new(dept_service);

    // Create parent department
    let parent_cmd = CreateDeptCommand {
        name: "总公司".to_string(),
        parent_id: 0,
        sort: 1,
        leader_user_id: None,
        phone: None,
        email: None,
    };

    let parent = app_service.create_dept(parent_cmd, Some("admin".to_string())).await.unwrap();

    // Create child department
    let child_cmd = CreateDeptCommand {
        name: "技术部".to_string(),
        parent_id: parent.id,
        sort: 1,
        leader_user_id: None,
        phone: None,
        email: None,
    };

    let child = app_service.create_dept(child_cmd, Some("admin".to_string())).await.unwrap();
    assert_eq!(child.parent_id, parent.id);
}

#[tokio::test]
async fn test_get_dept_tree() {
    let (dept_service, _) = create_dept_service();
    let app_service = DepartmentAppService::new(dept_service);

    // Create hierarchy
    let root_cmd = CreateDeptCommand {
        name: "总公司".to_string(),
        parent_id: 0,
        sort: 1,
        leader_user_id: None,
        phone: None,
        email: None,
    };

    let root = app_service.create_dept(root_cmd, Some("admin".to_string())).await.unwrap();

    let child1_cmd = CreateDeptCommand {
        name: "技术部".to_string(),
        parent_id: root.id,
        sort: 1,
        leader_user_id: None,
        phone: None,
        email: None,
    };

    let child2_cmd = CreateDeptCommand {
        name: "产品部".to_string(),
        parent_id: root.id,
        sort: 2,
        leader_user_id: None,
        phone: None,
        email: None,
    };

    app_service.create_dept(child1_cmd, Some("admin".to_string())).await.unwrap();
    app_service.create_dept(child2_cmd, Some("admin".to_string())).await.unwrap();

    // Get tree
    let query = DeptQueryRequest {
        name: None,
        status: None,
    };

    let tree = app_service.get_dept_tree(query).await.unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].children.len(), 2);
}

// ============================================================================
// Config Tests
// ============================================================================

#[tokio::test]
async fn test_create_config() {
    let (config_service, _) = create_config_service();
    let app_service = ConfigAppService::new(config_service);

    let cmd = CreateConfigCommand {
        category: "system".to_string(),
        config_type: 0,
        name: "系统名称".to_string(),
        config_key: "sys.name".to_string(),
        value: "Admin System".to_string(),
        remark: None,
    };

    let result = app_service.create_config(cmd, Some("admin".to_string())).await;
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.name, "系统名称");
    assert_eq!(config.config_key, "sys.name");
    assert_eq!(config.value, "Admin System");
}

#[tokio::test]
async fn test_create_duplicate_config_key() {
    let (config_service, _) = create_config_service();
    let app_service = ConfigAppService::new(config_service);

    let cmd1 = CreateConfigCommand {
        category: "system".to_string(),
        config_type: 0,
        name: "系统名称".to_string(),
        config_key: "sys.name".to_string(),
        value: "Admin System".to_string(),
        remark: None,
    };

    let cmd2 = CreateConfigCommand {
        category: "system".to_string(),
        config_type: 0,
        name: "系统标题".to_string(),
        config_key: "sys.name".to_string(),
        value: "My Admin".to_string(),
        remark: None,
    };

    // First creation should succeed
    let result1 = app_service.create_config(cmd1, Some("admin".to_string())).await;
    assert!(result1.is_ok());

    // Second creation with same key should fail
    let result2 = app_service.create_config(cmd2, Some("admin".to_string())).await;
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_get_config_by_key() {
    let (config_service, _) = create_config_service();
    let app_service = ConfigAppService::new(config_service);

    let cmd = CreateConfigCommand {
        category: "system".to_string(),
        config_type: 0,
        name: "系统名称".to_string(),
        config_key: "sys.name".to_string(),
        value: "Admin System".to_string(),
        remark: None,
    };

    app_service.create_config(cmd, Some("admin".to_string())).await.unwrap();

    let result = app_service.get_by_key("sys.name").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().value, "Admin System");
}

// ============================================================================
// Dictionary Tests
// ============================================================================

#[tokio::test]
async fn test_create_dict_type() {
    let (dict_type_service, _) = create_dict_type_service();
    let app_service = DictTypeAppService::new(dict_type_service);

    let cmd = CreateDictTypeCommand {
        name: "用户性别".to_string(),
        dict_type: "sys_user_sex".to_string(),
        remark: Some("用户性别字典".to_string()),
    };

    let result = app_service.create_dict_type(cmd, Some("admin".to_string())).await;
    assert!(result.is_ok());

    let dt = result.unwrap();
    assert_eq!(dt.name, "用户性别");
    assert_eq!(dt.dict_type, "sys_user_sex");
}

#[tokio::test]
async fn test_create_dict_data() {
    let (dict_data_service, _) = create_dict_data_service();
    let app_service = DictDataAppService::new(dict_data_service);

    let cmd = CreateDictDataCommand {
        sort: 1,
        label: "男".to_string(),
        value: "0".to_string(),
        dict_type: "sys_user_sex".to_string(),
        color_type: None,
        css_class: None,
        remark: None,
    };

    let result = app_service.create_dict_data(cmd, Some("admin".to_string())).await;
    assert!(result.is_ok());

    let dd = result.unwrap();
    assert_eq!(dd.label, "男");
    assert_eq!(dd.value, "0");
    assert_eq!(dd.dict_type, "sys_user_sex");
}

#[tokio::test]
async fn test_get_dict_data_by_type() {
    let (dict_data_service, _) = create_dict_data_service();
    let app_service = DictDataAppService::new(dict_data_service);

    // Create multiple dict data entries
    for (i, label) in ["男", "女", "未知"].iter().enumerate() {
        let cmd = CreateDictDataCommand {
            sort: i as i32,
            label: label.to_string(),
            value: i.to_string(),
            dict_type: "sys_user_sex".to_string(),
            color_type: None,
            css_class: None,
            remark: None,
        };
        app_service.create_dict_data(cmd, Some("admin".to_string())).await.unwrap();
    }

    let result = app_service.get_by_dict_type("sys_user_sex").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 3);
}

// ============================================================================
// Permission Tests
// ============================================================================

#[tokio::test]
async fn test_get_all_permissions() {
    let permission_repo = Arc::new(permission_repo::MockPermissionRepository::new());
    let permission_service = Arc::new(PermissionService::new(permission_repo));
    let app_service = PermissionAppService::new(permission_service);

    let result = app_service.get_all_permissions().await;
    assert!(result.is_ok());

    let permissions = result.unwrap();
    assert!(!permissions.is_empty());
    assert!(permissions.iter().any(|p| p.code == "system:user:list"));
    assert!(permissions.iter().any(|p| p.code == "system:role:create"));
}

#[tokio::test]
async fn test_check_permission() {
    let permission_repo = Arc::new(
        permission_repo::MockPermissionRepository::new()
            .with_user_roles(1, vec![1])
            .with_role_permissions(1, HashSet::from(["system:user:list".to_string(), "system:user:create".to_string()]))
    );
    let permission_service = Arc::new(PermissionService::new(permission_repo));
    let app_service = PermissionAppService::new(permission_service);

    // Check existing permission
    let check1 = PermissionCheckRequest {
        user_id: 1,
        permission: "system:user:list".to_string(),
    };
    let result1 = app_service.check_permission(check1).await.unwrap();
    assert!(result1.has_permission);

    // Check non-existing permission
    let check2 = PermissionCheckRequest {
        user_id: 1,
        permission: "system:user:delete".to_string(),
    };
    let result2 = app_service.check_permission(check2).await.unwrap();
    assert!(!result2.has_permission);
}

#[tokio::test]
async fn test_get_user_permissions() {
    let permission_repo = Arc::new(
        permission_repo::MockPermissionRepository::new()
            .with_user_roles(1, vec![1, 2])
            .with_role_permissions(1, HashSet::from(["system:user:list".to_string()]))
            .with_role_permissions(2, HashSet::from(["system:role:list".to_string()]))
    );
    let permission_service = Arc::new(PermissionService::new(permission_repo));
    let app_service = PermissionAppService::new(permission_service);

    let result = app_service.get_user_permissions(1).await.unwrap();
    assert_eq!(result.permissions.len(), 2);
    assert!(result.permissions.contains(&"system:user:list".to_string()));
    assert!(result.permissions.contains(&"system:role:list".to_string()));
}

// ============================================================================
// Auth Tests
// ============================================================================

#[tokio::test]
async fn test_login_success() {
    let user_repo = Arc::new(
        user_repo::MockUserRepository::new()
            .with_user(User::create(1, "admin".to_string(), "password123".to_string(), "管理员".to_string(), None))
    );
    let permission_repo = Arc::new(
        permission_repo::MockPermissionRepository::new()
            .with_user_roles(1, vec![1])
            .with_role_permissions(1, HashSet::from(["*".to_string()]))
    );
    let role_repo = Arc::new(role_repo::MockRoleRepository::new());

    let user_service = Arc::new(UserService::new(user_repo, permission_repo.clone()));
    let role_service = Arc::new(RoleService::new(role_repo));
    let permission_service = Arc::new(PermissionService::new(permission_repo));

    let app_service = AuthAppService::new(user_service, role_service, permission_service);

    let cmd = LoginCommand {
        username: "admin".to_string(),
        password: "password123".to_string(),
        login_ip: "127.0.0.1".to_string(),
    };

    let result = app_service.login(cmd).await;
    assert!(result.is_ok());

    let login_resp = result.unwrap();
    assert_eq!(login_resp.user_id, 1);
    assert_eq!(login_resp.username, "admin");
    assert_eq!(login_resp.nickname, "管理员");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let user_repo = Arc::new(
        user_repo::MockUserRepository::new()
            .with_user(User::create(1, "admin".to_string(), "password123".to_string(), "管理员".to_string(), None))
    );
    let permission_repo = Arc::new(permission_repo::MockPermissionRepository::new());
    let role_repo = Arc::new(role_repo::MockRoleRepository::new());

    let user_service = Arc::new(UserService::new(user_repo, permission_repo.clone()));
    let role_service = Arc::new(RoleService::new(role_repo));
    let permission_service = Arc::new(PermissionService::new(permission_repo));

    let app_service = AuthAppService::new(user_service, role_service, permission_service);

    let cmd = LoginCommand {
        username: "admin".to_string(),
        password: "wrongpassword".to_string(),
        login_ip: "127.0.0.1".to_string(),
    };

    let result = app_service.login(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let user_repo = Arc::new(user_repo::MockUserRepository::new());
    let permission_repo = Arc::new(permission_repo::MockPermissionRepository::new());
    let role_repo = Arc::new(role_repo::MockRoleRepository::new());

    let user_service = Arc::new(UserService::new(user_repo, permission_repo.clone()));
    let role_service = Arc::new(RoleService::new(role_repo));
    let permission_service = Arc::new(PermissionService::new(permission_repo));

    let app_service = AuthAppService::new(user_service, role_service, permission_service);

    let cmd = LoginCommand {
        username: "nonexistent".to_string(),
        password: "password".to_string(),
        login_ip: "127.0.0.1".to_string(),
    };

    let result = app_service.login(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_login_disabled_user() {
    let mut user = User::create(1, "admin".to_string(), "password123".to_string(), "管理员".to_string(), None);
    user.change_status(UserStatus::Disabled, None); // Disable user

    let user_repo = Arc::new(
        user_repo::MockUserRepository::new()
            .with_user(user)
    );
    let permission_repo = Arc::new(permission_repo::MockPermissionRepository::new());
    let role_repo = Arc::new(role_repo::MockRoleRepository::new());

    let user_service = Arc::new(UserService::new(user_repo, permission_repo.clone()));
    let role_service = Arc::new(RoleService::new(role_repo));
    let permission_service = Arc::new(PermissionService::new(permission_repo));

    let app_service = AuthAppService::new(user_service, role_service, permission_service);

    let cmd = LoginCommand {
        username: "admin".to_string(),
        password: "password123".to_string(),
        login_ip: "127.0.0.1".to_string(),
    };

    let result = app_service.login(cmd).await;
    assert!(result.is_err());
}

// ============================================================================
// Integration Test - Full Workflow
// ============================================================================

#[tokio::test]
async fn test_full_workflow() {
    // 1. Create departments
    let (dept_service, _) = create_dept_service();
    let dept_app = DepartmentAppService::new(dept_service);

    let root_dept = dept_app.create_dept(
        CreateDeptCommand {
            name: "总公司".to_string(),
            parent_id: 0,
            sort: 1,
            leader_user_id: None,
            phone: None,
            email: None,
        },
        Some("admin".to_string()),
    ).await.unwrap();

    let tech_dept = dept_app.create_dept(
        CreateDeptCommand {
            name: "技术部".to_string(),
            parent_id: root_dept.id,
            sort: 1,
            leader_user_id: None,
            phone: None,
            email: None,
        },
        Some("admin".to_string()),
    ).await.unwrap();

    // 2. Create roles
    let (role_service, _) = create_role_service();
    let role_app = RoleAppService::new(role_service);

    let admin_role = role_app.create_role(
        CreateRoleCommand {
            name: "管理员".to_string(),
            code: "admin".to_string(),
            sort: 1,
            remark: None,
            menu_ids: None,
        },
        Some("admin".to_string()),
    ).await.unwrap();

    // 3. Create users
    let (user_service, _) = create_user_service();
    let user_app = UserAppService::new(user_service);

    let user = user_app.create_user(
        CreateUserCommand {
            username: "zhangsan".to_string(),
            password: "password123".to_string(),
            nickname: "张三".to_string(),
            email: Some("zhangsan@example.com".to_string()),
            mobile: Some("13800138001".to_string()),
            sex: Some(Sex::Male),
            remark: None,
            role_ids: Some(vec![admin_role.id]),
            dept_ids: Some(vec![tech_dept.id]),
        },
        Some("admin".to_string()),
    ).await.unwrap();

    // 4. Verify user
    let retrieved = user_app.get_user(user.id).await.unwrap();
    assert_eq!(retrieved.username, "zhangsan");
    assert_eq!(retrieved.nickname, "张三");
    // Note: get_user returns user from repository, role_ids are stored separately
    // The create_user response already confirmed role_ids were assigned
    assert_eq!(user.role_ids, vec![admin_role.id]);

    // 5. Create menus
    let (menu_service, _) = create_menu_service();
    let menu_app = MenuAppService::new(menu_service);

    let system_menu = menu_app.create_menu(
        CreateMenuCommand {
            name: "系统管理".to_string(),
            permission: "system".to_string(),
            types: 0,
            sort: 1,
            parent_id: 0,
            path: Some("/system".to_string()),
            icon: Some("setting".to_string()),
            component: None,
            component_name: Some("System".to_string()),
        },
        Some("admin".to_string()),
    ).await.unwrap();

    let user_menu = menu_app.create_menu(
        CreateMenuCommand {
            name: "用户管理".to_string(),
            permission: "system:user:list".to_string(),
            types: 1,
            sort: 1,
            parent_id: system_menu.id,
            path: Some("/system/user".to_string()),
            icon: Some("user".to_string()),
            component: Some("system/user/index".to_string()),
            component_name: Some("UserManagement".to_string()),
        },
        Some("admin".to_string()),
    ).await.unwrap();

    // 6. Assign menus to role
    role_app.assign_menus(AssignMenusCommand {
        role_id: admin_role.id,
        menu_ids: vec![system_menu.id, user_menu.id],
    }).await.unwrap();

    // 7. Create config
    let (config_service, _) = create_config_service();
    let config_app = ConfigAppService::new(config_service);

    config_app.create_config(
        CreateConfigCommand {
            category: "system".to_string(),
            config_type: 0,
            name: "系统名称".to_string(),
            config_key: "sys.name".to_string(),
            value: "Admin System".to_string(),
            remark: None,
        },
        Some("admin".to_string()),
    ).await.unwrap();

    // 8. Create dictionary
    let (dict_type_service, _) = create_dict_type_service();
    let dict_type_app = DictTypeAppService::new(dict_type_service);

    dict_type_app.create_dict_type(
        CreateDictTypeCommand {
            name: "用户性别".to_string(),
            dict_type: "sys_user_sex".to_string(),
            remark: None,
        },
        Some("admin".to_string()),
    ).await.unwrap();

    println!("Full workflow test completed successfully!");
}
