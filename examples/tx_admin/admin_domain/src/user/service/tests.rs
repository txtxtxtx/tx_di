// ============================================================
// UNIT TESTS: UserService (domain service, mocked repos)
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

    // Manually create a mock UserRepository using a test struct
    use async_trait::async_trait;
    use crate::user::repository::UserRepository;
    use crate::role::repository::RoleRepository;
    use crate::role::model::aggregate::Role;
    use crate::role::model::value_object::RoleQuery;
    use crate::department::repository::DepartmentRepository;
    use crate::department::model::aggregate::Department;
    use crate::department::model::value_object::DeptQuery;
    use crate::menu::repository::MenuRepository;
    use crate::menu::model::aggregate::Menu;
    use crate::menu::model::value_object::MenuQuery;

    struct TestUserRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<User>> + Send + Sync>,
        find_by_username_fn: Box<dyn Fn(&str) -> AppResult<Option<User>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&User) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&User) -> AppResult<()> + Send + Sync>,
        exists_by_username_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
        exists_by_email_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
        exists_by_mobile_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
        bind_roles_fn: Box<dyn Fn(u64, &[u64]) -> AppResult<()> + Send + Sync>,
        bind_departments_fn: Box<dyn Fn(u64, &[u64]) -> AppResult<()> + Send + Sync>,
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
                bind_roles_fn: Box::new(|_, _| panic!("unexpected call")),
                bind_departments_fn: Box::new(|_, _| panic!("unexpected call")),
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
        async fn bind_roles(&self, uid: u64, rids: &[u64]) -> AppResult<()> { (self.bind_roles_fn)(uid, rids) }
        async fn bind_departments(&self, uid: u64, dids: &[u64]) -> AppResult<()> { (self.bind_departments_fn)(uid, dids) }
        async fn get_role_ids(&self, uid: u64) -> AppResult<Vec<u64>> { (self.get_role_ids_fn)(uid) }
        async fn get_dept_ids(&self, uid: u64) -> AppResult<Vec<u64>> { (self.get_dept_ids_fn)(uid) }
    }

    // --- Role repo mock (needed by UserService::assign_roles) ---
    struct TestRoleRepo {
        find_by_ids_fn: Box<dyn Fn(&[u64]) -> AppResult<Vec<Role>> + Send + Sync>,
    }

    impl TestRoleRepo {
        fn new() -> Self {
            Self {
                find_by_ids_fn: Box::new(|_| Ok(vec![])),
            }
        }
    }

    #[async_trait]
    impl RoleRepository for TestRoleRepo {
        async fn find_by_id(&self, _: u64) -> AppResult<Option<Role>> { Ok(None) }
        async fn find_by_code(&self, _: &str) -> AppResult<Option<Role>> { Ok(None) }
        async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Role>> { (self.find_by_ids_fn)(ids) }
        async fn find_page(&self, _: &RoleQuery, p: Page<Role>) -> AppResult<Page<Role>> { Ok(p) }
        async fn find_all(&self, _: &RoleQuery) -> AppResult<Vec<Role>> { Ok(vec![]) }
        async fn insert(&self, _: &Role) -> AppResult<()> { Ok(()) }
        async fn update(&self, _: &Role) -> AppResult<()> { Ok(()) }
        async fn soft_delete(&self, _: u64) -> AppResult<()> { Ok(()) }
        async fn exists_by_code(&self, _: &str) -> AppResult<bool> { Ok(false) }
        async fn bind_menus(&self, _: u64, _: &[u64]) -> AppResult<()> { Ok(()) }
        async fn get_menu_ids(&self, _: u64) -> AppResult<Vec<u64>> { Ok(vec![]) }
        async fn get_user_ids(&self, _: u64) -> AppResult<Vec<u64>> { Ok(vec![]) }
        async fn find_users_by_role_id(&self, _: u64) -> AppResult<Vec<User>> { Ok(vec![]) }
        async fn bind_users(&self, _: u64, _: &[u64]) -> AppResult<()> { Ok(()) }
        async fn unbind_users(&self, _: u64, _: &[u64]) -> AppResult<()> { Ok(()) }
    }

    // --- Department repo mock (needed by UserService::assign_departments) ---
    struct TestDeptRepo {
        find_by_ids_fn: Box<dyn Fn(&[u64]) -> AppResult<Vec<Department>> + Send + Sync>,
    }

    impl TestDeptRepo {
        fn new() -> Self {
            Self {
                find_by_ids_fn: Box::new(|_| Ok(vec![])),
            }
        }
    }

    #[async_trait]
    impl DepartmentRepository for TestDeptRepo {
        async fn find_by_id(&self, _: u64) -> AppResult<Option<Department>> { Ok(None) }
        async fn find_all(&self, _: &DeptQuery) -> AppResult<Vec<Department>> { Ok(vec![]) }
        async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Department>> { (self.find_by_ids_fn)(ids) }
        async fn find_by_parent_id(&self, _: u64) -> AppResult<Vec<Department>> { Ok(vec![]) }
        async fn insert(&self, _: &Department) -> AppResult<()> { Ok(()) }
        async fn update(&self, _: &Department) -> AppResult<()> { Ok(()) }
        async fn soft_delete(&self, _: u64) -> AppResult<()> { Ok(()) }
        async fn has_children(&self, _: u64) -> AppResult<bool> { Ok(false) }
        async fn has_users(&self, _: u64) -> AppResult<bool> { Ok(false) }
    }

    struct TestMenuRepo {}

    #[async_trait]
    impl MenuRepository for TestMenuRepo {
        async fn find_by_id(&self, _: u64) -> AppResult<Option<Menu>> { Ok(None) }
        async fn find_all(&self, _: &MenuQuery) -> AppResult<Vec<Menu>> { Ok(vec![]) }
        async fn find_by_ids(&self, _: &[u64]) -> AppResult<Vec<Menu>> { Ok(vec![]) }
        async fn find_by_parent_id(&self, _: u64) -> AppResult<Vec<Menu>> { Ok(vec![]) }
        async fn insert(&self, _: &Menu) -> AppResult<()> { Ok(()) }
        async fn update(&self, _: &Menu) -> AppResult<()> { Ok(()) }
        async fn soft_delete(&self, _: u64) -> AppResult<()> { Ok(()) }
        async fn has_children(&self, _: u64) -> AppResult<bool> { Ok(false) }
        async fn find_permission_codes_by_user_id(&self, _: u64) -> AppResult<std::collections::HashSet<String>> {
            let mut s = std::collections::HashSet::new();
            s.insert("read".into());
            Ok(s)
        }
    }

    fn make_user() -> User {
        User::create(1, "testuser".into(), "pwd".into(), "Test".into(), None)
    }

    // ── create_user ──

    #[tokio::test]
    async fn test_create_user_success() {
        let mut repo = TestUserRepo::new();
        repo.exists_by_username_fn = Box::new(|_| Ok(false));
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.create_user("new".into(), "p".into(), "N".into(), None).await.is_ok());
    }

    #[tokio::test]
    async fn test_create_user_duplicate() {
        let mut repo = TestUserRepo::new();
        repo.exists_by_username_fn = Box::new(|_| Ok(true));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.create_user("dup".into(), "p".into(), "N".into(), None).await.is_err());
    }

    // ── update_user ──

    #[tokio::test]
    async fn test_update_user_success() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        let r = svc.update_user(1, "New".into(), None, None, crate::user::model::value_object::Sex::Unknown, None, None).await;
        assert!(r.is_ok());
        assert_eq!(r.unwrap().nickname, "New");
    }

    #[tokio::test]
    async fn test_update_user_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.update_user(999, "X".into(), None, None, crate::user::model::value_object::Sex::Unknown, None, None).await.is_err());
    }

    // ── delete_user ──

    #[tokio::test]
    async fn test_delete_user() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.delete_user(1, None).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_user_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.delete_user(999, None).await.is_err());
    }

    // ── change_status ──

    #[tokio::test]
    async fn test_change_status() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        let r = svc.change_status(1, UserStatus::Locked, None).await;
        assert!(r.is_ok());
        assert_eq!(r.unwrap().status, UserStatus::Locked);
    }

    // ── assign_roles ──

    #[tokio::test]
    async fn test_assign_roles_success() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.bind_roles_fn = Box::new(|_, _| Ok(()));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.assign_roles(1, vec![10, 20]).await.is_ok());
    }

    #[tokio::test]
    async fn test_assign_roles_user_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.assign_roles(999, vec![1]).await.is_err());
    }

    #[tokio::test]
    async fn test_assign_roles_user_disabled() {
        let mut repo = TestUserRepo::new();
        let mut user = make_user();
        user.status = UserStatus::Disabled;
        repo.find_by_id_fn = Box::new(move |_| Ok(Some(user.clone())));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.assign_roles(1, vec![10]).await.is_err());
    }

    // ── assign_departments ──

    #[tokio::test]
    async fn test_assign_departments_success() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.bind_departments_fn = Box::new(|_, _| Ok(()));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.assign_departments(1, vec![10, 20]).await.is_ok());
    }

    #[tokio::test]
    async fn test_assign_departments_user_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.assign_departments(999, vec![1]).await.is_err());
    }

    // ── get_user ──

    #[tokio::test]
    async fn test_get_user_with_associations() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.get_role_ids_fn = Box::new(|_| Ok(vec![1, 2]));
        repo.get_dept_ids_fn = Box::new(|_| Ok(vec![10]));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        let u = svc.get_user(1).await.unwrap();
        assert_eq!(u.role_ids, vec![1, 2]);
        assert_eq!(u.dept_ids, vec![10]);
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.get_user(999).await.is_err());
    }

    // ── build_login_user ──

    #[tokio::test]
    async fn test_build_login_user() {
        let mut repo = TestUserRepo::new();
        repo.get_role_ids_fn = Box::new(|_| Ok(vec![1]));
        repo.get_dept_ids_fn = Box::new(|_| Ok(vec![10]));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        let lu = svc.build_login_user(&make_user()).await.unwrap();
        assert_eq!(lu.role_ids, vec![1]);
        assert!(lu.permissions.contains("read"));
    }

    // ── record_login ──

    #[tokio::test]
    async fn test_record_login() {
        let mut repo = TestUserRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_user())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        let u = svc.record_login(1, "10.0.0.1".into()).await.unwrap();
        assert_eq!(u.login_ip.as_deref(), Some("10.0.0.1"));
    }

    // ── exists_by_email ──

    #[tokio::test]
    async fn test_exists_by_email() {
        let mut repo = TestUserRepo::new();
        repo.exists_by_email_fn = Box::new(|_| Ok(true));
        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        assert!(svc.exists_by_email("x@y.com").await.unwrap());
    }

    // ── get_by_username ──

    #[tokio::test]
    async fn test_get_by_username_found() {
        let mut repo = TestUserRepo::new();
        let user = make_user();
        repo.find_by_username_fn = Box::new(move |_| Ok(Some(user.clone())));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        let result = svc.get_by_username("testuser").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().username, "testuser");
    }

    #[tokio::test]
    async fn test_get_by_username_not_found() {
        let mut repo = TestUserRepo::new();
        repo.find_by_username_fn = Box::new(|_| Ok(None));

        let svc = UserService::new(
            Arc::new(repo),
            Arc::new(TestRoleRepo::new()),
            Arc::new(TestDeptRepo::new()),
            Arc::new(TestMenuRepo {}),
        );
        let result = svc.get_by_username("nobody").await.unwrap();
        assert!(result.is_none());
    }
}
