// ============================================================
// UNIT TESTS: Role 聚合根
// Coverage: create, update_info, change_status, set_menus,
//           soft_delete, is_active
// ============================================================

#[cfg(test)]
mod role_tests {
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::model::AggregateRoot;
    use crate::role::model::aggregate::Role;
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    fn make_role() -> Role {
        Role::create(100, "Admin".into(), "admin".into(), 1, Some("system".into()))
    }

    #[test]
    fn test_create_role_sets_defaults() {
        let role = Role::create(1, "Editor".into(), "editor".into(), 5, None);
        assert_eq!(role.id, 1);
        assert_eq!(role.name, "Editor");
        assert_eq!(role.code, "editor");
        assert_eq!(role.sort, 5);
        assert_eq!(role.data_scope, 4);
        assert_eq!(role.status, 0);
        assert!(role.remark.is_none());
        assert!(role.menu_ids.is_empty());
    }

    #[test]
    fn test_create_role_raises_event() {
        let role = make_role();
        assert_eq!(role.events().len(), 1);
        assert!(matches!(role.events()[0], DomainEvent::RoleCreated { role_id: 100 }));
    }

    #[test]
    fn test_update_info_changes_all_fields() {
        let mut role = make_role();
        role.update_info("SuperAdmin".into(), "super_admin".into(), 0, 2, Some("remark".into()), Some("updater".into()));

        assert_eq!(role.name, "SuperAdmin");
        assert_eq!(role.code, "super_admin");
        assert_eq!(role.sort, 0);
        assert_eq!(role.data_scope, 2);
        assert_eq!(role.remark.as_deref(), Some("remark"));
        assert_eq!(role.audit.updater.as_deref(), Some("updater"));
    }

    #[test]
    fn test_update_info_raises_event() {
        let mut role = make_role();
        let before = role.events().len();
        role.update_info("X".into(), "x".into(), 1, 1, None, None);
        assert_eq!(role.events().len(), before + 1);
        assert!(matches!(role.events().last(), Some(DomainEvent::RoleUpdated { role_id: 100 })));
    }

    #[test]
    fn test_change_status() {
        let mut role = make_role();
        role.change_status(1, Some("admin".into()));
        assert_eq!(role.status, 1);
    }

    #[test]
    fn test_set_menus() {
        let mut role = make_role();
        role.set_menus(vec![1, 2, 3]);
        assert_eq!(role.menu_ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_set_menus_raises_event() {
        let mut role = make_role();
        let before = role.events().len();
        role.set_menus(vec![1]);
        assert_eq!(role.events().len(), before + 1);
        assert!(matches!(role.events().last(), Some(DomainEvent::RolePermissionsChanged { role_id: 100 })));
    }

    #[test]
    fn test_soft_delete() {
        let mut role = make_role();
        role.soft_delete(None);
        assert_eq!(role.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_soft_delete_raises_event() {
        let mut role = make_role();
        let before = role.events().len();
        role.soft_delete(None);
        assert_eq!(role.events().len(), before + 1);
        assert!(matches!(role.events().last(), Some(DomainEvent::RoleDeleted { role_id: 100 })));
    }

    #[test]
    fn test_is_active_true() {
        let role = make_role();
        assert!(role.is_active());
    }

    #[test]
    fn test_is_active_false_when_disabled() {
        let mut role = make_role();
        role.status = 1;
        assert!(!role.is_active());
    }

    #[test]
    fn test_is_active_false_when_deleted() {
        let mut role = make_role();
        role.audit.deleted = DeletedStatus::Deleted;
        assert!(!role.is_active());
    }

    // ============================================================
    // Business rule: restore does not raise events
    // ============================================================

    #[test]
    fn test_restore_does_not_raise_events() {
        use crate::shared::model::AuditFields;
        let role = Role::restore(
            1, "R".into(), "r".into(), 0, 4, None, 0, None, 0,
            AuditFields::default(), vec![],
        );
        assert!(role.events().is_empty());
    }

    // ============================================================
    // Business rule: change_status does not raise event
    // (This is a design choice - status change is silent)
    // ============================================================

    #[test]
    fn test_change_status_does_not_raise_event() {
        let mut role = make_role();
        let before = role.events().len();
        role.change_status(1, Some("admin".into()));
        // change_status does NOT add a domain event (by design)
        assert_eq!(role.events().len(), before);
    }

    // ============================================================
    // Business rule: set_menus replaces entire list
    // ============================================================

    #[test]
    fn test_set_menus_replaces_all() {
        let mut role = make_role();
        role.menu_ids = vec![1, 2, 3];
        role.set_menus(vec![10]);
        assert_eq!(role.menu_ids, vec![10]);
    }

    #[test]
    fn test_set_menus_to_empty() {
        let mut role = make_role();
        role.menu_ids = vec![1, 2];
        role.set_menus(vec![]);
        assert!(role.menu_ids.is_empty());
    }

    // ============================================================
    // Business rule: soft_delete sets deleted status
    // ============================================================

    #[test]
    fn test_soft_delete_sets_audit() {
        let mut role = make_role();
        role.soft_delete(Some("admin".into()));
        assert_eq!(role.audit.deleted, DeletedStatus::Deleted);
        assert_eq!(role.audit.updater.as_deref(), Some("admin"));
    }

    // ============================================================
    // Business rule: create sets defaults
    // ============================================================

    #[test]
    fn test_create_sets_default_data_scope() {
        let role = Role::create(1, "R".into(), "r".into(), 0, None);
        assert_eq!(role.data_scope, 4); // default data scope
        assert_eq!(role.status, 0); // active by default
        assert_eq!(role.tenant_id, 0);
        assert!(role.menu_ids.is_empty());
    }

    // ============================================================
    // Business rule: update_info updates all fields
    // ============================================================

    #[test]
    fn test_update_info_clears_remark_with_none() {
        let mut role = make_role();
        role.remark = Some("old remark".into());
        role.update_info("X".into(), "x".into(), 0, 0, None, None);
        assert!(role.remark.is_none());
    }
}
