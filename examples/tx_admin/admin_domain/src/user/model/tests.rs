// ============================================================
// UNIT TESTS: User 聚合根 + 值对象 (UserStatus, Sex)
// Coverage: create, set_basic_info, change_status, change_password,
//           record_login, soft_delete, set_roles, set_departments,
//           is_active, is_locked, UserStatus::try_from_i32, Sex::from
// ============================================================

#[cfg(test)]
mod user_tests {
    use crate::shared::model::value_object::{DeletedStatus, TenantId};
    use crate::shared::model::{AggregateRoot, AuditFields};
    use crate::user::model::aggregate::User;
    use crate::user::model::value_object::{Sex, UserStatus};
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    // ---- Helpers / Fixtures --------------------------------
    fn make_user() -> User {
        User::create(
            1,
            "testuser".into(),
            "hashed_password".into(),
            "Test User".into(),
            Some("admin".into()),
        )
    }

    // ============================================================
    // User::create
    // ============================================================

    #[test]
    fn test_create_user_sets_all_fields_correctly() {
        let user = User::create(42, "alice".into(), "pwd".into(), "Alice".into(), Some("creator".into()));

        assert_eq!(user.id, 42);
        assert_eq!(user.username, "alice");
        assert_eq!(user.password, "pwd");
        assert_eq!(user.nickname, "Alice");
        assert_eq!(user.sex, Sex::Unknown);
        assert_eq!(user.status, UserStatus::Active);
        assert!(user.remark.is_none());
        assert!(user.email.is_none());
        assert!(user.mobile.is_none());
        assert!(user.avatar.is_none());
        assert!(user.login_ip.is_none());
        assert!(user.login_date.is_none());
        assert_eq!(user.tenant_id, TenantId::default());
        assert!(user.role_ids.is_empty());
        assert!(user.dept_ids.is_empty());
        assert_eq!(user.audit.creator.as_deref(), Some("creator"));
        assert_eq!(user.audit.updater.as_deref(), Some("creator"));
        assert_eq!(user.audit.deleted, DeletedStatus::Normal);
    }

    #[test]
    fn test_create_user_raises_user_created_event() {
        let user = User::create(1, "bob".into(), "pwd".into(), "Bob".into(), None);

        assert_eq!(user.events().len(), 1);
        match &user.events()[0] {
            DomainEvent::UserCreated { user_id, username } => {
                assert_eq!(*user_id, 1);
                assert_eq!(username, "bob");
            }
            _ => panic!("expected UserCreated event"),
        }
    }

    #[test]
    fn test_create_user_with_none_creator() {
        let user = User::create(1, "bob".into(), "pwd".into(), "Bob".into(), None);

        assert!(user.audit.creator.is_none());
        assert!(user.audit.updater.is_none());
    }

    // ============================================================
    // User::set_basic_info
    // ============================================================

    #[test]
    fn test_set_basic_info_updates_all_fields() {
        let mut user = make_user();

        user.set_basic_info(
            "NewNick".into(),
            Some("new@test.com".into()),
            Some("13800138000".into()),
            Sex::Male,
            Some("remark".into()),
            Some("updater".into()),
        );

        assert_eq!(user.nickname, "NewNick");
        assert_eq!(user.email.as_deref(), Some("new@test.com"));
        assert_eq!(user.mobile.as_deref(), Some("13800138000"));
        assert_eq!(user.sex, Sex::Male);
        assert_eq!(user.remark.as_deref(), Some("remark"));
        assert_eq!(user.audit.updater.as_deref(), Some("updater"));
    }

    #[test]
    fn test_set_basic_info_raises_user_updated_event() {
        let mut user = make_user();
        let before_len = user.events().len();

        user.set_basic_info("Nick".into(), None, None, Sex::Female, None, None);

        assert_eq!(user.events().len(), before_len + 1);
        assert!(matches!(user.events().last(), Some(DomainEvent::UserUpdated { user_id: 1 })));
    }

