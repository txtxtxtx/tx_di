// ============================================================
// UNIT TESTS: OperateLogService + LoginLogService (domain service, mocked repos)
// ============================================================

#[cfg(test)]
mod operate_log_service_tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_common::page::Page;
    use tx_error::AppResult;
    use crate::shared::model::AuditFields;
    use crate::log::model::aggregate::OperateLog;
    use crate::log::model::value_object::OperateLogQuery;
    use crate::log::repository::OperateLogRepository;
    use crate::log::service::OperateLogService;
    use pretty_assertions::assert_eq;

    // ----------------------------------------------------------
    // TestOperateLogRepo: function-closure based mock
    // ----------------------------------------------------------

    struct TestOperateLogRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<OperateLog>> + Send + Sync>,
        find_page_fn: Box<dyn Fn(&OperateLogQuery, Page<OperateLog>) -> AppResult<Page<OperateLog>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&OperateLog) -> AppResult<()> + Send + Sync>,
        delete_by_ids_fn: Box<dyn Fn(&[u64]) -> AppResult<()> + Send + Sync>,
        clean_all_fn: Box<dyn Fn() -> AppResult<()> + Send + Sync>,
    }

    impl TestOperateLogRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_page_fn: Box::new(|_, _| panic!("unexpected call: find_page")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                delete_by_ids_fn: Box::new(|_| panic!("unexpected call: delete_by_ids")),
                clean_all_fn: Box::new(|| panic!("unexpected call: clean_all")),
            }
        }
    }

    #[async_trait]
    impl OperateLogRepository for TestOperateLogRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<OperateLog>> {
            (self.find_by_id_fn)(id)
        }
        async fn find_page(&self, query: &OperateLogQuery, page: Page<OperateLog>) -> AppResult<Page<OperateLog>> {
            (self.find_page_fn)(query, page)
        }
        async fn insert(&self, log: &OperateLog) -> AppResult<()> {
            (self.insert_fn)(log)
        }
        async fn delete_by_ids(&self, ids: &[u64]) -> AppResult<()> {
            (self.delete_by_ids_fn)(ids)
        }
        async fn clean_all(&self) -> AppResult<()> {
            (self.clean_all_fn)()
        }
    }

    // ----------------------------------------------------------
    // Helper: create a sample OperateLog for testing
    // ----------------------------------------------------------

    fn make_operate_log() -> OperateLog {
        OperateLog::restore(
            1,
            "trace-001".into(),
            100,
            1,
            "user".into(),
            "create".into(),
            42,
            "create_user".into(),
            1,
            "{}".into(),
            Some("POST".into()),
            Some("/api/users".into()),
            Some("127.0.0.1".into()),
            Some("test-agent".into()),
            0,
            AuditFields::default(),
        )
    }

    // ==========================================================
    // create_log
    // ==========================================================

    #[tokio::test]
    async fn test_create_operate_log_success() {
        let mut repo = TestOperateLogRepo::new();
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = OperateLogService::new(Arc::new(repo));
        let result = svc.create_log(
            "trace-001".into(), 100, 1, "user".into(), "create".into(),
            42, "create_user".into(), 1, "{}".into(),
        ).await;
        assert!(result.is_ok());
        let log = result.unwrap();
        assert_eq!(log.trace_id, "trace-001");
        assert_eq!(log.user_id, 100);
        assert_eq!(log.action, "create_user");
        assert_eq!(log.success, 1);
    }

    #[tokio::test]
    async fn test_create_operate_log_insert_error() {
        let mut repo = TestOperateLogRepo::new();
        repo.insert_fn = Box::new(|_| Err(crate::shared::repository::RepositoryError::DatabaseLog.into()));

        let svc = OperateLogService::new(Arc::new(repo));
        let result = svc.create_log(
            "t".into(), 1, 1, "x".into(), "x".into(), 1, "x".into(), 0, "".into(),
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // get_log_page
    // ==========================================================

    #[tokio::test]
    async fn test_get_operate_log_page_success() {
        let mut repo = TestOperateLogRepo::new();
        let items = vec![make_operate_log()];
        repo.find_page_fn = Box::new(move |_, _| {
            Ok(Page::new(items.clone(), 1, 10, 1))
        });

        let svc = OperateLogService::new(Arc::new(repo));
        let query = OperateLogQuery::default();
        let result = svc.get_log_page(&query, Page::request(1, 10)).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.list.len(), 1);
    }

    // ==========================================================
    // delete_logs
    // ==========================================================

    #[tokio::test]
    async fn test_delete_operate_logs_success() {
        let mut repo = TestOperateLogRepo::new();
        repo.delete_by_ids_fn = Box::new(|_| Ok(()));

        let svc = OperateLogService::new(Arc::new(repo));
        assert!(svc.delete_logs(&[1, 2, 3]).await.is_ok());
    }

    // ==========================================================
    // clean_logs
    // ==========================================================

    #[tokio::test]
    async fn test_clean_operate_logs_success() {
        let mut repo = TestOperateLogRepo::new();
        repo.clean_all_fn = Box::new(|| Ok(()));

        let svc = OperateLogService::new(Arc::new(repo));
        assert!(svc.clean_logs().await.is_ok());
    }
}

