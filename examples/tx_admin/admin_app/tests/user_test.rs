//! 用户管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第1节）:
//!   1.1 用户CRUD         ✅
//!   1.2 用户状态管理     ✅ (启用/禁用/锁定)
//!   1.4 角色分配         ✅
//!   1.5 部门分配         ✅
//!   1.7 用户查询         ✅

mod common;
use admin_proto::{CreateUserRequest, UpdateUserRequest, ListUsersRequest, PageRequest};
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::user::model::value_object::{Sex, UserStatus};
use admin_domain::user::repository::UserRepository;

// ── 1.1 用户 CRUD ──────────────────────────────────────────────────────────

/// 创建用户测试规范（六大维度）
///
/// ## 验证维度
/// | # | 维度           | 验证方式                  | 覆盖内容                              |
/// |---|---------------|--------------------------|--------------------------------------|
/// | 1 | 返回值正确性    | 对返回值逐字段 assert      | 所有显式赋值字段 + 默认值字段           |
/// | 2 | 持久化正确性    | API 回查 get_user        | 与返回值一致的字段再次验证              |
/// | 3 | 默认值         | 断言默认/自动生成值        | ID>0、status=Active、空列表、None 字段  |
/// | 4 | 审计追踪       | 仓库直接查询 raw User     | creator/updater/create_time/deleted   |
/// | 5 | 业务规则       | 独立错误用例              | 用户名/邮箱/手机重复 → 略（另有用例）    |
/// | 6 | 副作用         | 独立用例                  | 角色绑定/部门绑定 → 略（另有用例）       |
#[tokio::test]
async fn create_user_success() {
    let (app, _, repo) = common::create_user_app().await;

    // ── Act ──────────────────────────────────────────────────────────────
    let user = app
        .create_user(
            CreateUserRequest {
                username: "testuser".into(),
                password: "password123".into(),
                nickname: "测试用户".into(),
                email: Some("test@example.com".into()),
                mobile: Some("13800138000".into()),
                sex: Some(Sex::Male as i32),
                remark: None,
                role_ids: vec![],
                dept_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    // ══════════════════════════════════════════════════════════════════════
    // 维度 1：返回值正确性 — 逐字段验证
    // ══════════════════════════════════════════════════════════════════════
    assert!(user.id > 0, "用户 ID 应由 ID 生成器分配，大于 0");
    assert_eq!(user.username, "testuser");
    assert_eq!(user.nickname, "测试用户");
    assert_eq!(user.email, Some("test@example.com".into()));
    assert_eq!(user.mobile, Some("13800138000".into()));
    assert_eq!(Sex::from(user.sex as i32), Sex::Male);
    assert_eq!(UserStatus::from(user.status), UserStatus::Active, "新用户默认状态应为 Active");
    assert_eq!(user.remark, None, "未提供备注时应为 None");
    assert!(user.role_ids.is_empty(), "未分配角色时 role_ids 应为空 Vec");
    assert!(user.dept_ids.is_empty(), "未分配部门时 dept_ids 应为空 Vec");

    // ══════════════════════════════════════════════════════════════════════
    // 维度 2：持久化正确性 — 通过 API 回查验证数据已落库
    // ══════════════════════════════════════════════════════════════════════
    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(found.id, user.id, "回查 ID 应与创建时一致");
    assert_eq!(found.username, "testuser");
    assert_eq!(found.nickname, "测试用户");
    assert_eq!(found.email, Some("test@example.com".into()));
    assert_eq!(found.mobile, Some("13800138000".into()));
    assert_eq!(Sex::from(found.sex as i32), Sex::Male);
    assert_eq!(UserStatus::from(found.status), UserStatus::Active);
    assert_eq!(found.remark, None);
    assert!(found.role_ids.is_empty());
    assert!(found.dept_ids.is_empty());

    // ══════════════════════════════════════════════════════════════════════
    // 维度 4：审计追踪 — 通过仓库直接查询，验证 API 不暴露的内部字段
    // ══════════════════════════════════════════════════════════════════════
    let raw = repo
        .find_by_id(user.id)
        .await
        .unwrap()
        .expect("用户必须在仓库中存在");

    // 审计：创建者和更新者
    assert_eq!(
        raw.audit.creator,
        Some("admin".into()),
        "创建者应记录为传入的 creator"
    );
    assert_eq!(
        raw.audit.updater,
        Some("admin".into()),
        "更新者应与创建者一致（创建即是首次更新）"
    );

    // 审计：时间戳已设置（不超过当前时间，且两个时间相近）
    let now = jiff::Timestamp::now();
    assert!(raw.audit.create_time <= now, "创建时间不应超过当前时间");
    assert!(raw.audit.update_time <= now, "更新时间不应超过当前时间");
    assert!(
        raw.audit.update_time >= raw.audit.create_time,
        "更新时间不应早于创建时间"
    );

    // 审计：未软删除
    assert_eq!(
        raw.audit.deleted,
        DeletedStatus::Normal,
        "新用户不应处于软删除状态"
    );

    // 密码：确认密码已哈希存储（argon2）
    assert!(raw.password.starts_with("$argon2"), "密码应以 argon2 哈希存储");
}

#[tokio::test]
async fn create_user_with_roles_and_depts() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "staff".into(),
                password: "pwd".into(),
                nickname: "员工".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: vec![1, 2],
                dept_ids: vec![100],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();
    assert_eq!(user.username, "staff");

    // 回查验证角色和部门已持久化
    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(found.role_ids.len(), 2);
    assert!(found.role_ids.contains(&1));
    assert!(found.role_ids.contains(&2));
    assert_eq!(found.dept_ids.len(), 1);
    assert!(found.dept_ids.contains(&100));
}

#[tokio::test]
async fn create_duplicate_username_should_fail() {
    let (app, _, _) = common::create_user_app().await;
    let cmd = |name: &str| CreateUserRequest {
        username: name.into(),
        password: "pwd".into(),
        nickname: "x".into(),
        email: None,
        mobile: None,
        sex: None,
        remark: None,
        role_ids: vec![],
        dept_ids: vec![],
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
    let (app, _, _) = common::create_user_app().await;
    let cmd = |name: &str, email: &str| CreateUserRequest {
        username: name.into(),
        password: "pwd".into(),
        nickname: "x".into(),
        email: Some(email.into()),
        mobile: None,
        sex: None,
        remark: None,
        role_ids: vec![],
        dept_ids: vec![],
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
    let (app, _, _) = common::create_user_app().await;
    let cmd = |name: &str, mobile: &str| CreateUserRequest {
        username: name.into(),
        password: "pwd".into(),
        nickname: "x".into(),
        email: None,
        mobile: Some(mobile.into()),
        sex: None,
        remark: None,
        role_ids: vec![],
        dept_ids: vec![],
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
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "old".into(),
                password: "pwd".into(),
                nickname: "Old".into(),
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

    app.update_user(
        UpdateUserRequest {
            user_id: user.id,
            nickname: Some("NewName".into()),
            email: Some("new@example.com".into()),
            mobile: Some("13800000000".into()),
            sex: Some(Sex::Female as i32),
            status: None,
            remark: Some("已更新".into()),
        },
        Some("admin".into()),
    )
    .await
    .unwrap();

    // 回查验证更新持久化
    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(found.nickname, "NewName");
    assert_eq!(found.email, Some("new@example.com".into()));
    assert_eq!(found.mobile, Some("13800000000".into()));
    assert_eq!(Sex::from(found.sex as i32), Sex::Female);
    assert_eq!(found.remark, Some("已更新".into()));
}

#[tokio::test]
async fn delete_user_soft_delete() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "todelete".into(),
                password: "pwd".into(),
                nickname: "Del".into(),
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

    app.delete_user(user.id, Some("admin".into()))
        .await
        .unwrap();
    assert!(app.get_user(user.id).await.is_err());
}

#[tokio::test]
async fn get_user_detail() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "detail".into(),
                password: "pwd".into(),
                nickname: "详情".into(),
                email: Some("detail@test.com".into()),
                mobile: None,
                sex: Some(Sex::Male as i32),
                remark: Some("备注".into()),
                role_ids: vec![],
                dept_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(found.username, "detail");
    assert_eq!(found.nickname, "详情");
    assert_eq!(found.email, Some("detail@test.com".into()));
    assert_eq!(Sex::from(found.sex as i32), Sex::Male);
}

// ── 1.2 用户状态管理 ───────────────────────────────────────────────────────

#[tokio::test]
async fn change_status_to_disabled() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "disableme".into(),
                password: "pwd".into(),
                nickname: "禁用".into(),
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

    app.change_status(user.id, UserStatus::Disabled, Some("admin".into()))
        .await
        .unwrap();

    // 回查验证状态已持久化
    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(UserStatus::from(found.status), UserStatus::Disabled);
}

#[tokio::test]
async fn change_status_to_active_reenable() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "reenable".into(),
                password: "pwd".into(),
                nickname: "重启用".into(),
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
    app.change_status(user.id, UserStatus::Disabled, Some("admin".into()))
        .await
        .unwrap();
    app.change_status(user.id, UserStatus::Active, Some("admin".into()))
        .await
        .unwrap();

    // 回查验证重新启用已持久化
    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(UserStatus::from(found.status), UserStatus::Active);
}

