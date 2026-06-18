// ============================================================
// UNIT TESTS: File 聚合根
// Coverage: File::create, soft_delete; FileConfig::create
// ============================================================

#[cfg(test)]
mod file_tests {
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::model::AggregateRoot;
    use crate::file::model::aggregate::{File, FileConfig};
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    // ── File ──────────────────────────────────────────

    fn make_file() -> File {
        File::create(1, Some(100), "test.png".into(), "/uploads/test.png".into(),
            "https://cdn.example.com/test.png".into(), Some("image/png".into()), 1024, Some("admin".into()))
    }

    #[test]
    fn test_create_file_sets_fields() {
        let f = make_file();
        assert_eq!(f.id, 1);
        assert_eq!(f.config_id, Some(100));
        assert_eq!(f.name, "test.png");
        assert_eq!(f.path, "/uploads/test.png");
        assert_eq!(f.url, "https://cdn.example.com/test.png");
        assert_eq!(f.file_type.as_deref(), Some("image/png"));
        assert_eq!(f.size, 1024);
    }

    #[test]
    fn test_create_file_with_none_optionals() {
        let f = File::create(1, None, "a.txt".into(), "/a.txt".into(), "/a.txt".into(), None, 0, None);
        assert!(f.config_id.is_none());
        assert!(f.file_type.is_none());
    }

    #[test]
    fn test_create_file_raises_event() {
        let f = File::create(1, None, "a.txt".into(), "/a.txt".into(), "/a.txt".into(), None, 0, None);
        assert_eq!(f.events().len(), 1);
        assert!(matches!(f.events()[0], DomainEvent::FileUploaded { file_id: 1 }));
    }

    #[test]
    fn test_file_soft_delete() {
        let mut f = make_file();
        f.soft_delete(None);
        assert_eq!(f.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_file_soft_delete_raises_event() {
        let mut f = make_file();
        let before = f.events().len();
        f.soft_delete(None);
        assert_eq!(f.events().len(), before + 1);
        assert!(matches!(f.events().last(), Some(DomainEvent::FileDeleted { file_id: 1 })));
    }

    // ── FileConfig ────────────────────────────────────

    fn make_file_config() -> FileConfig {
        FileConfig::create(1, "Default".into(), 1, "{}".into(), Some("admin".into()))
    }

    #[test]
    fn test_create_file_config_sets_fields() {
        let fc = make_file_config();
        assert_eq!(fc.id, 1);
        assert_eq!(fc.name, "Default");
        assert_eq!(fc.storage, 1);
        assert_eq!(fc.config, "{}");
        assert_eq!(fc.master, 0);
    }

    #[test]
    fn test_create_file_config_no_event() {
        let fc = FileConfig::create(1, "x".into(), 0, "{}".into(), None);
        assert!(fc.events().is_empty());
    }

    // ============================================================
    // Business rule: restore does not raise events
    // ============================================================

    #[test]
    fn test_file_restore_no_events() {
        use crate::shared::model::AuditFields;
        let f = File::restore(
            1, None, "f".into(), "/f".into(), "/f".into(), None, 0,
            AuditFields::default(),
        );
        assert!(f.events().is_empty());
    }

    #[test]
    fn test_file_config_restore_no_events() {
        use crate::shared::model::AuditFields;
        let fc = FileConfig::restore(
            1, "c".into(), 0, None, 0, "{}".into(),
            AuditFields::default(),
        );
        assert!(fc.events().is_empty());
    }

    // ============================================================
    // Business rule: soft_delete sets audit
    // ============================================================

    #[test]
    fn test_file_soft_delete_sets_audit() {
        let mut f = make_file();
        f.soft_delete(Some("admin".into()));
        assert_eq!(f.audit.deleted, DeletedStatus::Deleted);
        assert_eq!(f.audit.updater.as_deref(), Some("admin"));
    }

    // ============================================================
    // Business rule: create sets defaults
    // ============================================================

    #[test]
    fn test_file_create_sets_audit() {
        let f = File::create(1, None, "f".into(), "/f".into(), "/f".into(), None, 100, Some("admin".into()));
        assert_eq!(f.audit.creator.as_deref(), Some("admin"));
        assert_eq!(f.audit.updater.as_deref(), Some("admin"));
        assert_eq!(f.audit.deleted, DeletedStatus::Normal);
    }

    #[test]
    fn test_file_config_create_sets_defaults() {
        let fc = FileConfig::create(1, "c".into(), 0, "{}".into(), None);
        assert_eq!(fc.master, 0);
        assert!(fc.remark.is_none());
    }
}