// ============================================================
// UNIT TESTS: LoginLogService
// ============================================================

#[cfg(test)]
mod login_log_service_tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use jiff::Timestamp;
    use tx_common::page::Page;
    use tx_error::AppResult;
    use crate::shared::model::AuditFields;
    use crate::log::model::aggregate::LoginLog;
    use crate::log::model::value_object::LoginLogQuery;
    use crate::log::repository::LoginLogRepository;
    use crate::log::service::LoginLogService;
    use pretty_assertions::assert_eq;

    // ----------------------------------------------------------
    // TestLoginLogRepo: function-closure based mock
    // ----------------------------------------------------------

    struct TestLoginLogRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<LoginLog>> + Send + Sync>,
        find_page_fn: Box<dyn Fn(&LoginLogQuery, Page<LoginLog>) -> AppResult<Page<LoginLog>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&LoginLog) -> AppResult<()> + Send + Sync>,
        delete_by_ids_fn: Box<dyn Fn(&[u64]) -> AppResult<()> + Send + Sync>,
        clean_all_fn: Box<dyn Fn() -> AppResult<()> + Send + Sync>,
    }

    impl TestLoginLogRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_page_fn: Box::new(|_, _| panic!("unexpected call: find_page")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                delete_by_ids_fn: Box::new(|_| panic!("unexpected call: delete_by_ids")),
                clean_all_fn: Box::new(|| panic!("unexpected call: clean_all")),
            }
        }
    }

    #[async_trait]
    impl LoginLogRepository for TestLoginLogRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<LoginLog>> {
            (self.find_by_id_fn)(id)
        }
        async fn find_page(&self, query: &LoginLogQuery, page: Page<LoginLog>) -> AppResult<Page<LoginLog>> {
            (self.find_page_fn)(query, page)
        }
        async fn insert(&self, log: &LoginLog) -> AppResult<()> {
            (self.insert_fn)(log)
        }
        async fn delete_by_ids(&self, ids: &[u64]) -> AppResult<()> {
            (self.delete_by_ids_fn)(ids)
        }
        async fn clean_all(&self) -> AppResult<()> {
            (self.clean_all_fn)()
        }
    }

    // ----------------------------------------------------------
    // Helper: create a sample LoginLog for testing
    // ----------------------------------------------------------

    fn make_login_log() -> LoginLog {
        LoginLog::restore(
            1,
            100,
            1,
            "admin".into(),
            "192.168.1.1".into(),
            Some("Beijing".into()),
            Some("Chrome".into()),
            Some("Windows".into()),
            "password".into(),
            1,
            None,
            Timestamp::now(),
            0,
            AuditFields::default(),
        )
    }

    // ==========================================================
    // create_log
    // ==========================================================

    #[tokio::test]
    async fn test_create_login_log_success() {
        let mut repo = TestLoginLogRepo::new();
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = LoginLogService::new(Arc::new(repo));
        let result = svc.create_log(
            100, 1, "admin".into(), "192.168.1.1".into(), "password".into(), 1,
        ).await;
        assert!(result.is_ok());
        let log = result.unwrap();
        assert_eq!(log.user_id, 100);
        assert_eq!(log.username, "admin");
        assert_eq!(log.login_ip, "192.168.1.1");
        assert_eq!(log.login_type, "password");
        assert_eq!(log.result, 1);
    }

    #[tokio::test]
    async fn test_create_login_log_insert_error() {
        let mut repo = TestLoginLogRepo::new();
        repo.insert_fn = Box::new(|_| Err(crate::shared::repository::RepositoryError::DatabaseLog.into()));

        let svc = LoginLogService::new(Arc::new(repo));
        let result = svc.create_log(
            1, 1, "user".into(), "127.0.0.1".into(), "password".into(), 0,
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // get_log_page
    // ==========================================================

    #[tokio::test]
    async fn test_get_login_log_page_success() {
        let mut repo = TestLoginLogRepo::new();
        let items = vec![make_login_log()];
        repo.find_page_fn = Box::new(move |_, _| {
            Ok(Page::new(items.clone(), 1, 10, 1))
        });

        let svc = LoginLogService::new(Arc::new(repo));
        let query = LoginLogQuery::default();
        let result = svc.get_log_page(&query, Page::request(1, 10)).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.list.len(), 1);
    }

    // ==========================================================
    // delete_logs
    // ==========================================================

    #[tokio::test]
    async fn test_delete_login_logs_success() {
        let mut repo = TestLoginLogRepo::new();
        repo.delete_by_ids_fn = Box::new(|_| Ok(()));

        let svc = LoginLogService::new(Arc::new(repo));
        assert!(svc.delete_logs(&[1, 2, 3]).await.is_ok());
    }

    // ==========================================================
    // clean_logs
    // ==========================================================

    #[tokio::test]
    async fn test_clean_login_logs_success() {
        let mut repo = TestLoginLogRepo::new();
        repo.clean_all_fn = Box::new(|| Ok(()));

        let svc = LoginLogService::new(Arc::new(repo));
        assert!(svc.clean_logs().await.is_ok());
    }
}