#[tokio::test]
async fn change_status_to_locked() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "lockme".into(),
                password: "pwd".into(),
                nickname: "锁定".into(),
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
    app.change_status(user.id, UserStatus::Locked, Some("admin".into()))
        .await
        .unwrap();

    // 回查验证锁定已持久化
    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(UserStatus::from(found.status), UserStatus::Locked);
}

// ── 1.4 角色分配 ───────────────────────────────────────────────────────────

#[tokio::test]
async fn assign_roles_to_user() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "multi_role".into(),
                password: "pwd".into(),
                nickname: "多角色".into(),
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

    app.assign_roles(user.id, vec![1, 2, 3]).await.unwrap();

    // 回查验证角色已持久化
    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(found.role_ids.len(), 3);
    assert!(found.role_ids.contains(&1));
    assert!(found.role_ids.contains(&2));
    assert!(found.role_ids.contains(&3));
}

#[tokio::test]
async fn assign_roles_empty_should_clear() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "clear_roles".into(),
                password: "pwd".into(),
                nickname: "清空角色".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: vec![1, 2],
                dept_ids: vec![],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    app.assign_roles(user.id, vec![]).await.unwrap();

    // 回查验证角色已清空
    let found = app.get_user(user.id).await.unwrap();
    assert!(found.role_ids.is_empty());
}

