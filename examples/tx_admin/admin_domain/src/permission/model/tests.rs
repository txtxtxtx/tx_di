// ============================================================
// UNIT TESTS: Permission 聚合根 + 值对象 (PermissionType)
// Coverage: create, update_info, soft_delete, is_active,
//           restore, events, PermissionType value object
// ============================================================

#[cfg(test)]
mod permission_tests {
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::model::AggregateRoot;
    use crate::permission::model::aggregate::Permission;
    use crate::permission::model::value_object::PermissionType;
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    fn make_permission() -> Permission {
        Permission::create(
            1,
            "View Users".into(),
            "system:user:view".into(),
            PermissionType::Menu,
            0,
            1,
            Some("View user list".into()),
            Some("admin".into()),
        )
    }

    // ============================================================
    // Permission::create
    // ============================================================

    #[test]
    fn test_create_permission_sets_all_fields() {
        let p = make_permission();
        assert_eq!(p.id, 1);
        assert_eq!(p.name, "View Users");
        assert_eq!(p.permission_code, "system:user:view");
        assert_eq!(p.permission_type, PermissionType::Menu);
        assert_eq!(p.parent_id, 0);
        assert_eq!(p.sort, 1);
        assert_eq!(p.description.as_deref(), Some("View user list"));
        assert_eq!(p.status, 0);
        assert_eq!(p.audit.creator.as_deref(), Some("admin"));
        assert_eq!(p.audit.updater.as_deref(), Some("admin"));
        assert_eq!(p.audit.deleted, DeletedStatus::Normal);
    }

    #[test]
    fn test_create_permission_raises_event() {
        let p = make_permission();
        assert_eq!(p.events().len(), 1);
        assert!(matches!(p.events()[0], DomainEvent::PermissionCreated { permission_id: 1 }));
    }

    #[test]
    fn test_create_permission_with_none_creator() {
        let p = Permission::create(
            1, "P".into(), "p".into(), PermissionType::Menu, 0, 0, None, None,
        );
        assert!(p.audit.creator.is_none());
        assert!(p.audit.updater.is_none());
    }

    // ============================================================
    // Permission::update_info
    // ============================================================

    #[test]
    fn test_update_info_changes_all_fields() {
        let mut p = make_permission();
        p.update_info(
            "Edit Users".into(),
            "system:user:edit".into(),
            PermissionType::Button,
            0,
            2,
            Some("Edit user info".into()),
            Some("updater".into()),
        );

        assert_eq!(p.name, "Edit Users");
        assert_eq!(p.permission_code, "system:user:edit");
        assert_eq!(p.permission_type, PermissionType::Button);
        assert_eq!(p.sort, 2);
        assert_eq!(p.description.as_deref(), Some("Edit user info"));
        assert_eq!(p.audit.updater.as_deref(), Some("updater"));
    }

    #[test]
    fn test_update_info_raises_event() {
        let mut p = make_permission();
        let before = p.events().len();
        p.update_info(
            "X".into(), "x".into(), PermissionType::Api, 0, 0, None, None,
        );
        assert_eq!(p.events().len(), before + 1);
        assert!(matches!(p.events().last(), Some(DomainEvent::PermissionUpdated { permission_id: 1 })));
    }

    #[test]
    fn test_update_info_clears_description_with_none() {
        let mut p = make_permission();
        p.description = Some("old desc".into());
        p.update_info("X".into(), "x".into(), PermissionType::Menu, 0, 0, None, None);
        assert!(p.description.is_none());
    }

    // ============================================================
    // Permission::soft_delete
    // ============================================================

    #[test]
    fn test_soft_delete_marks_as_deleted() {
        let mut p = make_permission();
        p.soft_delete(None);
        assert_eq!(p.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_soft_delete_raises_event() {
        let mut p = make_permission();
        let before = p.events().len();
        p.soft_delete(None);
        assert_eq!(p.events().len(), before + 1);
        assert!(matches!(p.events().last(), Some(DomainEvent::PermissionDeleted { permission_id: 1 })));
    }

    #[test]
    fn test_soft_delete_sets_audit_updater() {
        let mut p = make_permission();
        p.soft_delete(Some("admin".into()));
        assert_eq!(p.audit.updater.as_deref(), Some("admin"));
    }

    // ============================================================
    // Permission::is_active
    // ============================================================

    #[test]
    fn test_is_active_true_for_active_undeleted() {
        let p = make_permission();
        assert!(p.is_active());
    }

    #[test]
    fn test_is_active_false_when_status_nonzero() {
        let mut p = make_permission();
        p.status = 1;
        assert!(!p.is_active());
    }

    #[test]
    fn test_is_active_false_when_deleted() {
        let mut p = make_permission();
        p.audit.deleted = DeletedStatus::Deleted;
        assert!(!p.is_active());
    }

    // ============================================================
    // Permission::restore
    // ============================================================

    #[test]
    fn test_restore_does_not_raise_events() {
        use crate::shared::model::AuditFields;
        let p = Permission::restore(
            1, "P".into(), "p".into(), PermissionType::Menu, 0, 0, None, 0,
            AuditFields::default(),
        );
        assert!(p.events().is_empty());
    }

    // ============================================================
    // AggregateRoot trait
    // ============================================================

    #[test]
    fn test_clear_events() {
        let mut p = make_permission();
        assert_eq!(p.events().len(), 1);
        p.clear_events();
        assert!(p.events().is_empty());
    }

    #[test]
    fn test_multiple_operations_accumulate_events() {
        let mut p = make_permission();
        p.clear_events();

        p.update_info("X".into(), "x".into(), PermissionType::Menu, 0, 0, None, None); // Updated
        p.soft_delete(None); // Deleted

        assert_eq!(p.events().len(), 2);
    }

    // ============================================================
    // Business rule: different permission types
    // ============================================================

    #[test]
    fn test_create_menu_permission() {
        let p = Permission::create(
            1, "P".into(), "p".into(), PermissionType::Menu, 0, 0, None, None,
        );
        assert_eq!(p.permission_type, PermissionType::Menu);
    }

    #[test]
    fn test_create_button_permission() {
        let p = Permission::create(
            1, "P".into(), "p".into(), PermissionType::Button, 0, 0, None, None,
        );
        assert_eq!(p.permission_type, PermissionType::Button);
    }

    #[test]
    fn test_create_api_permission() {
        let p = Permission::create(
            1, "P".into(), "p".into(), PermissionType::Api, 0, 0, None, None,
        );
        assert_eq!(p.permission_type, PermissionType::Api);
    }

    // ============================================================
    // Business rule: parent_id for hierarchical permissions
    // ============================================================

    #[test]
    fn test_root_permission_has_zero_parent() {
        let p = make_permission();
        assert_eq!(p.parent_id, 0);
    }

    #[test]
    fn test_child_permission_has_nonzero_parent() {
        let p = Permission::create(
            2, "Child".into(), "child".into(), PermissionType::Button, 1, 0, None, None,
        );
        assert_eq!(p.parent_id, 1);
    }
}
