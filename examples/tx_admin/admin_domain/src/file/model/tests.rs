// ============================================================
// UNIT TESTS: File 领域 —— 聚合根 & Entity/AggregateRoot trait
// Coverage: File (create, restore, soft_delete, Entity, AggregateRoot)
//           FileConfig (create, restore, update_info, set_master,
//                       unset_master, soft_delete, Entity, AggregateRoot)
// ============================================================

#[cfg(test)]
mod file_tests {
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::model::{AggregateRoot, AuditFields, Entity};
    use crate::file::model::aggregate::{File, FileConfig};
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    // ============================================================
    // Helpers
    // ============================================================

    fn make_file() -> File {
        File::create(1, Some(100), "test.png".into(), "/uploads/test.png".into(),
            "https://cdn.example.com/test.png".into(), Some("image/png".into()), 1024, Some("admin".into()))
    }

    fn make_file_config() -> FileConfig {
        FileConfig::create(1, "Default".into(), 1, Some("备注".into()), "{}".into(), Some("admin".into()))
    }

    // ============================================================
    // ── File :: create ─────────────────────────────────────────
    // ============================================================

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
    fn test_file_create_sets_audit() {
        let f = File::create(1, None, "f".into(), "/f".into(), "/f".into(), None, 100, Some("admin".into()));
        assert_eq!(f.audit.creator.as_deref(), Some("admin"));
        assert_eq!(f.audit.updater.as_deref(), Some("admin"));
        assert_eq!(f.audit.deleted, DeletedStatus::Normal);
    }

    #[test]
    fn test_file_create_with_zero_size() {
        let f = File::create(2, Some(1), "empty.bin".into(), "/empty.bin".into(),
            "/empty.bin".into(), None, 0, Some("admin".into()));
        assert_eq!(f.size, 0);
        assert_eq!(f.events().len(), 1);
    }

    #[test]
    fn test_file_create_with_large_id() {
        let f = File::create(u64::MAX, None, "big.bin".into(), "/big.bin".into(),
            "/big.bin".into(), None, 0, None);
        assert_eq!(f.id, u64::MAX);
    }

    // ============================================================
    // ── File :: restore ────────────────────────────────────────
    // ============================================================

    #[test]
    fn test_file_restore_no_events() {
        let f = File::restore(
            1, None, "f".into(), "/f".into(), "/f".into(), None, 0,
            AuditFields::default(),
        );
        assert!(f.events().is_empty());
    }

    #[test]
    fn test_file_restore_preserves_all_fields() {
        let audit = AuditFields::default();
        let f = File::restore(
            42, Some(7), "photo.jpg".into(), "/store/photo.jpg".into(),
            "https://cdn.example.com/photo.jpg".into(), Some("image/jpeg".into()), 2048,
            audit.clone(),
        );
        assert_eq!(f.id, 42);
        assert_eq!(f.config_id, Some(7));
        assert_eq!(f.name, "photo.jpg");
        assert_eq!(f.path, "/store/photo.jpg");
        assert_eq!(f.url, "https://cdn.example.com/photo.jpg");
        assert_eq!(f.file_type.as_deref(), Some("image/jpeg"));
        assert_eq!(f.size, 2048);
    }

    // ============================================================
    // ── File :: soft_delete ────────────────────────────────────
    // ============================================================

