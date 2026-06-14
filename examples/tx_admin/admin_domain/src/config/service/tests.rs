// ============================================================
// UNIT TESTS: ConfigService (domain service, mocked repos)
// ============================================================

#[cfg(test)]
mod config_service_tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_common::page::Page;
    use tx_error::AppResult;
    use crate::config::model::aggregate::Config;
    use crate::config::model::value_object::ConfigQuery;
    use crate::config::repository::ConfigRepository;
    use crate::config::service::ConfigService;
    use crate::shared::model::AuditFields;
    use pretty_assertions::assert_eq;

    // ----------------------------------------------------------
    // TestConfigRepo: function-closure based mock
    // ----------------------------------------------------------

    struct TestConfigRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<Config>> + Send + Sync>,
        find_by_key_fn: Box<dyn Fn(&str) -> AppResult<Option<Config>> + Send + Sync>,
        find_by_keys_fn: Box<dyn Fn(&[String]) -> AppResult<Vec<Config>> + Send + Sync>,
        find_page_fn: Box<dyn Fn(&ConfigQuery, Page<Config>) -> AppResult<Page<Config>> + Send + Sync>,
        find_all_fn: Box<dyn Fn(&ConfigQuery) -> AppResult<Vec<Config>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&Config) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&Config) -> AppResult<()> + Send + Sync>,
        exists_by_key_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
    }

    impl TestConfigRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_by_key_fn: Box::new(|_| panic!("unexpected call: find_by_key")),
                find_by_keys_fn: Box::new(|_| panic!("unexpected call: find_by_keys")),
                find_page_fn: Box::new(|_, _| panic!("unexpected call: find_page")),
                find_all_fn: Box::new(|_| panic!("unexpected call: find_all")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                update_fn: Box::new(|_| panic!("unexpected call: update")),
                exists_by_key_fn: Box::new(|_| panic!("unexpected call: exists_by_key")),
            }
        }
    }

    #[async_trait]
    impl ConfigRepository for TestConfigRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<Config>> {
            (self.find_by_id_fn)(id)
        }
        async fn find_by_key(&self, key: &str) -> AppResult<Option<Config>> {
            (self.find_by_key_fn)(key)
        }
        async fn find_by_keys(&self, keys: &[String]) -> AppResult<Vec<Config>> {
            (self.find_by_keys_fn)(keys)
        }
        async fn find_page(&self, query: &ConfigQuery, page: Page<Config>) -> AppResult<Page<Config>> {
            (self.find_page_fn)(query, page)
        }
        async fn find_all(&self, query: &ConfigQuery) -> AppResult<Vec<Config>> {
            (self.find_all_fn)(query)
        }
        async fn insert(&self, config: &Config) -> AppResult<()> {
            (self.insert_fn)(config)
        }
        async fn update(&self, config: &Config) -> AppResult<()> {
            (self.update_fn)(config)
        }
        async fn soft_delete(&self, _id: u64) -> AppResult<()> {
            Ok(())
        }
        async fn exists_by_key(&self, key: &str) -> AppResult<bool> {
            (self.exists_by_key_fn)(key)
        }
    }

    // ----------------------------------------------------------
    // Helper: create a sample Config for testing
    // ----------------------------------------------------------

    fn make_config() -> Config {
        Config::restore(
            1,
            "system".into(),
            1,
            "Site Name".into(),
            "site.name".into(),
            "My App".into(),
            1,
            Some("The site name".into()),
            AuditFields::default(),
        )
    }

    // ==========================================================
    // create_config
    // ==========================================================

    #[tokio::test]
    async fn test_create_config_success() {
        let mut repo = TestConfigRepo::new();
        repo.exists_by_key_fn = Box::new(|_| Ok(false));
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = ConfigService::new(Arc::new(repo));
        let result = svc.create_config(
            "system".into(),
            1,
            "Site Name".into(),
            "site.name".into(),
            "My App".into(),
            Some("admin".into()),
        ).await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.name, "Site Name");
        assert_eq!(config.config_key, "site.name");
        assert_eq!(config.value, "My App");
    }

    #[tokio::test]
    async fn test_create_config_duplicate_key() {
        let mut repo = TestConfigRepo::new();
        repo.exists_by_key_fn = Box::new(|_| Ok(true));

        let svc = ConfigService::new(Arc::new(repo));
        let result = svc.create_config(
            "system".into(),
            1,
            "Dup".into(),
            "site.name".into(),
            "val".into(),
            None,
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // update_config
    // ==========================================================

    #[tokio::test]
    async fn test_update_config_success() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_config())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = ConfigService::new(Arc::new(repo));
        let result = svc.update_config(
            1,
            "system".into(),
            1,
            "Site Name Updated".into(),
            "site.name".into(),
            "New Value".into(),
            1,
            Some("updated remark".into()),
            Some("admin".into()),
        ).await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.name, "Site Name Updated");
        assert_eq!(config.value, "New Value");
    }

    #[tokio::test]
    async fn test_update_config_not_found() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = ConfigService::new(Arc::new(repo));
        let result = svc.update_config(
            999,
            "system".into(),
            1,
            "X".into(),
            "x".into(),
            "v".into(),
            1,
            None,
            None,
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // delete_config
    // ==========================================================

    #[tokio::test]
    async fn test_delete_config_success() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_config())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = ConfigService::new(Arc::new(repo));
        assert!(svc.delete_config(1, Some("admin".into())).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_config_not_found() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = ConfigService::new(Arc::new(repo));
        assert!(svc.delete_config(999, None).await.is_err());
    }

    // ==========================================================
    // get_config_page
    // ==========================================================

    #[tokio::test]
    async fn test_get_config_page_success() {
        let mut repo = TestConfigRepo::new();
        let configs = vec![make_config()];
        repo.find_page_fn = Box::new(move |_, _| {
            Ok(Page::new(configs.clone(), 1, 10, 1))
        });

        let svc = ConfigService::new(Arc::new(repo));
        let query = ConfigQuery::default();
        let result = svc.get_config_page(&query, Page::request(1, 10)).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.list.len(), 1);
    }

    // ==========================================================
    // get_config
    // ==========================================================

    #[tokio::test]
    async fn test_get_config_success() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_config())));

        let svc = ConfigService::new(Arc::new(repo));
        let result = svc.get_config(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().config_key, "site.name");
    }

    #[tokio::test]
    async fn test_get_config_not_found() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = ConfigService::new(Arc::new(repo));
        assert!(svc.get_config(999).await.is_err());
    }

    // ==========================================================
    // get_by_key
    // ==========================================================

    #[tokio::test]
    async fn test_get_by_key_success() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_key_fn = Box::new(|_| Ok(Some(make_config())));

        let svc = ConfigService::new(Arc::new(repo));
        let result = svc.get_by_key("site.name").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, "My App");
    }

    #[tokio::test]
    async fn test_get_by_key_not_found() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_key_fn = Box::new(|_| Ok(None));

        let svc = ConfigService::new(Arc::new(repo));
        assert!(svc.get_by_key("nonexistent.key").await.is_err());
    }

    // ==========================================================
    // get_by_keys
    // ==========================================================

    #[tokio::test]
    async fn test_get_by_keys_success() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_keys_fn = Box::new(|_| {
            Ok(vec![
                Config::restore(
                    1,
                    "system".into(),
                    1,
                    "Site Name".into(),
                    "site.name".into(),
                    "My App".into(),
                    1,
                    None,
                    AuditFields::default(),
                ),
                Config::restore(
                    2,
                    "system".into(),
                    1,
                    "Site URL".into(),
                    "site.url".into(),
                    "https://example.com".into(),
                    1,
                    None,
                    AuditFields::default(),
                ),
            ])
        });

        let svc = ConfigService::new(Arc::new(repo));
        let keys = vec!["site.name".to_string(), "site.url".to_string()];
        let result = svc.get_by_keys(&keys).await;
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("site.name").unwrap(), "My App");
        assert_eq!(map.get("site.url").unwrap(), "https://example.com");
    }

    #[tokio::test]
    async fn test_get_by_keys_empty() {
        let mut repo = TestConfigRepo::new();
        repo.find_by_keys_fn = Box::new(|_| Ok(vec![]));

        let svc = ConfigService::new(Arc::new(repo));
        let keys: Vec<String> = vec![];
        let result = svc.get_by_keys(&keys).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