    #[test]
    fn test_set_basic_info_clears_optional_fields_with_none() {
        let mut user = make_user();
        user.email = Some("old@test.com".into());
        user.mobile = Some("old_phone".into());

        user.set_basic_info("Nick".into(), None, None, Sex::Unknown, None, None);

        assert!(user.email.is_none());
        assert!(user.mobile.is_none());
    }

    // ============================================================
    // User::change_status
    // ============================================================

    #[test]
    fn test_change_status_to_disabled() {
        let mut user = make_user();

        user.change_status(UserStatus::Disabled, Some("admin".into()));

        assert_eq!(user.status, UserStatus::Disabled);
        assert_eq!(user.audit.updater.as_deref(), Some("admin"));
    }

    #[test]
    fn test_change_status_to_locked() {
        let mut user = make_user();

        user.change_status(UserStatus::Locked, None);

        assert_eq!(user.status, UserStatus::Locked);
        assert!(user.audit.updater.is_none());
    }

    #[test]
    fn test_change_status_raises_user_status_changed_event() {
        let mut user = make_user();
        let before = user.events().len();

        user.change_status(UserStatus::Disabled, None);

        assert_eq!(user.events().len(), before + 1);
        match user.events().last().unwrap() {
            DomainEvent::UserStatusChanged { user_id, status } => {
                assert_eq!(*user_id, 1);
                assert_eq!(*status, UserStatus::Disabled);
            }
            _ => panic!("expected UserStatusChanged event"),
        }
    }

    #[test]
    fn test_change_status_back_to_active() {
        let mut user = make_user();
        user.status = UserStatus::Locked;

        user.change_status(UserStatus::Active, None);

        assert_eq!(user.status, UserStatus::Active);
    }

    // ============================================================
    // User::change_password
    // ============================================================

    #[test]
    fn test_change_password_updates_field_and_audit() {
        let mut user = make_user();

        user.change_password("new_hashed_pwd".into(), Some("admin".into()));

        assert_eq!(user.password, "new_hashed_pwd");
        assert_eq!(user.audit.updater.as_deref(), Some("admin"));
    }

    #[test]
    fn test_change_password_raises_event() {
        let mut user = make_user();
        let before = user.events().len();

        user.change_password("new_pwd".into(), None);

        assert_eq!(user.events().len(), before + 1);
        assert!(matches!(user.events().last(), Some(DomainEvent::UserPasswordChanged { user_id: 1 })));
    }

    // ============================================================
    // User::record_login
    // ============================================================

    #[test]
    fn test_record_login_sets_ip_and_date() {
        let mut user = make_user();

        user.record_login("192.168.1.1".into());

        assert_eq!(user.login_ip.as_deref(), Some("192.168.1.1"));
        assert!(user.login_date.is_some());
    }

    #[test]
    fn test_record_login_raises_user_logged_in_event() {
        let mut user = make_user();
        let before = user.events().len();

        user.record_login("10.0.0.1".into());

        assert_eq!(user.events().len(), before + 1);
        match user.events().last().unwrap() {
            DomainEvent::UserLoggedIn { user_id, ip } => {
                assert_eq!(*user_id, 1);
                assert_eq!(ip, "10.0.0.1");
            }
            _ => panic!("expected UserLoggedIn event"),
        }
    }

    // ============================================================
    // User::soft_delete
    // ============================================================

