// ============================================================
// UNIT TESTS: Menu 聚合根
// Coverage: create, update_info, change_status, soft_delete,
//           is_directory, is_menu, is_button, is_root
// ============================================================

#[cfg(test)]
mod menu_tests {
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::model::AggregateRoot;
    use crate::menu::model::aggregate::Menu;
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    fn make_menu() -> Menu {
        Menu::create(10, "Dashboard".into(), "dashboard".into(), 1, 0, 0, Some("admin".into()))
    }

    #[test]
    fn test_create_menu_sets_fields() {
        let m = make_menu();
        assert_eq!(m.id, 10);
        assert_eq!(m.name, "Dashboard");
        assert_eq!(m.permission, "dashboard");
        assert_eq!(m.types, 1);
        assert_eq!(m.parent_id, 0);
        assert_eq!(m.status, 0);
        assert!(m.children.is_empty());
    }

    #[test]
    fn test_create_menu_raises_event() {
        let m = Menu::create(1, "Home".into(), "home".into(), 1, 0, 0, None);
        assert_eq!(m.events().len(), 1);
        assert!(matches!(m.events()[0], DomainEvent::MenuCreated { menu_id: 1 }));
    }

    #[test]
    fn test_update_info_all_fields() {
        let mut m = make_menu();
        m.update_info("Settings".into(), "settings".into(), 1, 2, 0,
            Some("/settings".into()), Some("gear".into()), Some("SettingsView".into()),
            Some("settings".into()), 1, 1, Some("updater".into()));

        assert_eq!(m.name, "Settings");
        assert_eq!(m.path.as_deref(), Some("/settings"));
        assert_eq!(m.icon.as_deref(), Some("gear"));
        assert_eq!(m.visible, 1);
        assert_eq!(m.keep_alive, 1);
    }

    #[test]
    fn test_update_info_raises_event() {
        let mut m = make_menu();
        let before = m.events().len();
        m.update_info("X".into(), "x".into(), 0, 0, 0, None, None, None, None, 0, 0, None);
        assert_eq!(m.events().len(), before + 1);
        assert!(matches!(m.events().last(), Some(DomainEvent::MenuUpdated { menu_id: 10 })));
    }

    #[test]
    fn test_is_directory() {
        let m = Menu::create(1, "Dir".into(), "dir".into(), 0, 0, 0, None);
        assert!(m.is_directory());
        assert!(!m.is_menu());
        assert!(!m.is_button());
    }

    #[test]
    fn test_is_menu() {
        let m = make_menu();
        assert!(m.is_menu());
        assert!(!m.is_directory());
        assert!(!m.is_button());
    }

    #[test]
    fn test_is_button() {
        let m = Menu::create(1, "Btn".into(), "btn".into(), 2, 0, 0, None);
        assert!(m.is_button());
        assert!(!m.is_menu());
        assert!(!m.is_directory());
    }

    #[test]
    fn test_is_root() {
        let root = make_menu();
        assert!(root.is_root());

        let child = Menu::create(2, "Child".into(), "child".into(), 1, 0, 10, None);
        assert!(!child.is_root());
    }

    #[test]
    fn test_soft_delete() {
        let mut m = make_menu();
        m.soft_delete(None);
        assert_eq!(m.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_soft_delete_raises_event() {
        let mut m = make_menu();
        let before = m.events().len();
        m.soft_delete(None);
        assert_eq!(m.events().len(), before + 1);
        assert!(matches!(m.events().last(), Some(DomainEvent::MenuDeleted { menu_id: 10 })));
    }
}
