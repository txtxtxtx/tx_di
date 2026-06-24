// ============================================================
// UNIT TESTS: UserService (domain service, mocked repos)
// 重构后 UserService 只依赖 UserRepository。
// 跨聚合测试（assign_roles/assign_departments/build_login_user）
// 已移至 Application 层 UserAppService 测试。
// ============================================================

#[cfg(test)]
mod user_service_tests {
    use std::sync::Arc;
    use tx_common::page::Page;
    use tx_error::AppResult;
    use crate::user::model::aggregate::User;
    use crate::user::model::value_object::{UserQuery, UserStatus};
    use crate::user::service::UserService;
    use pretty_assertions::assert_eq;

    use async_trait::async_trait;
    use crate::user::repository::UserRepository;

    struct TestUserRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<User>> + Send + Sync>,
        find_by_username_fn: Box<dyn Fn(&str) -> AppResult<Option<User>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&User) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&User) -> AppResult<()> + Send + Sync>,
        exists_by_username_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
        exists_by_email_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
        exists_by_mobile_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
        get_role_ids_fn: Box<dyn Fn(u64) -> AppResult<Vec<u64>> + Send + Sync>,
        get_dept_ids_fn: Box<dyn Fn(u64) -> AppResult<Vec<u64>> + Send + Sync>,
        find_page_fn: Box<dyn Fn(&UserQuery, Page<User>) -> AppResult<Page<User>> + Send + Sync>,
    }

    impl TestUserRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call")),
                find_by_username_fn: Box::new(|_| panic!("unexpected call")),
                insert_fn: Box::new(|_| panic!("unexpected call")),
                update_fn: Box::new(|_| panic!("unexpected call")),
                exists_by_username_fn: Box::new(|_| panic!("unexpected call")),
                exists_by_email_fn: Box::new(|_| panic!("unexpected call")),
                exists_by_mobile_fn: Box::new(|_| panic!("unexpected call")),
                get_role_ids_fn: Box::new(|_| panic!("unexpected call")),
                get_dept_ids_fn: Box::new(|_| panic!("unexpected call")),
                find_page_fn: Box::new(|_, _| panic!("unexpected call")),
            }
        }
    }

    #[async_trait]
    impl UserRepository for TestUserRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<User>> { (self.find_by_id_fn)(id) }
        async fn find_by_username(&self, u: &str) -> AppResult<Option<User>> { (self.find_by_username_fn)(u) }
        async fn find_page(&self, q: &UserQuery, p: Page<User>) -> AppResult<Page<User>> { (self.find_page_fn)(q, p) }
        async fn find_all(&self, _: &UserQuery) -> AppResult<Vec<User>> { Ok(vec![]) }
        async fn insert(&self, u: &User) -> AppResult<()> { (self.insert_fn)(u) }
        async fn update(&self, u: &User) -> AppResult<()> { (self.update_fn)(u) }
        async fn soft_delete(&self, _: u64) -> AppResult<()> { Ok(()) }
        async fn exists_by_username(&self, u: &str) -> AppResult<bool> { (self.exists_by_username_fn)(u) }
        async fn exists_by_email(&self, e: &str) -> AppResult<bool> { (self.exists_by_email_fn)(e) }
        async fn exists_by_mobile(&self, m: &str) -> AppResult<bool> { (self.exists_by_mobile_fn)(m) }
        async fn count(&self, _: &UserQuery) -> AppResult<i64> { Ok(0) }
        async fn find_by_role_id(&self, _: u64) -> AppResult<Vec<User>> { Ok(vec![]) }
        async fn find_by_dept_id(&self, _: u64) -> AppResult<Vec<User>> { Ok(vec![]) }
        async fn bind_roles(&self, _: u64, _: &[u64]) -> AppResult<()> { Ok(()) }
        async fn bind_departments(&self, _: u64, _: &[u64]) -> AppResult<()> { Ok(()) }
        async fn get_role_ids(&self, uid: u64) -> AppResult<Vec<u64>> { (self.get_role_ids_fn)(uid) }
        async fn get_dept_ids(&self, uid: u64) -> AppResult<Vec<u64>> { (self.get_dept_ids_fn)(uid) }
    }

    fn make_user() -> User {
        User::create(1, "testuser".into(), "pwd".into(), "Test".into(), None)
    }

    // ── create_user ──

    /// 创建用户成功：username 不重复 + insert 成功。
    #[tokio::test]
    async fn test_create_user_success() {
        let mut repo = TestUserRepo::new();
        repo.exists_by_username_fn = Box::new(|_| Ok(false));
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(Arc::new(repo));
        assert!(svc.create_user("new".into(), "p".into(), "N".into(), None).await.is_ok());
    }

    /// 用户名重复时 create_user 应失败。
    #[tokio::test]
    async fn test_create_user_duplicate() {
        let mut repo = TestUserRepo::new();
        repo.exists_by_username_fn = Box::new(|_| Ok(true));

        let svc = UserService::new(Arc::new(repo));
        assert!(svc.create_user("dup".into(), "p".into(), "N".into(), None).await.is_err());
    }

    // ── update_user ──

    /// 更新用户成功，校验 nickname 已变更。
    #[tokio::test]
    async fn test_update_user_success() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(Arc::new(repo));
        let r = svc.update_user(1, "New".into(), None, None,
            crate::user::model::value_object::Sex::Unknown, None, None).await;
        assert!(r.is_ok());
        assert_eq!(r.unwrap().nickname, "New");
    }

    /// 更新不存在的用户应失败。
    #[tokio::test]
    async fn test_update_user_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(Arc::new(repo));
        assert!(svc.update_user(999, "X".into(), None, None,
            crate::user::model::value_object::Sex::Unknown, None, None).await.is_err());
    }

    // ── delete_user ──

    /// 软删除成功。
    #[tokio::test]
    async fn test_delete_user() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(Arc::new(repo));
        assert!(svc.delete_user(1, None).await.is_ok());
    }

    /// 删除不存在的用户应失败。
    #[tokio::test]
    async fn test_delete_user_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(Arc::new(repo));
        assert!(svc.delete_user(999, None).await.is_err());
    }

    // ── change_status ──

    /// 变更状态成功，校验 status 已更新。
    #[tokio::test]
    async fn test_change_status() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(Arc::new(repo));
        let r = svc.change_status(1, UserStatus::Locked, None).await;
        assert!(r.is_ok());
        assert_eq!(r.unwrap().status, UserStatus::Locked);
    }

    // ── get_user ──

    /// 获取用户时填充 role_ids + dept_ids。
    #[tokio::test]
    async fn test_get_user_with_associations() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.get_role_ids_fn = Box::new(|_| Ok(vec![1, 2]));
        repo.get_dept_ids_fn = Box::new(|_| Ok(vec![10]));

        let svc = UserService::new(Arc::new(repo));
        let u = svc.get_user(1).await.unwrap();
        assert_eq!(u.role_ids, vec![1, 2]);
        assert_eq!(u.dept_ids, vec![10]);
    }

    /// 查询不存在的用户应失败。
    #[tokio::test]
    async fn test_get_user_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(Arc::new(repo));
        assert!(svc.get_user(999).await.is_err());
    }

    // ── record_login ──

    /// 记录登录 IP，校验 login_ip 已设置。
    #[tokio::test]
    async fn test_record_login() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(Arc::new(repo));
        let u = svc.record_login(1, "10.0.0.1".into()).await.unwrap();
        assert_eq!(u.login_ip.as_deref(), Some("10.0.0.1"));
    }

    // ── exists_by_email ──

    /// email 已存在返回 true。
    #[tokio::test]
    async fn test_exists_by_email() {
        let mut repo = TestUserRepo::new();
        repo.exists_by_email_fn = Box::new(|_| Ok(true));
        let svc = UserService::new(Arc::new(repo));
        assert!(svc.exists_by_email("x@y.com").await.unwrap());
    }

    // ── get_by_username ──

    /// 按用户名查询：存在。
    #[tokio::test]
    async fn test_get_by_username_found() {
        let mut repo = TestUserRepo::new();
        let user = make_user();
        repo.find_by_username_fn = Box::new(move |_| Ok(Some(user.clone())));

        let svc = UserService::new(Arc::new(repo));
        let result = svc.get_by_username("testuser").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().username, "testuser");
    }

    /// 按用户名查询：不存在。
    #[tokio::test]
    async fn test_get_by_username_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_username_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(Arc::new(repo));
        let result = svc.get_by_username("nobody").await.unwrap();
        assert!(result.is_none());
    }
}