    #[test]
    fn test_soft_delete_marks_as_deleted() {
        let mut user = make_user();

        user.soft_delete(Some("admin".into()));

        assert_eq!(user.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_soft_delete_raises_user_deleted_event() {
        let mut user = make_user();
        let before = user.events().len();

        user.soft_delete(None);

        assert_eq!(user.events().len(), before + 1);
        assert!(matches!(user.events().last(), Some(DomainEvent::UserDeleted { user_id: 1 })));
    }

    // ============================================================
    // User::set_roles / set_departments
    // ============================================================

    #[test]
    fn test_set_roles_replaces_role_ids() {
        let mut user = make_user();
        user.role_ids = vec![1, 2, 3];

        user.set_roles(vec![4, 5]);

        assert_eq!(user.role_ids, vec![4, 5]);
    }

    #[test]
    fn test_set_roles_to_empty() {
        let mut user = make_user();
        user.role_ids = vec![1, 2];

        user.set_roles(vec![]);

        assert!(user.role_ids.is_empty());
    }

    #[test]
    fn test_set_departments_replaces_dept_ids() {
        let mut user = make_user();
        user.dept_ids = vec![10, 20];

        user.set_departments(vec![30]);

        assert_eq!(user.dept_ids, vec![30]);
    }

    #[test]
    fn test_set_departments_does_not_raise_event() {
        let mut user = make_user();
        let before = user.events().len();

        user.set_departments(vec![1, 2]);

        assert_eq!(user.events().len(), before); // 无领域事件
    }

    // ============================================================
    // User::is_active / is_locked
    // ============================================================

    #[test]
    fn test_is_active_returns_true_for_active_undeleted_user() {
        let user = make_user();
        assert!(user.is_active());
    }

    #[test]
    fn test_is_active_returns_false_when_disabled() {
        let mut user = make_user();
        user.status = UserStatus::Disabled;
        assert!(!user.is_active());
    }

    #[test]
    fn test_is_active_returns_false_when_soft_deleted() {
        let mut user = make_user();
        user.audit.deleted = DeletedStatus::Deleted;
        assert!(!user.is_active());
    }

    #[test]
    fn test_is_locked_returns_true_when_locked() {
        let mut user = make_user();
        user.status = UserStatus::Locked;
        assert!(user.is_locked());
    }

    #[test]
    fn test_is_locked_returns_false_when_active() {
        let user = make_user();
        assert!(!user.is_locked());
    }

    #[test]
    fn test_is_locked_returns_false_when_disabled() {
        let mut user = make_user();
        user.status = UserStatus::Disabled;
        assert!(!user.is_locked());
    }

    // ============================================================
    // AggregateRoot trait (events management)
    // ============================================================

    #[test]
    fn test_clear_events_empties_event_list() {
        let mut user = make_user();
        assert_eq!(user.events().len(), 1);

        user.clear_events();
        assert!(user.events().is_empty());
    }

    // ============================================================
    // Multiple events accumulation
    // ============================================================

    #[test]
    fn test_multiple_operations_accumulate_events() {
        let mut user = make_user();
        user.clear_events(); // 清除 create 事件

        user.change_status(UserStatus::Disabled, None);   // StatusChanged
        user.change_password("pwd2".into(), None);        // PasswordChanged
        user.record_login("1.1.1.1".into());              // LoggedIn

        assert_eq!(user.events().len(), 3);
    }

    // ============================================================
    // UserStatus::try_from_i32
    // ============================================================

    #[test]
    fn test_user_status_try_from_i32_valid_values() {
        assert_eq!(UserStatus::try_from_i32(0).unwrap(), UserStatus::Active);
        assert_eq!(UserStatus::try_from_i32(1).unwrap(), UserStatus::Disabled);
        assert_eq!(UserStatus::try_from_i32(2).unwrap(), UserStatus::Locked);
    }

    #[test]
    fn test_user_status_try_from_i32_invalid_value() {
        let err = UserStatus::try_from_i32(99).unwrap_err();
        assert_eq!(err.0, 99);
        assert_eq!(err.to_string(), "invalid user status value: 99");
    }

    #[test]
    fn test_user_status_try_from_i32_negative_value() {
        let err = UserStatus::try_from_i32(-1).unwrap_err();
        assert_eq!(err.0, -1);
    }

    #[test]
    fn test_user_status_from_i32_fallback() {
        // From<i32> falls back to Active for invalid values
        assert_eq!(UserStatus::from(99), UserStatus::Active);
        assert_eq!(UserStatus::from(-1), UserStatus::Active);
    }

    // ============================================================
    // Sex::From<i32>
    // ============================================================

    #[test]
    fn test_sex_from_i32_valid_values() {
        assert_eq!(Sex::from(0), Sex::Unknown);
        assert_eq!(Sex::from(1), Sex::Male);
        assert_eq!(Sex::from(2), Sex::Female);
    }

    #[test]
    fn test_sex_from_i32_invalid_falls_back_to_unknown() {
        assert_eq!(Sex::from(99), Sex::Unknown);
        assert_eq!(Sex::from(-1), Sex::Unknown);
    }

    // ============================================================
    // UserStatus serde roundtrip (via serde_json for i32 repr)
    // ============================================================

    #[test]
    fn test_user_status_serialize_deserialize() {
        let status = UserStatus::Disabled;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "1");
        let back: UserStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, UserStatus::Disabled);
    }

