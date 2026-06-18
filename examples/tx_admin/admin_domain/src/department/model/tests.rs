// ============================================================
// UNIT TESTS: Department 聚合根
// Coverage: create, update_info, change_status, soft_delete, is_root
// ============================================================

#[cfg(test)]
mod dept_tests {
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::model::AggregateRoot;
    use crate::department::model::aggregate::Department;
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    fn make_dept() -> Department {
        Department::create(1, "Engineering".into(), 0, 0, Some("admin".into()))
    }

    #[test]
    fn test_create_department_sets_fields() {
        let d = make_dept();
        assert_eq!(d.id, 1);
        assert_eq!(d.name, "Engineering");
        assert_eq!(d.parent_id, 0);
        assert_eq!(d.status, 0);
        assert!(d.leader_user_id.is_none());
        assert!(d.children.is_empty());
    }

    #[test]
    fn test_create_raises_event() {
        let d = Department::create(2, "HR".into(), 1, 1, None);
        assert_eq!(d.events().len(), 1);
        assert!(matches!(d.events()[0], DomainEvent::DepartmentCreated { dept_id: 2 }));
    }

    #[test]
    fn test_update_info() {
        let mut d = make_dept();
        d.update_info("R&D".into(), 0, 2, Some(100), Some("123".into()), Some("rd@test.com".into()), Some("updater".into()));

        assert_eq!(d.name, "R&D");
        assert_eq!(d.sort, 2);
        assert_eq!(d.leader_user_id, Some(100));
        assert_eq!(d.phone.as_deref(), Some("123"));
        assert_eq!(d.email.as_deref(), Some("rd@test.com"));
    }

    #[test]
    fn test_update_info_raises_event() {
        let mut d = make_dept();
        let before = d.events().len();
        d.update_info("X".into(), 0, 0, None, None, None, None);
        assert_eq!(d.events().len(), before + 1);
        assert!(matches!(d.events().last(), Some(DomainEvent::DepartmentUpdated { dept_id: 1 })));
    }

    #[test]
    fn test_is_root() {
        let d = make_dept();
        assert!(d.is_root());
    }

    #[test]
    fn test_is_not_root() {
        let d = Department::create(3, "Child".into(), 1, 0, None);
        assert!(!d.is_root());
    }

    #[test]
    fn test_soft_delete() {
        let mut d = make_dept();
        d.soft_delete(None);
        assert_eq!(d.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_soft_delete_raises_event() {
        let mut d = make_dept();
        let before = d.events().len();
        d.soft_delete(None);
        assert_eq!(d.events().len(), before + 1);
        assert!(matches!(d.events().last(), Some(DomainEvent::DepartmentDeleted { dept_id: 1 })));
    }

    // ============================================================
    // Business rule: change_status does not raise event
    // ============================================================

    #[test]
    fn test_change_status_does_not_raise_event() {
        let mut d = make_dept();
        let before = d.events().len();
        d.change_status(1, Some("admin".into()));
        assert_eq!(d.events().len(), before);
        assert_eq!(d.status, 1);
    }

    // ============================================================
    // Business rule: restore does not raise events
    // ============================================================

    #[test]
    fn test_restore_does_not_raise_events() {
        use crate::shared::model::AuditFields;
        let d = Department::restore(
            1, "D".into(), 0, 0, None, None, None, 0, 0,
            AuditFields::default(),
        );
        assert!(d.events().is_empty());
    }

    // ============================================================
    // Business rule: update_info clears optional fields with None
    // ============================================================

    #[test]
    fn test_update_info_clears_optional_fields() {
        let mut d = make_dept();
        d.leader_user_id = Some(100);
        d.phone = Some("123".into());
        d.email = Some("a@b.com".into());
        d.update_info("X".into(), 0, 0, None, None, None, None);
        assert!(d.leader_user_id.is_none());
        assert!(d.phone.is_none());
        assert!(d.email.is_none());
    }

    // ============================================================
    // Business rule: soft_delete sets audit updater
    // ============================================================

    #[test]
    fn test_soft_delete_sets_audit_updater() {
        let mut d = make_dept();
        d.soft_delete(Some("admin".into()));
        assert_eq!(d.audit.deleted, DeletedStatus::Deleted);
        assert_eq!(d.audit.updater.as_deref(), Some("admin"));
    }

    // ============================================================
    // Business rule: create sets default status
    // ============================================================

    #[test]
    fn test_create_sets_default_status() {
        let d = Department::create(1, "D".into(), 0, 0, None);
        assert_eq!(d.status, 0);
        assert_eq!(d.tenant_id, 0);
        assert!(d.children.is_empty());
    }
}
