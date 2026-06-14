// ============================================================
// UNIT TESTS: DictTypeService + DictDataService (domain service, mocked repos)
// ============================================================

#[cfg(test)]
mod dict_type_service_tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_common::page::Page;
    use tx_error::AppResult;
    use crate::shared::model::AuditFields;
    use crate::dictionary::model::aggregate::DictType;
    use crate::dictionary::model::value_object::DictTypeQuery;
    use crate::dictionary::repository::DictTypeRepository;
    use crate::dictionary::service::DictTypeService;
    use pretty_assertions::assert_eq;

    // ----------------------------------------------------------
    // TestDictTypeRepo: function-closure based mock
    // ----------------------------------------------------------

    struct TestDictTypeRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<DictType>> + Send + Sync>,
        find_by_type_fn: Box<dyn Fn(&str) -> AppResult<Option<DictType>> + Send + Sync>,
        find_page_fn: Box<dyn Fn(&DictTypeQuery, Page<DictType>) -> AppResult<Page<DictType>> + Send + Sync>,
        find_all_fn: Box<dyn Fn(&DictTypeQuery) -> AppResult<Vec<DictType>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&DictType) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&DictType) -> AppResult<()> + Send + Sync>,
        soft_delete_fn: Box<dyn Fn(u64) -> AppResult<()> + Send + Sync>,
        exists_by_type_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
    }

    impl TestDictTypeRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_by_type_fn: Box::new(|_| panic!("unexpected call: find_by_type")),
                find_page_fn: Box::new(|_, _| panic!("unexpected call: find_page")),
                find_all_fn: Box::new(|_| panic!("unexpected call: find_all")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                update_fn: Box::new(|_| panic!("unexpected call: update")),
                soft_delete_fn: Box::new(|_| panic!("unexpected call: soft_delete")),
                exists_by_type_fn: Box::new(|_| panic!("unexpected call: exists_by_type")),
            }
        }
    }

    #[async_trait]
    impl DictTypeRepository for TestDictTypeRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<DictType>> {
            (self.find_by_id_fn)(id)
        }
        async fn find_by_type(&self, dict_type: &str) -> AppResult<Option<DictType>> {
            (self.find_by_type_fn)(dict_type)
        }
        async fn find_page(&self, query: &DictTypeQuery, page: Page<DictType>) -> AppResult<Page<DictType>> {
            (self.find_page_fn)(query, page)
        }
        async fn find_all(&self, query: &DictTypeQuery) -> AppResult<Vec<DictType>> {
            (self.find_all_fn)(query)
        }
        async fn insert(&self, dict_type: &DictType) -> AppResult<()> {
            (self.insert_fn)(dict_type)
        }
        async fn update(&self, dict_type: &DictType) -> AppResult<()> {
            (self.update_fn)(dict_type)
        }
        async fn soft_delete(&self, id: u64) -> AppResult<()> {
            (self.soft_delete_fn)(id)
        }
        async fn exists_by_type(&self, dict_type: &str) -> AppResult<bool> {
            (self.exists_by_type_fn)(dict_type)
        }
    }

    // ----------------------------------------------------------
    // Helper: create a sample DictType for testing
    // ----------------------------------------------------------

    fn make_dict_type() -> DictType {
        DictType::restore(
            1,
            "sys_status".into(),
            "sys_status".into(),
            0,
            None,
            AuditFields::default(),
        )
    }

    // ==========================================================
    // create_dict_type
    // ==========================================================

    #[tokio::test]
    async fn test_create_dict_type_success() {
        let mut repo = TestDictTypeRepo::new();
        repo.exists_by_type_fn = Box::new(|_| Ok(false));
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = DictTypeService::new(Arc::new(repo));
        let result = svc.create_dict_type("sys_status".into(), "sys_status".into(), Some("admin".into())).await;
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.name, "sys_status");
        assert_eq!(dt.dict_type, "sys_status");
    }

    #[tokio::test]
    async fn test_create_dict_type_duplicate() {
        let mut repo = TestDictTypeRepo::new();
        repo.exists_by_type_fn = Box::new(|_| Ok(true));

        let svc = DictTypeService::new(Arc::new(repo));
        let result = svc.create_dict_type("dup".into(), "dup".into(), None).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // update_dict_type
    // ==========================================================

    #[tokio::test]
    async fn test_update_dict_type_success() {
        let mut repo = TestDictTypeRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_dict_type())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = DictTypeService::new(Arc::new(repo));
        let result = svc.update_dict_type(
            1,
            "updated_name".into(),
            "updated_type".into(),
            Some("remark".into()),
            Some("admin".into()),
        ).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "updated_name");
    }

    #[tokio::test]
    async fn test_update_dict_type_not_found() {
        let mut repo = TestDictTypeRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = DictTypeService::new(Arc::new(repo));
        let result = svc.update_dict_type(999, "x".into(), "x".into(), None, None).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // delete_dict_type
    // ==========================================================

    #[tokio::test]
    async fn test_delete_dict_type_success() {
        let mut repo = TestDictTypeRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_dict_type())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = DictTypeService::new(Arc::new(repo));
        assert!(svc.delete_dict_type(1, Some("admin".into())).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_dict_type_not_found() {
        let mut repo = TestDictTypeRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = DictTypeService::new(Arc::new(repo));
        assert!(svc.delete_dict_type(999, None).await.is_err());
    }

    // ==========================================================
    // get_dict_type_page
    // ==========================================================

    #[tokio::test]
    async fn test_get_dict_type_page_success() {
        let mut repo = TestDictTypeRepo::new();
        let items = vec![make_dict_type()];
        repo.find_page_fn = Box::new(move |_, _| {
            Ok(Page::new(items.clone(), 1, 10, 1))
        });

        let svc = DictTypeService::new(Arc::new(repo));
        let query = DictTypeQuery::default();
        let result = svc.get_dict_type_page(&query, Page::request(1, 10)).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.list.len(), 1);
    }

    // ==========================================================
    // get_all_dict_types
    // ==========================================================

    #[tokio::test]
    async fn test_get_all_dict_types_success() {
        let mut repo = TestDictTypeRepo::new();
        repo.find_all_fn = Box::new(|_| Ok(vec![make_dict_type()]));

        let svc = DictTypeService::new(Arc::new(repo));
        let query = DictTypeQuery::default();
        let result = svc.get_all_dict_types(&query).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }
}

