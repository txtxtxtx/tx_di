// ============================================================
// UNIT TESTS: Dictionary (DictType + DictData) 聚合根
// Coverage: create, update_info, change_status, soft_delete
// ============================================================

#[cfg(test)]
mod dict_tests {
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::model::AggregateRoot;
    use crate::dictionary::model::aggregate::{DictType, DictData};
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    // ── DictType ───────────────────────────────────────

    fn make_dict_type() -> DictType {
        DictType::create(1, "Gender".into(), "sys_gender".into(), Some("admin".into()))
    }

    #[test]
    fn test_create_dict_type_sets_fields() {
        let dt = make_dict_type();
        assert_eq!(dt.id, 1);
        assert_eq!(dt.name, "Gender");
        assert_eq!(dt.dict_type, "sys_gender");
        assert_eq!(dt.status, 0);
    }

    #[test]
    fn test_create_dict_type_raises_event() {
        let dt = DictType::create(2, "Status".into(), "sys_status".into(), None);
        assert_eq!(dt.events().len(), 1);
        assert!(matches!(dt.events()[0], DomainEvent::DictTypeCreated { dict_type_id: 2 }));
    }

    #[test]
    fn test_update_dict_type_info() {
        let mut dt = make_dict_type();
        dt.update_info("Sex".into(), "sys_sex".into(), Some("remark".into()), Some("updater".into()));
        assert_eq!(dt.name, "Sex");
        assert_eq!(dt.dict_type, "sys_sex");
    }

    #[test]
    fn test_update_dict_type_raises_event() {
        let mut dt = make_dict_type();
        let before = dt.events().len();
        dt.update_info("X".into(), "x".into(), None, None);
        assert_eq!(dt.events().len(), before + 1);
        assert!(matches!(dt.events().last(), Some(DomainEvent::DictTypeUpdated { dict_type_id: 1 })));
    }

    #[test]
    fn test_dict_type_change_status() {
        let mut dt = make_dict_type();
        dt.change_status(1, Some("admin".into()));
        assert_eq!(dt.status, 1);
    }

    #[test]
    fn test_dict_type_soft_delete() {
        let mut dt = make_dict_type();
        dt.soft_delete(None);
        assert_eq!(dt.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_dict_type_soft_delete_raises_event() {
        let mut dt = make_dict_type();
        let before = dt.events().len();
        dt.soft_delete(None);
        assert_eq!(dt.events().len(), before + 1);
        assert!(matches!(dt.events().last(), Some(DomainEvent::DictTypeDeleted { dict_type_id: 1 })));
    }

    // ── DictData ───────────────────────────────────────

    fn make_dict_data() -> DictData {
        DictData::create(100, 1, "Male".into(), "1".into(), "sys_gender".into(), Some("admin".into()))
    }

    #[test]
    fn test_create_dict_data_sets_fields() {
        let dd = make_dict_data();
        assert_eq!(dd.id, 100);
        assert_eq!(dd.label, "Male");
        assert_eq!(dd.value, "1");
        assert_eq!(dd.dict_type, "sys_gender");
        assert_eq!(dd.sort, 1);
        assert_eq!(dd.status, 0);
    }

    #[test]
    fn test_create_dict_data_raises_event() {
        let dd = DictData::create(200, 1, "Female".into(), "2".into(), "sys_gender".into(), None);
        assert_eq!(dd.events().len(), 1);
        assert!(matches!(dd.events()[0], DomainEvent::DictDataCreated { dict_data_id: 200 }));
    }

    #[test]
    fn test_update_dict_data_info() {
        let mut dd = make_dict_data();
        dd.update_info(2, "Man".into(), "M".into(), "sys_gender".into(),
            Some("blue".into()), Some("bold".into()), Some("remark".into()), Some("updater".into()));
        assert_eq!(dd.sort, 2);
        assert_eq!(dd.label, "Man");
        assert_eq!(dd.value, "M");
        assert_eq!(dd.color_type.as_deref(), Some("blue"));
    }

    #[test]
    fn test_update_dict_data_raises_event() {
        let mut dd = make_dict_data();
        let before = dd.events().len();
        dd.update_info(0, "X".into(), "X".into(), "X".into(), None, None, None, None);
        assert_eq!(dd.events().len(), before + 1);
        assert!(matches!(dd.events().last(), Some(DomainEvent::DictDataUpdated { dict_data_id: 100 })));
    }

    #[test]
    fn test_dict_data_change_status() {
        let mut dd = make_dict_data();
        dd.change_status(1, None);
        assert_eq!(dd.status, 1);
    }

    #[test]
    fn test_dict_data_soft_delete() {
        let mut dd = make_dict_data();
        dd.soft_delete(None);
        assert_eq!(dd.audit.deleted, DeletedStatus::Deleted);
    }

    #[test]
    fn test_dict_data_soft_delete_raises_event() {
        let mut dd = make_dict_data();
        let before = dd.events().len();
        dd.soft_delete(None);
        assert_eq!(dd.events().len(), before + 1);
        assert!(matches!(dd.events().last(), Some(DomainEvent::DictDataDeleted { dict_data_id: 100 })));
    }
}
