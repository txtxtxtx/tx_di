//! Real database integration tests using Toasty + SQLite in-memory.
//!
//! These tests exercise the full stack (AppService -> DomainService -> Repository -> SQLite)
//! for methods not covered by the mock-based tests.

mod common;

use std::sync::Arc;

use admin_proto::{CreateConfigRequest, CreateDictTypeRequest, CreateDictDataRequest};
use admin_proto::{CreateUserRequest, ChangePasswordRequest};
use admin_proto::CreateRoleRequest;
use admin_domain::user::repository::UserRepository;

// ══════════════════════════════════════════════════════════════════════════════
// 1. UserAppService::change_password
// ══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_change_password() {
    let plugin = common::create_db_plugin().await;
    let user_repo = Arc::new(admin_infra::user::repository::ToastyUserRepository::new(plugin.clone()));
    let role_repo = Arc::new(admin_infra::role::repository::ToastyRoleRepository::new(plugin.clone()));
    let dept_repo = Arc::new(admin_infra::department::repository::ToastyDepartmentRepository::new(plugin.clone()));
    let menu_repo = Arc::new(admin_infra::menu::repository::ToastyMenuRepository::new(plugin.clone()));
    let user_svc = Arc::new(admin_domain::user::service::UserService::new(user_repo.clone()));
    let app = admin_app::user::app_service::UserAppService::new(user_svc.clone(), role_repo, dept_repo, menu_repo);

    // Create a user with an initial password
    let user = app
        .create_user(
            CreateUserRequest {
                username: "pwd_user".into(),
                password: "old_password".into(),
                nickname: "Pwd User".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: vec![],
                dept_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    // Verify the initial password is stored
    let raw = user_repo.find_by_id(user.id).await.unwrap().unwrap();
    assert_ne!(raw.password, "old_password", "Password should be hashed");
    let initial_hash = raw.password.clone();

    // Change the password
    app.change_password(
        ChangePasswordRequest {
            user_id: user.id,
            new_password: "new_password".into(),
        },
        Some("admin".into()),
    )
    .await
    .unwrap();

    // Verify the password hash has changed
    let raw = user_repo.find_by_id(user.id).await.unwrap().unwrap();
    assert_ne!(raw.password, initial_hash, "Password hash should have changed after change_password");
    assert_ne!(raw.password, "new_password", "New password should also be hashed");
}

// ══════════════════════════════════════════════════════════════════════════════
// 2. RoleAppService::change_status
// ══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_role_change_status() {
    let (app, _, _) = common::create_role_app().await;

    let role = app
        .create_role(
            CreateRoleRequest {
                name: "Test Role".into(),
                code: "test_role".into(),
                sort: 0,
                remark: None,
                menu_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    assert_eq!(role.status, 0, "New role should have status 0");

    // Change status to 1 (disabled)
    let updated = app
        .change_status(role.id, 1, Some("admin".into()))
        .await
        .unwrap();
    assert_eq!(updated.status, 1, "Role status should be 1 after change");

    // Change status back to 0 (active)
    let updated = app
        .change_status(role.id, 0, Some("admin".into()))
        .await
        .unwrap();
    assert_eq!(updated.status, 0, "Role status should be 0 after re-enabling");
}

// ══════════════════════════════════════════════════════════════════════════════
// 3. RoleAppService::get_all_roles
// ══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_get_all_roles() {
    let (app, _, _) = common::create_role_app().await;

    // Create multiple roles
    for i in 0..3 {
        app.create_role(
            CreateRoleRequest {
                name: format!("Role {}", i),
                code: format!("role_{}", i),
                sort: i,
                remark: None,
                menu_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();
    }

    let all_roles = app.get_all_roles().await.unwrap();
    assert_eq!(all_roles.len(), 3, "Should return all 3 roles");
    assert!(all_roles.iter().any(|r| r.code == "role_0"));
    assert!(all_roles.iter().any(|r| r.code == "role_1"));
    assert!(all_roles.iter().any(|r| r.code == "role_2"));
}

// ══════════════════════════════════════════════════════════════════════════════
// 4. RoleAppService::get_role_users
// 5. RoleAppService::add_users_to_role
// ══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_add_users_to_role_and_get_role_users() {
    // Both role and user services share the same database
    let plugin = common::create_db_plugin().await;

    let user_repo = Arc::new(admin_infra::user::repository::ToastyUserRepository::new(plugin.clone()));
    let role_repo = Arc::new(admin_infra::role::repository::ToastyRoleRepository::new(plugin.clone()));
    let dept_repo = Arc::new(admin_infra::department::repository::ToastyDepartmentRepository::new(plugin.clone()));
    let menu_repo = Arc::new(admin_infra::menu::repository::ToastyMenuRepository::new(plugin.clone()));
    let user_svc = Arc::new(admin_domain::user::service::UserService::new(user_repo.clone()));
    let role_svc = Arc::new(admin_domain::role::service::RoleService::new(role_repo.clone()));
    let user_app = admin_app::user::app_service::UserAppService::new(user_svc.clone(), role_repo.clone(), dept_repo, menu_repo);
    let role_app = admin_app::role::app_service::RoleAppService::new(role_svc.clone(), user_repo);

    // Create a role
    let role = role_app
        .create_role(
            CreateRoleRequest {
                name: "Shared Role".into(),
                code: "shared_role".into(),
                sort: 0,
                remark: None,
                menu_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    // Create users
    let user1 = user_app
        .create_user(
            CreateUserRequest {
                username: "user_a".into(),
                password: "pwd".into(),
                nickname: "User A".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: vec![],
                dept_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    let user2 = user_app
        .create_user(
            CreateUserRequest {
                username: "user_b".into(),
                password: "pwd".into(),
                nickname: "User B".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: vec![],
                dept_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    // Add users to role
    role_app
        .add_users_to_role(role.id, vec![user1.id, user2.id])
        .await
        .unwrap();

    // Get role users
    let role_users = role_app.get_role_users(role.id).await.unwrap();
    assert_eq!(role_users.len(), 2, "Role should have 2 users");
    assert!(role_users.iter().any(|u| u.username == "user_a"));
    assert!(role_users.iter().any(|u| u.username == "user_b"));
}

// ══════════════════════════════════════════════════════════════════════════════
// 6. RoleAppService::remove_users_from_role
// ══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_remove_users_from_role() {
    let plugin = common::create_db_plugin().await;

    let user_repo = Arc::new(admin_infra::user::repository::ToastyUserRepository::new(plugin.clone()));
    let role_repo = Arc::new(admin_infra::role::repository::ToastyRoleRepository::new(plugin.clone()));
    let dept_repo = Arc::new(admin_infra::department::repository::ToastyDepartmentRepository::new(plugin.clone()));
    let menu_repo = Arc::new(admin_infra::menu::repository::ToastyMenuRepository::new(plugin.clone()));
    let user_svc = Arc::new(admin_domain::user::service::UserService::new(user_repo.clone()));
    let role_svc = Arc::new(admin_domain::role::service::RoleService::new(role_repo.clone()));
    let user_app = admin_app::user::app_service::UserAppService::new(user_svc.clone(), role_repo.clone(), dept_repo, menu_repo);
    let role_app = admin_app::role::app_service::RoleAppService::new(role_svc.clone(), user_repo);

    // Create role and users
    let role = role_app
        .create_role(
            CreateRoleRequest {
                name: "Remove Test Role".into(),
                code: "remove_role".into(),
                sort: 0,
                remark: None,
                menu_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    let user1 = user_app
        .create_user(
            CreateUserRequest {
                username: "rem_user1".into(),
                password: "pwd".into(),
                nickname: "Rem1".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: vec![],
                dept_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    let user2 = user_app
        .create_user(
            CreateUserRequest {
                username: "rem_user2".into(),
                password: "pwd".into(),
                nickname: "Rem2".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: vec![],
                dept_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    // Add both users
    role_app
        .add_users_to_role(role.id, vec![user1.id, user2.id])
        .await
        .unwrap();

    let users = role_app.get_role_users(role.id).await.unwrap();
    assert_eq!(users.len(), 2);

    // Remove one user
    role_app
        .remove_users_from_role(role.id, vec![user1.id])
        .await
        .unwrap();

    let users = role_app.get_role_users(role.id).await.unwrap();
    assert_eq!(users.len(), 1, "Should have 1 user after removal");
    assert_eq!(users[0].username, "rem_user2");
}


// ══════════════════════════════════════════════════════════════════════════════
// 12. ConfigAppService::get_by_keys
// ══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_config_get_by_keys() {
    let (app, _, _) = common::create_config_app().await;

    // Create configs
    app.create_config(
        CreateConfigRequest {
            category: "sys".into(),
            config_type: 0,
            name: "Site Name".into(),
            config_key: "sys.site.name".into(),
            value: "My Site".into(),
            remark: None,
        },
        Some("admin".into()),
    )
    .await
    .unwrap();

    app.create_config(
        CreateConfigRequest {
            category: "sys".into(),
            config_type: 0,
            name: "Site URL".into(),
            config_key: "sys.site.url".into(),
            value: "https://example.com".into(),
            remark: None,
        },
        Some("admin".into()),
    )
    .await
    .unwrap();

    app.create_config(
        CreateConfigRequest {
            category: "mail".into(),
            config_type: 1,
            name: "SMTP Host".into(),
            config_key: "mail.smtp.host".into(),
            value: "smtp.example.com".into(),
            remark: None,
        },
        Some("admin".into()),
    )
    .await
    .unwrap();

    // Batch query by keys
    let result = app
        .get_by_keys(vec![
            "sys.site.name".into(),
            "mail.smtp.host".into(),
            "nonexistent.key".into(),
        ])
        .await
        .unwrap();

    assert_eq!(result.len(), 2, "Should return 2 matching configs");
    assert_eq!(result.get("sys.site.name").unwrap(), "My Site");
    assert_eq!(result.get("mail.smtp.host").unwrap(), "smtp.example.com");
    assert!(!result.contains_key("nonexistent.key"), "Non-existent key should not be in result");
}

// ══════════════════════════════════════════════════════════════════════════════
// 13. DictDataAppService::get_by_dict_types
// ══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_dict_data_get_by_dict_types() {
    let plugin = common::create_db_plugin().await;

    // Create dict type repo and service for creating dict types
    let dict_type_repo = Arc::new(admin_infra::dictionary::repository::ToastyDictTypeRepository::new(plugin.clone()));
    let dict_type_svc = Arc::new(admin_domain::dictionary::service::DictTypeService::new(dict_type_repo));
    let dict_type_app = admin_app::dictionary::app_service::DictTypeAppService::new(dict_type_svc);

    let dict_data_repo = Arc::new(admin_infra::dictionary::repository::ToastyDictDataRepository::new(plugin));
    let dict_data_svc = Arc::new(admin_domain::dictionary::service::DictDataService::new(dict_data_repo));
    let dict_data_app = admin_app::dictionary::app_service::DictDataAppService::new(dict_data_svc);

    // Create dict types
    dict_type_app
        .create_dict_type(
            CreateDictTypeRequest {
                name: "Gender".into(),
                dict_type: "sys_gender".into(),
                remark: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    dict_type_app
        .create_dict_type(
            CreateDictTypeRequest {
                name: "Status".into(),
                dict_type: "sys_status".into(),
                remark: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    // Create dict data entries
    dict_data_app
        .create_dict_data(
            CreateDictDataRequest {
                sort: 1,
                label: "Male".into(),
                value: "0".into(),
                dict_type: "sys_gender".into(),
                color_type: None,
                css_class: None,
                remark: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    dict_data_app
        .create_dict_data(
            CreateDictDataRequest {
                sort: 2,
                label: "Female".into(),
                value: "1".into(),
                dict_type: "sys_gender".into(),
                color_type: None,
                css_class: None,
                remark: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    dict_data_app
        .create_dict_data(
            CreateDictDataRequest {
                sort: 1,
                label: "Active".into(),
                value: "0".into(),
                dict_type: "sys_status".into(),
                color_type: None,
                css_class: None,
                remark: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    // Batch query by dict types
    let result = dict_data_app
        .get_by_dict_types(vec!["sys_gender".into(), "sys_status".into()])
        .await
        .unwrap();

    assert_eq!(result.len(), 2, "Should return data for 2 dict types");
    assert_eq!(result.get("sys_gender").unwrap().len(), 2, "sys_gender should have 2 entries");
    assert_eq!(result.get("sys_status").unwrap().len(), 1, "sys_status should have 1 entry");

    // Verify labels
    let gender_data = result.get("sys_gender").unwrap();
    assert!(gender_data.iter().any(|d| d.label == "Male"));
    assert!(gender_data.iter().any(|d| d.label == "Female"));

    // Query with a non-existent type should return empty
    let result = dict_data_app
        .get_by_dict_types(vec!["nonexistent_type".into()])
        .await
        .unwrap();
    assert!(result.is_empty(), "Non-existent dict type should return empty map");
}

// ══════════════════════════════════════════════════════════════════════════════
// 14. FileAppService::download_file_stream
// ══════════════════════════════════════════════════════════════════════════════

use std::io::Cursor;

#[tokio::test]
async fn test_download_file_stream() {
    let (app, _, _, _temp) = common::create_file_app().await;

    // Upload a PDF file using stream API
    let data = b"This is a test PDF content.";
    let mut cursor = Cursor::new(data);
    let file = app
        .upload_file_stream(
            "report.pdf".into(),
            "application/pdf".into(),
            &mut cursor,
            None,
            Some("admin".into()),
        )
        .await
        .unwrap();

    // Download via stream API
    let download = app.download_file_stream(file.id).await.unwrap();
    assert_eq!(download.filename, "report.pdf");
    assert_eq!(download.size, data.len() as u64);
    assert_eq!(download.content_type, "application/pdf", "PDF files should have application/pdf content type");

    // Upload a PNG file and verify content type
    let png_data = b"This is a test PNG content.";
    let mut cursor = Cursor::new(png_data);
    let png_file = app
        .upload_file_stream(
            "image.png".into(),
            "image/png".into(),
            &mut cursor,
            None,
            Some("admin".into()),
        )
        .await
        .unwrap();

    let download = app.download_file_stream(png_file.id).await.unwrap();
    assert_eq!(download.content_type, "image/png", "PNG files should have image/png content type");

    // Non-existent file should fail
    let result = app.download_file_stream(999999).await;
    assert!(result.is_err(), "Non-existent file download should return error");
}
