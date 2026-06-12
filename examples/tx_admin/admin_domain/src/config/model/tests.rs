// ============================================================
// UNIT TESTS: Config 聚合根
// Coverage: create, update_info, soft_delete
// ============================================================

#[cfg(test)]
mod config_tests {
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::model::AggregateRoot;
    use crate::config::model::aggregate::Config;
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    fn make_config() -> Config {
        Config::create(1, "system".into(), 1, "SiteName".into(), "site.name".into(), "MyApp".into(), Some("admin".into()))
    }

    #[test]
    fn test_create_config_sets_fields() {
        let c = make_config();
        assert_eq!(c.id, 1);
        assert_eq!(c.category, "system");
        assert_eq!(c.config_key, "site.name");
        assert_eq!(c.value, "MyApp");
        assert_eq!(c.visible, 1);
    }

    #[test]
    fn test_create_config_raises_event() {
        let c = Config::create(2, "email".into(), 2, "SMTP".into(), "smtp.host".into(), "localhost".into(), None);
        assert_eq!(c.events().len(), 1);
        assert!(matches!(c.events()[0], DomainEvent::ConfigCreated { config_id: 2 }));
    }

    #[test]
    fn test_update_info_changes_all() {
        let mut c = make_config();
        c.update_info("email".into(), 2, "SMTP Host".into(), "smtp.host".into(), "mail.example.com".into(), 0, Some("remark".into()), Some("updater".into()));

        assert_eq!(c.category, "email");
        assert_eq!(c.value, "mail.example.com");
        assert_eq!(c.visible, 0);
        assert_eq!(c.remark.as_deref(), Some("remark"));
    }

    #[test]
    fn test_update_info_raises_event() {
        let mut c = make_config();
        let before = c.events().len();
        c.update_info("x".into(), 0, "x".into(), "x".into(), "x".into(), 0, None, None);
        assert_eq!(c.events().len(), before + 1);
        assert!(matches!(c.events().last(), Some(DomainEvent::ConfigUpdated { config_id: 1 })));
    }

    #[test]
    fn test_soft_delete() {
        let mut c = make_config();
        c.soft_delete(None);
        assert_eq!(c.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_soft_delete_raises_event() {
        let mut c = make_config();
        let before = c.events().len();
        c.soft_delete(None);
        assert_eq!(c.events().len(), before + 1);
        assert!(matches!(c.events().last(), Some(DomainEvent::ConfigDeleted { config_id: 1 })));
    }
}