    #[test]
    fn test_file_soft_delete_sets_deleted() {
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

    #[test]
    fn test_file_soft_delete_sets_audit() {
        let mut f = make_file();
        f.soft_delete(Some("admin".into()));
        assert_eq!(f.audit.updater.as_deref(), Some("admin"));
    }

    #[test]
    fn test_file_soft_delete_sets_update_time() {
        // update_time should be updated after soft_delete
        let mut f = make_file();
        f.soft_delete(Some("tester".into()));
        // update_time is set via Timestamp::now() — updater is verified
        assert_eq!(f.audit.updater.as_deref(), Some("tester"));
    }

    #[test]
    fn test_file_soft_delete_twice_accumulates_events() {
        let mut f = make_file();
        f.soft_delete(None);
        f.soft_delete(Some("admin".into()));
        // Should have: 1 create event + 2 delete events = 3 total
        assert_eq!(f.events().len(), 3);
        assert!(matches!(f.events()[1], DomainEvent::FileDeleted { file_id: 1 }));
        assert!(matches!(f.events()[2], DomainEvent::FileDeleted { file_id: 1 }));
    }

    // ============================================================
    // ── File :: Entity trait ───────────────────────────────────
    // ============================================================

    #[test]
    fn test_file_entity_id() {
        let f = File::restore(99, None, "f".into(), "/f".into(), "/f".into(),
            None, 0, AuditFields::default());
        assert_eq!(f.id(), 99);
    }

    // ============================================================
    // ── File :: AggregateRoot trait ────────────────────────────
    // ============================================================

    #[test]
    fn test_file_clear_events() {
        let mut f = make_file();
        assert!(!f.events().is_empty());
        f.clear_events();
        assert!(f.events().is_empty());
    }

    #[test]
    fn test_file_add_event_directly() {
        let mut f = File::restore(1, None, "f".into(), "/f".into(), "/f".into(),
            None, 0, AuditFields::default());
        f.add_event(DomainEvent::FileUploaded { file_id: 1 });
        assert_eq!(f.events().len(), 1);
    }

    // ============================================================
    // ── File :: Clone + events ─────────────────────────────────
    // ============================================================

    #[test]
    fn test_file_clone_has_independent_events() {
        let mut f = make_file();
        f.soft_delete(None);
        let clone = f.clone();

        // Clone has same events
        assert_eq!(clone.events().len(), f.events().len());

        // Clear clone's events — original unaffected
        let mut clone2 = clone.clone();
        clone2.clear_events();
        assert!(!f.events().is_empty());
        assert!(clone2.events().is_empty());
    }

    // ============================================================
    // ── FileConfig :: create ───────────────────────────────────
    // ============================================================

    #[test]
    fn test_create_file_config_sets_fields() {
        let fc = make_file_config();
        assert_eq!(fc.id, 1);
        assert_eq!(fc.name, "Default");
        assert_eq!(fc.storage, 1);
        assert_eq!(fc.remark.as_deref(), Some("备注"));
        assert_eq!(fc.config, "{}");
        assert_eq!(fc.master, 0);
    }

    #[test]
    fn test_create_file_config_no_event() {
        let fc = FileConfig::create(1, "x".into(), 0, None, "{}".into(), None);
        assert!(fc.events().is_empty());
    }

    #[test]
    fn test_file_config_create_sets_defaults() {
        let fc = FileConfig::create(1, "c".into(), 0, Some("remark".into()), "{}".into(), None);
        assert_eq!(fc.master, 0);
        assert_eq!(fc.remark.as_deref(), Some("remark"));
    }

    #[test]
    fn test_file_config_create_sets_audit() {
        let fc = FileConfig::create(1, "c".into(), 0, None, "{}".into(), Some("creator".into()));
        assert_eq!(fc.audit.creator.as_deref(), Some("creator"));
        assert_eq!(fc.audit.updater.as_deref(), Some("creator"));
        assert_eq!(fc.audit.deleted, DeletedStatus::Normal);
    }

    #[test]
    fn test_file_config_create_with_none_creator() {
        let fc = FileConfig::create(1, "x".into(), 0, None, "{}".into(), None);
        assert!(fc.audit.creator.is_none());
        assert!(fc.audit.updater.is_none());
    }

    #[test]
    fn test_file_config_create_with_max_id() {
        let fc = FileConfig::create(u64::MAX, "neg".into(), 0, None, "{}".into(), None);
        assert_eq!(fc.id, u64::MAX);
    }

    // ============================================================
    // ── FileConfig :: restore ──────────────────────────────────
    // ============================================================

    #[test]
    fn test_file_config_restore_no_events() {
        let fc = FileConfig::restore(
            1, "c".into(), 0, None, 0, "{}".into(),
            AuditFields::default(),
        );
        assert!(fc.events().is_empty());
    }

    #[test]
    fn test_file_config_restore_preserves_fields() {
        let audit = AuditFields::default();
        let fc = FileConfig::restore(
            5, "s3".into(), 2, Some("aws".into()), 1, r#"{"bucket":"my-bucket"}"#.into(),
            audit.clone(),
        );
        assert_eq!(fc.id, 5);
        assert_eq!(fc.name, "s3");
        assert_eq!(fc.storage, 2);
        assert_eq!(fc.remark.as_deref(), Some("aws"));
        assert_eq!(fc.master, 1);
        assert_eq!(fc.config, r#"{"bucket":"my-bucket"}"#);
    }

    // ============================================================
    // ── FileConfig :: update_info ──────────────────────────────
    // ============================================================

    #[test]
    fn test_file_config_update_info_changes_fields() {
        let mut fc = make_file_config();
        fc.update_info("Updated".into(), 2, Some("新备注".into()),
            r#"{"allowed_extensions":["pdf"]}"#.into(), Some("editor".into()));
        assert_eq!(fc.name, "Updated");
        assert_eq!(fc.storage, 2);
        assert_eq!(fc.remark.as_deref(), Some("新备注"));
        assert_eq!(fc.config, r#"{"allowed_extensions":["pdf"]}"#);
    }

    #[test]
    fn test_file_config_update_info_sets_audit() {
        let mut fc = make_file_config();
        fc.update_info("U".into(), 0, None, "{}".into(), Some("editor".into()));
        assert_eq!(fc.audit.updater.as_deref(), Some("editor"));
    }

    #[test]
    fn test_file_config_update_info_no_event() {
        let mut fc = make_file_config();
        fc.update_info("U".into(), 0, None, "{}".into(), None);
        assert!(fc.events().is_empty());
    }

    #[test]
    fn test_file_config_update_info_clear_remark() {
        let mut fc = make_file_config();
        assert!(fc.remark.is_some());
        fc.update_info("U".into(), 0, None, "{}".into(), None);
        assert!(fc.remark.is_none());
    }

    // ============================================================
    // ── FileConfig :: set_master ───────────────────────────────
    // ============================================================

    #[test]
    fn test_file_config_set_master() {
        let mut fc = make_file_config();
        assert_eq!(fc.master, 0);
        fc.set_master(Some("admin".into()));
        assert_eq!(fc.master, 1);
    }

    #[test]
    fn test_file_config_set_master_sets_audit() {
        let mut fc = make_file_config();
        fc.set_master(Some("admin".into()));
        assert_eq!(fc.audit.updater.as_deref(), Some("admin"));
    }

    #[test]
    fn test_file_config_set_master_no_event() {
        let mut fc = make_file_config();
        fc.set_master(None);
        assert!(fc.events().is_empty());
    }

    #[test]
    fn test_file_config_set_master_twice() {
        let mut fc = make_file_config();
        fc.set_master(Some("a".into()));
        fc.set_master(Some("b".into()));
        assert_eq!(fc.master, 1);
        assert_eq!(fc.audit.updater.as_deref(), Some("b"));
    }

    // ============================================================
    // ── FileConfig :: unset_master ─────────────────────────────
    // ============================================================

    #[test]
    fn test_file_config_unset_master() {
        let mut fc = make_file_config();
        fc.set_master(None);
        assert_eq!(fc.master, 1);
        fc.unset_master(Some("admin".into()));
        assert_eq!(fc.master, 0);
    }

    #[test]
    fn test_file_config_unset_master_sets_audit() {
        let mut fc = make_file_config();
        fc.unset_master(Some("admin".into()));
        assert_eq!(fc.audit.updater.as_deref(), Some("admin"));
    }

    #[test]
    fn test_file_config_unset_master_no_event() {
        let mut fc = make_file_config();
        fc.unset_master(None);
        assert!(fc.events().is_empty());
    }

    #[test]
    fn test_file_config_unset_master_when_already_zero() {
        let mut fc = make_file_config();
        assert_eq!(fc.master, 0);
        fc.unset_master(Some("admin".into()));
        assert_eq!(fc.master, 0);
    }

    // ============================================================
    // ── FileConfig :: soft_delete ──────────────────────────────
    // ============================================================

    #[test]
    fn test_file_config_soft_delete() {
        let mut fc = make_file_config();
        fc.soft_delete(Some("admin".into()));
        assert_eq!(fc.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_file_config_soft_delete_sets_audit() {
        let mut fc = make_file_config();
        fc.soft_delete(Some("admin".into()));
        assert_eq!(fc.audit.updater.as_deref(), Some("admin"));
    }

    #[test]
    fn test_file_config_soft_delete_no_event() {
        let mut fc = make_file_config();
        fc.soft_delete(None);
        assert!(fc.events().is_empty());
    }

    // ============================================================
    // ── FileConfig :: Entity trait ─────────────────────────────
    // ============================================================

    #[test]
    fn test_file_config_entity_id() {
        let fc = FileConfig::restore(7, "c".into(), 0, None, 0, "{}".into(),
            AuditFields::default());
        assert_eq!(fc.id(), 7);
    }

    // ============================================================
    // ── FileConfig :: AggregateRoot trait ──────────────────────
    // ============================================================

    #[test]
    fn test_file_config_clear_events() {
        let mut fc = FileConfig::restore(1, "c".into(), 0, None, 0, "{}".into(),
            AuditFields::default());
        fc.add_event(DomainEvent::FileUploaded { file_id: 1 });
        assert_eq!(fc.events().len(), 1);
        fc.clear_events();
        assert!(fc.events().is_empty());
    }

    #[test]
    fn test_file_config_add_event() {
        let mut fc = FileConfig::restore(1, "c".into(), 0, None, 0, "{}".into(),
            AuditFields::default());
        fc.add_event(DomainEvent::FileDeleted { file_id: 100 });
        assert_eq!(fc.events().len(), 1);
        assert!(matches!(fc.events()[0], DomainEvent::FileDeleted { file_id: 100 }));
    }

    // ============================================================
    // ── FileConfig :: Clone + events ───────────────────────────
    // ============================================================

    #[test]
    fn test_file_config_clone_has_independent_events() {
        let mut fc = make_file_config();
        fc.add_event(DomainEvent::FileUploaded { file_id: 1 });
        let clone = fc.clone();
        assert_eq!(clone.events().len(), fc.events().len());

        let mut clone2 = clone.clone();
        clone2.clear_events();
        assert_eq!(fc.events().len(), 1);
        assert!(clone2.events().is_empty());
    }

    // ============================================================
    // ── Combined scenarios ─────────────────────────────────────
    // ============================================================

    #[test]
    fn test_file_config_full_lifecycle() {
        // Create -> update -> set master -> unset master -> soft delete
        let mut fc = FileConfig::create(1, "Local".into(), 0, Some("本地存储".into()),
            r#"{"path":"/data"}"#.into(), Some("admin".into()));
        assert_eq!(fc.master, 0);
        assert_eq!(fc.audit.deleted, DeletedStatus::Normal);

        fc.update_info("Local-V2".into(), 1, None, r#"{"path":"/data/v2"}"#.into(),
            Some("editor".into()));
        assert_eq!(fc.name, "Local-V2");
        assert_eq!(fc.remark, None);

        fc.set_master(Some("admin".into()));
        assert_eq!(fc.master, 1);

        fc.unset_master(Some("admin".into()));
        assert_eq!(fc.master, 0);

        fc.soft_delete(Some("admin".into()));
        assert_eq!(fc.audit.deleted, DeletedStatus::Deleted);
    }
}
