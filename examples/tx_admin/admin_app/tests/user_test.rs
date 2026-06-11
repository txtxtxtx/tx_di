//! 用户管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第1节）:
//!   1.1 用户CRUD         ✅
//!   1.2 用户状态管理     ✅ (启用/禁用/锁定)
//!   1.4 角色分配         ✅
//!   1.5 部门分配         ✅
//!   1.7 用户查询         ✅

mod common;
use admin_app::user::dto::*;
use admin_domain::user::model::value_object::{Sex, UserStatus};

// ── 1.1 用户 CRUD ──────────────────────────────────────────────────────────

#[tokio::test]
async fn create_user_success() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "testuser".into(),
                password: "password123".into(),
                nickname: "测试用户".into(),
                email: Some("test@example.com".into()),
                mobile: Some("13800138000".into()),
                sex: Some(Sex::Male),
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();
    assert_eq!(user.username, "testuser");
    assert_eq!(user.nickname, "测试用户");
    assert_eq!(user.email, Some("test@example.com".into()));
    assert_eq!(user.sex, Sex::Male);
    assert_eq!(user.status, UserStatus::Active);
}

#[tokio::test]
async fn create_user_with_roles_and_depts() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "staff".into(),
                password: "pwd".into(),
                nickname: "员工".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: Some(vec![1, 2]),
                dept_ids: Some(vec![100]),
            },
            Some("admin".into()),
        )
        .await
        .unwrap();
    assert_eq!(user.username, "staff");
    assert!(!user.role_ids.is_empty());
    assert!(!user.dept_ids.is_empty());
}