// ── 1.5 部门分配 ───────────────────────────────────────────────────────────

#[tokio::test]
async fn assign_departments_to_user() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "multi_dept".into(),
                password: "pwd".into(),
                nickname: "多部门".into(),
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

    app.assign_departments(user.id, vec![100, 200]).await.unwrap();

    // 回查验证部门已持久化
    let found = app.get_user(user.id).await.unwrap();
    assert_eq!(found.dept_ids.len(), 2);
    assert!(found.dept_ids.contains(&100));
    assert!(found.dept_ids.contains(&200));
}

#[tokio::test]
async fn assign_departments_empty_should_clear() {
    let (app, _, _) = common::create_user_app().await;
    let user = app
        .create_user(
            CreateUserRequest {
                username: "clear_dept".into(),
                password: "pwd".into(),
                nickname: "清空部门".into(),
                email: None,
                mobile: None,
                sex: None,
                remark: None,
                role_ids: vec![],
                dept_ids: vec![100],
            },
            Some("admin".into()),
        )
        .await
        .unwrap();

    app.assign_departments(user.id, vec![]).await.unwrap();

    // 回查验证部门已清空
    let found = app.get_user(user.id).await.unwrap();
    assert!(found.dept_ids.is_empty());
}

// ── 1.7 用户查询（分页 & 筛选）────────────────────────────────────────────

#[tokio::test]
async fn paginate_users() {
    let (app, _, _) = common::create_user_app().await;
    for i in 0..7 {
        app.create_user(
            CreateUserRequest {
                username: format!("u{}", i),
                password: "pwd".into(),
                nickname: format!("U{}", i),
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
    }
    let page = app
        .get_user_page(ListUsersRequest {
            username: None,
            nickname: None,
            mobile: None,
            status: None,
            dept_id: None,
            page_info: Some(PageRequest { page: 1, size: 3 }),
        })
        .await
        .unwrap();
    assert_eq!(page.list.len(), 3);
    assert_eq!(page.total, 7);
}

#[tokio::test]
async fn query_user_by_username_fuzzy() {
    let (app, _, _) = common::create_user_app().await;
    app.create_user(
        CreateUserRequest {
            username: "zhangsan".into(),
            password: "pwd".into(),
            nickname: "张三".into(),
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
    app.create_user(
        CreateUserRequest {
            username: "lisi".into(),
            password: "pwd".into(),
            nickname: "李四".into(),
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

    let page = app
        .get_user_page(ListUsersRequest {
            username: Some("zhang".into()),
            nickname: None,
            mobile: None,
            status: None,
            dept_id: None,
            page_info: Some(PageRequest { page: 1, size: 10 }),
        })
        .await
        .unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].username, "zhangsan");
}

#[tokio::test]
async fn query_user_by_status() {
    let (app, _, _) = common::create_user_app().await;
    let u = app
        .create_user(
            CreateUserRequest {
                username: "active_user".into(),
                password: "pwd".into(),
                nickname: "活跃".into(),
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
    app.change_status(u.id, UserStatus::Disabled, Some("admin".into()))
        .await
        .unwrap();

    let page = app
        .get_user_page(ListUsersRequest {
            username: None,
            nickname: None,
            mobile: None,
            status: Some(UserStatus::Disabled as i32),
            dept_id: None,
            page_info: Some(PageRequest { page: 1, size: 10 }),
        })
        .await
        .unwrap();
    assert!(
        page.list
            .iter()
            .any(|u| u.username == "active_user" && UserStatus::from(u.status) == UserStatus::Disabled)
    );
}

#[tokio::test]
async fn query_user_by_nickname() {
    let (app, _, _) = common::create_user_app().await;
    app.create_user(
        CreateUserRequest {
            username: "nick1".into(),
            password: "pwd".into(),
            nickname: "王小明".into(),
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
    app.create_user(
        CreateUserRequest {
            username: "nick2".into(),
            password: "pwd".into(),
            nickname: "李大刚".into(),
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

    let page = app
        .get_user_page(ListUsersRequest {
            username: None,
            nickname: Some("小明".into()),
            mobile: None,
            status: None,
            dept_id: None,
            page_info: Some(PageRequest { page: 1, size: 10 }),
        })
        .await
        .unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].nickname, "王小明");
}