// ============================================================
// UNIT TESTS: DictDataService
// ============================================================

#[cfg(test)]
mod dict_data_service_tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_common::page::Page;
    use tx_error::AppResult;
    use crate::shared::model::AuditFields;
    use crate::dictionary::model::aggregate::DictData;
    use crate::dictionary::model::value_object::DictDataQuery;
    use crate::dictionary::repository::DictDataRepository;
    use crate::dictionary::service::DictDataService;
    use pretty_assertions::assert_eq;

    // ----------------------------------------------------------
    // TestDictDataRepo: function-closure based mock
    // ----------------------------------------------------------

    struct TestDictDataRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<DictData>> + Send + Sync>,
        find_by_type_fn: Box<dyn Fn(&str) -> AppResult<Vec<DictData>> + Send + Sync>,
        find_by_types_fn: Box<dyn Fn(&[String]) -> AppResult<Vec<DictData>> + Send + Sync>,
        find_page_fn: Box<dyn Fn(&DictDataQuery, Page<DictData>) -> AppResult<Page<DictData>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&DictData) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&DictData) -> AppResult<()> + Send + Sync>,
        soft_delete_fn: Box<dyn Fn(u64) -> AppResult<()> + Send + Sync>,
    }

    impl TestDictDataRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_by_type_fn: Box::new(|_| panic!("unexpected call: find_by_type")),
                find_by_types_fn: Box::new(|_| panic!("unexpected call: find_by_types")),
                find_page_fn: Box::new(|_, _| panic!("unexpected call: find_page")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                update_fn: Box::new(|_| panic!("unexpected call: update")),
                soft_delete_fn: Box::new(|_| panic!("unexpected call: soft_delete")),
            }
        }
    }

    #[async_trait]
    impl DictDataRepository for TestDictDataRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<DictData>> {
            (self.find_by_id_fn)(id)
        }
        async fn find_by_type(&self, dict_type: &str) -> AppResult<Vec<DictData>> {
            (self.find_by_type_fn)(dict_type)
        }
        async fn find_by_types(&self, dict_types: &[String]) -> AppResult<Vec<DictData>> {
            (self.find_by_types_fn)(dict_types)
        }
        async fn find_page(&self, query: &DictDataQuery, page: Page<DictData>) -> AppResult<Page<DictData>> {
            (self.find_page_fn)(query, page)
        }
        async fn insert(&self, data: &DictData) -> AppResult<()> {
            (self.insert_fn)(data)
        }
        async fn update(&self, data: &DictData) -> AppResult<()> {
            (self.update_fn)(data)
        }
        async fn soft_delete(&self, id: u64) -> AppResult<()> {
            (self.soft_delete_fn)(id)
        }
    }

    // ----------------------------------------------------------
    // Helper: create a sample DictData for testing
    // ----------------------------------------------------------

    fn make_dict_data() -> DictData {
        DictData::restore(
            1,
            1,
            "enabled".into(),
            "1".into(),
            "sys_status".into(),
            0,
            None,
            None,
            None,
            AuditFields::default(),
        )
    }

    // ==========================================================
    // create_dict_data
    // ==========================================================

    #[tokio::test]
    async fn test_create_dict_data_success() {
        let mut repo = TestDictDataRepo::new();
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = DictDataService::new(Arc::new(repo));
        let result = svc.create_dict_data(
            1, "enabled".into(), "1".into(), "sys_status".into(), Some("admin".into()),
        ).await;
        assert!(result.is_ok());
        let dd = result.unwrap();
        assert_eq!(dd.label, "enabled");
        assert_eq!(dd.value, "1");
        assert_eq!(dd.dict_type, "sys_status");
    }

    #[tokio::test]
    async fn test_create_dict_data_insert_error() {
        let mut repo = TestDictDataRepo::new();
        repo.insert_fn = Box::new(|_| Err(crate::shared::repository::RepositoryError::Duplicate)?);

        let svc = DictDataService::new(Arc::new(repo));
        let result = svc.create_dict_data(
            1, "dup".into(), "1".into(), "sys_status".into(), None,
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // update_dict_data
    // ==========================================================

    #[tokio::test]
    async fn test_update_dict_data_success() {
        let mut repo = TestDictDataRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_dict_data())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = DictDataService::new(Arc::new(repo));
        let result = svc.update_dict_data(
            1, 2, "updated_label".into(), "2".into(), "sys_status".into(),
            Some("primary".into()), Some("tag".into()), Some("remark".into()), Some("admin".into()),
        ).await;
        assert!(result.is_ok());
        let dd = result.unwrap();
        assert_eq!(dd.label, "updated_label");
        assert_eq!(dd.value, "2");
        assert_eq!(dd.color_type, Some("primary".into()));
    }

    #[tokio::test]
    async fn test_update_dict_data_not_found() {
        let mut repo = TestDictDataRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = DictDataService::new(Arc::new(repo));
        let result = svc.update_dict_data(
            999, 1, "x".into(), "x".into(), "x".into(), None, None, None, None,
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // delete_dict_data
    // ==========================================================

    #[tokio::test]
    async fn test_delete_dict_data_success() {
        let mut repo = TestDictDataRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_dict_data())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = DictDataService::new(Arc::new(repo));
        assert!(svc.delete_dict_data(1, Some("admin".into())).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_dict_data_not_found() {
        let mut repo = TestDictDataRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = DictDataService::new(Arc::new(repo));
        assert!(svc.delete_dict_data(999, None).await.is_err());
    }

    // ==========================================================
    // get_dict_data_page
    // ==========================================================

    #[tokio::test]
    async fn test_get_dict_data_page_success() {
        let mut repo = TestDictDataRepo::new();
        let items = vec![make_dict_data()];
        repo.find_page_fn = Box::new(move |_, _| {
            Ok(Page::new(items.clone(), 1, 10, 1))
        });

        let svc = DictDataService::new(Arc::new(repo));
        let query = DictDataQuery::default();
        let result = svc.get_dict_data_page(&query, Page::request(1, 10)).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.list.len(), 1);
    }

    // ==========================================================
    // get_by_dict_type
    // ==========================================================

    #[tokio::test]
    async fn test_get_by_dict_type_success() {
        let mut repo = TestDictDataRepo::new();
        repo.find_by_type_fn = Box::new(|_| Ok(vec![make_dict_data()]));

        let svc = DictDataService::new(Arc::new(repo));
        let result = svc.get_by_dict_type("sys_status").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_get_by_dict_type_empty() {
        let mut repo = TestDictDataRepo::new();
        repo.find_by_type_fn = Box::new(|_| Ok(vec![]));

        let svc = DictDataService::new(Arc::new(repo));
        let result = svc.get_by_dict_type("nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ==========================================================
    // get_by_dict_types
    // ==========================================================

    #[tokio::test]
    async fn test_get_by_dict_types_success() {
        let mut repo = TestDictDataRepo::new();
        repo.find_by_types_fn = Box::new(|_| {
            Ok(vec![
                make_dict_data(),
                DictData::restore(
                    2, 2, "disabled".into(), "0".into(), "sys_status".into(),
                    0, None, None, None, AuditFields::default(),
                ),
            ])
        });

        let svc = DictDataService::new(Arc::new(repo));
        let result = svc.get_by_dict_types(&["sys_status".into()]).await;
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("sys_status").unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_dict_types_empty() {
        let mut repo = TestDictDataRepo::new();
        repo.find_by_types_fn = Box::new(|_| Ok(vec![]));

        let svc = DictDataService::new(Arc::new(repo));
        let result = svc.get_by_dict_types(&["nonexistent".into()]).await;
        assert!(result.is_ok());
        let map: HashMap<String, Vec<DictData>> = result.unwrap();
        assert!(map.is_empty());
    }
}