#[tokio::test]
async fn create_duplicate_username_should_fail() {
    let (app, _, _) = common::create_user_app();
    let cmd = |name: &str| CreateUserCommand {
        username: name.into(),
        password: "pwd".into(),
        nickname: "x".into(),
        email: None,
        mobile: None,
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };
    app.create_user(cmd("dup"), Some("admin".into()))
        .await
        .unwrap();
    assert!(
        app.create_user(cmd("dup"), Some("admin".into()))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn create_duplicate_email_should_fail() {
    let (app, _, _) = common::create_user_app();
    let cmd = |name: &str, email: &str| CreateUserCommand {
        username: name.into(),
        password: "pwd".into(),
        nickname: "x".into(),
        email: Some(email.into()),
        mobile: None,
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };
    app.create_user(cmd("u1", "dup@test.com"), Some("admin".into()))
        .await
        .unwrap();
    assert!(
        app.create_user(cmd("u2", "dup@test.com"), Some("admin".into()))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn create_duplicate_mobile_should_fail() {
    let (app, _, _) = common::create_user_app();
    let cmd = |name: &str, mobile: &str| CreateUserCommand {
        username: name.into(),
        password: "pwd".into(),
        nickname: "x".into(),
        email: None,
        mobile: Some(mobile.into()),
        sex: None,
        remark: None,
        role_ids: None,
        dept_ids: None,
    };
    app.create_user(cmd("u1", "13900000001"), Some("admin".into()))
        .await
        .unwrap();
    assert!(
        app.create_user(cmd("u2", "13900000001"), Some("admin".into()))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn update_user_success() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "old".into(),
                password: "pwd".into(),
                nickname: "Old".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    let updated = app
        .update_user(
            UpdateUserCommand {
                user_id: user.id,
                nickname: "NewName".into(),
                email: Some("new@example.com".into()),
                mobile: Some("13800000000".into()),
                sex: Sex::Female,
                remark: Some("已更新".into()),
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    assert_eq!(updated.nickname, "NewName");
    assert_eq!(updated.email, Some("new@example.com".into()));
    assert_eq!(updated.sex, Sex::Female);
}

#[tokio::test]
async fn delete_user_soft_delete() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "todelete".into(),
                password: "pwd".into(),
                nickname: "Del".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    app.delete_user(user.id, Some("admin".into()))
        .await
        .unwrap();
    assert!(app.get_user(user.id).await.is_err());
}

#[tokio::test]
async fn get_user_detail() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "detail".into(),
                password: "pwd".into(),
                nickname: "详情".into(),
                email: Some("detail@test.com".into()),
                mobile: None,
                sex: Some(Sex::Male),
                remark: Some("备注".into()),
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(found.username, "detail");
    assert_eq!(found.nickname, "详情");
    assert_eq!(found.email, Some("detail@test.com".into()));
    assert_eq!(found.sex, Sex::Male);
}

// ── 1.2 用户状态管理 ───────────────────────────────────────────────────────

#[tokio::test]
async fn change_status_to_disabled() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "disableme".into(),
                password: "pwd".into(),
                nickname: "禁用".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();
    assert_eq!(user.status, UserStatus::Active);

    let u = app
        .change_status(user.id, UserStatus::Disabled, Some("admin".into()))
        .await
        .unwrap();
    assert_eq!(u.status, UserStatus::Disabled);
}

#[tokio::test]
async fn change_status_to_active_reenable() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "reenable".into(),
                password: "pwd".into(),
                nickname: "重启用".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();
    app.change_status(user.id, UserStatus::Disabled, Some("admin".into()))
        .await
        .unwrap();
    let u = app
        .change_status(user.id, UserStatus::Active, Some("admin".into()))
        .await
        .unwrap();
    assert_eq!(u.status, UserStatus::Active);
}

#[tokio::test]
async fn change_status_to_locked() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "lockme".into(),
                password: "pwd".into(),
                nickname: "锁定".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();
    let u = app
        .change_status(user.id, UserStatus::Locked, Some("admin".into()))
        .await
        .unwrap();
    assert_eq!(u.status, UserStatus::Locked);
}

// ── 1.4 角色分配 ───────────────────────────────────────────────────────────

#[tokio::test]
async fn assign_roles_to_user() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "multi_role".into(),
                password: "pwd".into(),
                nickname: "多角色".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    app.assign_roles(AssignRolesCommand {
        user_id: user.id,
        role_ids: vec![1, 2, 3],
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn assign_roles_empty_should_clear() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "clear_roles".into(),
                password: "pwd".into(),
                nickname: "清空角色".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: Some(vec![1, 2]),
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    app.assign_roles(AssignRolesCommand {
        user_id: user.id,
        role_ids: vec![],
    })
    .await
    .unwrap();
}

// ── 1.5 部门分配 ───────────────────────────────────────────────────────────

#[tokio::test]
async fn assign_departments_to_user() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "multi_dept".into(),
                password: "pwd".into(),
                nickname: "多部门".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    app.assign_departments(AssignDeptsCommand {
        user_id: user.id,
        dept_ids: vec![100, 200],
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn assign_departments_empty_should_clear() {
    let (app, _, _) = common::create_user_app();
    let user = app
        .create_user(
            CreateUserCommand {
                username: "clear_dept".into(),
                password: "pwd".into(),
                nickname: "清空部门".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: Some(vec![100]),
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    app.assign_departments(AssignDeptsCommand {
        user_id: user.id,
        dept_ids: vec![],
    })
    .await
    .unwrap();
}

// ── 1.7 用户查询（分页 & 筛选）────────────────────────────────────────────

#[tokio::test]
async fn paginate_users() {
    let (app, _, _) = common::create_user_app();
    for i in 0..7 {
        app.create_user(
            CreateUserCommand {
                username: format!("u{}", i),
                password: "pwd".into(),
                nickname: format!("U{}", i),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();
    }
    let page = app
        .get_user_page(UserQueryRequest {
            username: None,
            nickname: None,
            mobile: None,
            status: None,
            dept_id: None,
            page: 1,
            page_size: 3,
        })
        .await
        .unwrap();
    assert_eq!(page.list.len(), 3);
    assert_eq!(page.total, 7);
}

#[tokio::test]
async fn query_user_by_username_fuzzy() {
    let (app, _, _) = common::create_user_app();
    app.create_user(
        CreateUserCommand {
            username: "zhangsan".into(),
            password: "pwd".into(),
            nickname: "张三".into(),
            email: None,
            mobile: None,
            sex: None,
            remark: None,
            role_ids: None,
            dept_ids: None,
        },
        Some("admin".into()),
    )
    .await
    .unwrap();
    app.create_user(
        CreateUserCommand {
            username: "lisi".into(),
            password: "pwd".into(),
            nickname: "李四".into(),
            email: None,
            mobile: None,
            sex: None,
            remark: None,
            role_ids: None,
            dept_ids: None,
        },
        Some("admin".into()),
    )
    .await
    .unwrap();

    let page = app
        .get_user_page(UserQueryRequest {
            username: Some("zhang".into()),
            nickname: None,
            mobile: None,
            status: None,
            dept_id: None,
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].username, "zhangsan");
}

#[tokio::test]
async fn query_user_by_status() {
    let (app, _, _) = common::create_user_app();
    let u = app
        .create_user(
            CreateUserCommand {
                username: "active_user".into(),
                password: "pwd".into(),
                nickname: "活跃".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: None,
                dept_ids: None,
            },
            Some("admin".into()),
        )
        .await
        .unwrap();
    app.change_status(u.id, UserStatus::Disabled, Some("admin".into()))
        .await
        .unwrap();

    let page = app
        .get_user_page(UserQueryRequest {
            username: None,
            nickname: None,
            mobile: None,
            status: Some(UserStatus::Disabled),
            dept_id: None,
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();
    assert!(
        page.list
            .iter()
            .any(|u| u.username == "active_user" && u.status == UserStatus::Disabled)
    );
}

#[tokio::test]
async fn query_user_by_nickname() {
    let (app, _, _) = common::create_user_app();
    app.create_user(
        CreateUserCommand {
            username: "nick1".into(),
            password: "pwd".into(),
            nickname: "王小明".into(),
            email: None,
            mobile: None,
            sex: None,
            remark: None,
            role_ids: None,
            dept_ids: None,
        },
        Some("admin".into()),
    )
    .await
    .unwrap();
    app.create_user(
        CreateUserCommand {
            username: "nick2".into(),
            password: "pwd".into(),
            nickname: "李大刚".into(),
            email: None,
            mobile: None,
            sex: None,
            remark: None,
            role_ids: None,
            dept_ids: None,
        },
        Some("admin".into()),
    )
    .await
    .unwrap();

    let page = app
        .get_user_page(UserQueryRequest {
            username: None,
            nickname: Some("小明".into()),
            mobile: None,
            status: None,
            dept_id: None,
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].nickname, "王小明");
}