    #[test]
    fn test_sex_serialize_deserialize() {
        let sex = Sex::Male;
        let json = serde_json::to_string(&sex).unwrap();
        assert_eq!(json, "1");
        let back: Sex = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Sex::Male);
    }

    // ============================================================
    // AuditFields::is_deleted check via soft_delete
    // ============================================================

    #[test]
    fn test_audit_is_deleted_after_soft_delete() {
        let mut user = make_user();
        assert!(!user.audit.is_deleted());

        user.soft_delete(None);
        assert!(user.audit.is_deleted());
    }

    // ============================================================
    // Business rule: inactive user is not active
    // ============================================================

    #[test]
    fn test_disabled_user_is_not_active() {
        let mut user = make_user();
        user.status = UserStatus::Disabled;
        assert!(!user.is_active());
    }

    #[test]
    fn test_locked_user_is_not_active() {
        let mut user = make_user();
        user.status = UserStatus::Locked;
        assert!(!user.is_active());
    }

    #[test]
    fn test_deleted_user_is_not_active() {
        let mut user = make_user();
        user.audit.deleted = DeletedStatus::Deleted;
        assert!(!user.is_active());
    }

    #[test]
    fn test_locked_user_is_locked() {
        let mut user = make_user();
        user.status = UserStatus::Locked;
        assert!(user.is_locked());
    }

    #[test]
    fn test_active_user_is_not_locked() {
        let user = make_user();
        assert!(!user.is_locked());
    }

    #[test]
    fn test_disabled_user_is_not_locked() {
        let mut user = make_user();
        user.status = UserStatus::Disabled;
        assert!(!user.is_locked());
    }

    // ============================================================
    // Business rule: restore does not raise events
    // ============================================================

    #[test]
    fn test_restore_does_not_raise_events() {
        let user = User::restore(
            1, "u".into(), "p".into(), "N".into(), None,
            None, None, Sex::Unknown, None, UserStatus::Active,
            None, None, TenantId::default(), AuditFields::default(),
            vec![], vec![],
        );
        assert!(user.events().is_empty());
    }

    // ============================================================
    // Business rule: set_roles/set_departments replace entire list
    // ============================================================

    #[test]
    fn test_set_roles_replaces_all() {
        let mut user = make_user();
        user.role_ids = vec![1, 2, 3];
        user.set_roles(vec![4]);
        assert_eq!(user.role_ids, vec![4]);
    }

    #[test]
    fn test_set_departments_replaces_all() {
        let mut user = make_user();
        user.dept_ids = vec![10, 20];
        user.set_departments(vec![]);
        assert!(user.dept_ids.is_empty());
    }

    // ============================================================
    // Business rule: change_password updates audit timestamp
    // ============================================================

    #[test]
    fn test_change_password_updates_audit() {
        let mut user = make_user();
        user.change_password("new_hash".into(), Some("admin".into()));
        assert_eq!(user.password, "new_hash");
        assert_eq!(user.audit.updater.as_deref(), Some("admin"));
    }
}
