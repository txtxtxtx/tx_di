// ============================================================
// UNIT TESTS: DepartmentService (domain service, mocked repos)
// ============================================================

#[cfg(test)]
mod department_service_tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_error::AppResult;
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::repository::RepositoryError;
    use crate::department::model::aggregate::Department;
    use crate::department::model::value_object::DeptQuery;
    use crate::department::repository::DepartmentRepository;
    use crate::department::service::DepartmentService;
    use pretty_assertions::assert_eq;

    struct TestDeptRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<Department>> + Send + Sync>,
        find_all_fn: Box<dyn Fn(&DeptQuery) -> AppResult<Vec<Department>> + Send + Sync>,
        find_by_ids_fn: Box<dyn Fn(&[u64]) -> AppResult<Vec<Department>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&Department) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&Department) -> AppResult<()> + Send + Sync>,
        has_children_fn: Box<dyn Fn(u64) -> AppResult<bool> + Send + Sync>,
        has_users_fn: Box<dyn Fn(u64) -> AppResult<bool> + Send + Sync>,
    }

    impl TestDeptRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call")),
                find_all_fn: Box::new(|_| panic!("unexpected call")),
                find_by_ids_fn: Box::new(|_| panic!("unexpected call")),
                insert_fn: Box::new(|_| panic!("unexpected call")),
                update_fn: Box::new(|_| panic!("unexpected call")),
                has_children_fn: Box::new(|_| panic!("unexpected call")),
                has_users_fn: Box::new(|_| panic!("unexpected call")),
            }
        }
    }

    #[async_trait]
    impl DepartmentRepository for TestDeptRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<Department>> { (self.find_by_id_fn)(id) }
        async fn find_all(&self, query: &DeptQuery) -> AppResult<Vec<Department>> { (self.find_all_fn)(query) }
        async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Department>> { (self.find_by_ids_fn)(ids) }
        async fn find_by_parent_id(&self, _: u64) -> AppResult<Vec<Department>> { Ok(vec![]) }
        async fn insert(&self, dept: &Department) -> AppResult<()> { (self.insert_fn)(dept) }
        async fn update(&self, dept: &Department) -> AppResult<()> { (self.update_fn)(dept) }
        async fn soft_delete(&self, _: u64) -> AppResult<()> { Ok(()) }
        async fn has_children(&self, parent_id: u64) -> AppResult<bool> { (self.has_children_fn)(parent_id) }
        async fn has_users(&self, dept_id: u64) -> AppResult<bool> { (self.has_users_fn)(dept_id) }
    }

    fn make_dept() -> Department {
        Department::create(1, "Engineering".into(), 0, 1, Some("admin".into()))
    }

    fn make_dept_with(id: u64, name: &str, parent_id: u64) -> Department {
        Department::create(id, name.into(), parent_id, 1, Some("admin".into()))
    }

    // ---- create_dept ----

    #[tokio::test]
    async fn test_create_dept_success() {
        let mut repo = TestDeptRepo::new();
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = DepartmentService::new(Arc::new(repo));
        let r = svc.create_dept("HR".into(), 0, 1, Some("admin".into())).await;
        assert!(r.is_ok());
        let dept = r.unwrap();
        assert_eq!(dept.name, "HR");
        assert_eq!(dept.parent_id, 0);
    }

    #[tokio::test]
    async fn test_create_dept_insert_error() {
        let mut repo = TestDeptRepo::new();
        repo.insert_fn = Box::new(|_| Err(RepositoryError::Database.into()));

        let svc = DepartmentService::new(Arc::new(repo));
        assert!(svc.create_dept("Fail".into(), 0, 1, None).await.is_err());
    }

    // ---- update_dept ----

    #[tokio::test]
    async fn test_update_dept_success() {
        let mut repo = TestDeptRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_dept())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = DepartmentService::new(Arc::new(repo));
        let r = svc.update_dept(
            1, "NewName".into(), 0, 2, Some(10), Some("123".into()), Some("a@b.com".into()), Some("admin".into()),
        ).await;
        assert!(r.is_ok());
        let dept = r.unwrap();
        assert_eq!(dept.name, "NewName");
        assert_eq!(dept.sort, 2);
        assert_eq!(dept.leader_user_id, Some(10));
    }

    #[tokio::test]
    async fn test_update_dept_not_found() {
        let mut repo = TestDeptRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = DepartmentService::new(Arc::new(repo));
        assert!(svc.update_dept(
            999, "X".into(), 0, 0, None, None, None, None,
        ).await.is_err());
    }

    #[tokio::test]
    async fn test_update_dept_self_parent() {
        let mut repo = TestDeptRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_dept())));

        let svc = DepartmentService::new(Arc::new(repo));
        // parent_id == dept_id should fail
        assert!(svc.update_dept(
            1, "Name".into(), 1, 0, None, None, None, None,
        ).await.is_err());
    }

    // ---- delete_dept ----

    #[tokio::test]
    async fn test_delete_dept_success() {
        let mut repo = TestDeptRepo::new();
        repo.has_children_fn = Box::new(|_| Ok(false));
        repo.has_users_fn = Box::new(|_| Ok(false));
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_dept())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = DepartmentService::new(Arc::new(repo));
        assert!(svc.delete_dept(1, Some("admin".into())).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_dept_has_children() {
        let mut repo = TestDeptRepo::new();
        repo.has_children_fn = Box::new(|_| Ok(true));

        let svc = DepartmentService::new(Arc::new(repo));
        assert!(svc.delete_dept(1, None).await.is_err());
    }

    #[tokio::test]
    async fn test_delete_dept_has_users() {
        let mut repo = TestDeptRepo::new();
        repo.has_children_fn = Box::new(|_| Ok(false));
        repo.has_users_fn = Box::new(|_| Ok(true));

        let svc = DepartmentService::new(Arc::new(repo));
        assert!(svc.delete_dept(1, None).await.is_err());
    }

    #[tokio::test]
    async fn test_delete_dept_not_found() {
        let mut repo = TestDeptRepo::new();
        repo.has_children_fn = Box::new(|_| Ok(false));
        repo.has_users_fn = Box::new(|_| Ok(false));
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = DepartmentService::new(Arc::new(repo));
        assert!(svc.delete_dept(999, None).await.is_err());
    }

    // ---- get_dept_tree ----

    #[tokio::test]
    async fn test_get_dept_tree_simple() {
        let mut repo = TestDeptRepo::new();
        repo.find_all_fn = Box::new(|_| {
            Ok(vec![
                make_dept_with(1, "Company", 0),
                make_dept_with(2, "Engineering", 1),
                make_dept_with(3, "HR", 1),
            ])
        });

        let svc = DepartmentService::new(Arc::new(repo));
        let tree = svc.get_dept_tree(&DeptQuery::default()).await.unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "Company");
        assert_eq!(tree[0].children.len(), 2);
    }

    #[tokio::test]
    async fn test_get_dept_tree_empty() {
        let mut repo = TestDeptRepo::new();
        repo.find_all_fn = Box::new(|_| Ok(vec![]));

        let svc = DepartmentService::new(Arc::new(repo));
        let tree = svc.get_dept_tree(&DeptQuery::default()).await.unwrap();
        assert!(tree.is_empty());
    }

    #[tokio::test]
    async fn test_get_dept_tree_skips_deleted() {
        let mut repo = TestDeptRepo::new();
        repo.find_all_fn = Box::new(|_| {
            let mut deleted = make_dept_with(2, "OldDept", 0);
            deleted.audit.deleted = DeletedStatus::Deleted;
            Ok(vec![
                make_dept_with(1, "Active", 0),
                deleted,
            ])
        });

        let svc = DepartmentService::new(Arc::new(repo));
        let tree = svc.get_dept_tree(&DeptQuery::default()).await.unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "Active");
    }

    // ---- get_all_depts ----

    #[tokio::test]
    async fn test_get_all_depts() {
        let mut repo = TestDeptRepo::new();
        repo.find_all_fn = Box::new(|_| Ok(vec![make_dept()]));

        let svc = DepartmentService::new(Arc::new(repo));
        let depts = svc.get_all_depts(&DeptQuery::default()).await.unwrap();
        assert_eq!(depts.len(), 1);
        assert_eq!(depts[0].name, "Engineering");
    }

    // ---- get_dept ----

    #[tokio::test]
    async fn test_get_dept_success() {
        let mut repo = TestDeptRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_dept())));

        let svc = DepartmentService::new(Arc::new(repo));
        let dept = svc.get_dept(1).await.unwrap();
        assert_eq!(dept.name, "Engineering");
    }

    #[tokio::test]
    async fn test_get_dept_not_found() {
        let mut repo = TestDeptRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = DepartmentService::new(Arc::new(repo));
        assert!(svc.get_dept(999).await.is_err());
    }
}
