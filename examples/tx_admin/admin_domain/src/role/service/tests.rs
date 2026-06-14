// ============================================================
// UNIT TESTS: RoleService (domain service, mocked repos)
// ============================================================

#[cfg(test)]
mod role_service_tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_common::page::Page;
    use tx_error::AppResult;
    use crate::shared::model::AuditFields;
    use crate::role::model::aggregate::Role;
    use crate::role::model::value_object::RoleQuery;
    use crate::role::repository::RoleRepository;
    use crate::role::service::RoleService;
    use crate::user::model::aggregate::User;
    use pretty_assertions::assert_eq;

    // ----------------------------------------------------------
    // TestRoleRepo: function-closure based mock
    // ----------------------------------------------------------

    struct TestRoleRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<Role>> + Send + Sync>,
        find_by_code_fn: Box<dyn Fn(&str) -> AppResult<Option<Role>> + Send + Sync>,
        find_by_ids_fn: Box<dyn Fn(&[u64]) -> AppResult<Vec<Role>> + Send + Sync>,
        find_page_fn: Box<dyn Fn(&RoleQuery, Page<Role>) -> AppResult<Page<Role>> + Send + Sync>,
        find_all_fn: Box<dyn Fn(&RoleQuery) -> AppResult<Vec<Role>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&Role) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&Role) -> AppResult<()> + Send + Sync>,
        exists_by_code_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
        bind_menus_fn: Box<dyn Fn(u64, &[u64]) -> AppResult<()> + Send + Sync>,
        find_users_by_role_id_fn: Box<dyn Fn(u64) -> AppResult<Vec<User>> + Send + Sync>,
        bind_users_fn: Box<dyn Fn(u64, &[u64]) -> AppResult<()> + Send + Sync>,
        unbind_users_fn: Box<dyn Fn(u64, &[u64]) -> AppResult<()> + Send + Sync>,
    }

    impl TestRoleRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_by_code_fn: Box::new(|_| panic!("unexpected call: find_by_code")),
                find_by_ids_fn: Box::new(|_| panic!("unexpected call: find_by_ids")),
                find_page_fn: Box::new(|_, _| panic!("unexpected call: find_page")),
                find_all_fn: Box::new(|_| panic!("unexpected call: find_all")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                update_fn: Box::new(|_| panic!("unexpected call: update")),
                exists_by_code_fn: Box::new(|_| panic!("unexpected call: exists_by_code")),
                bind_menus_fn: Box::new(|_, _| panic!("unexpected call: bind_menus")),
                find_users_by_role_id_fn: Box::new(|_| panic!("unexpected call: find_users_by_role_id")),
                bind_users_fn: Box::new(|_, _| panic!("unexpected call: bind_users")),
                unbind_users_fn: Box::new(|_, _| panic!("unexpected call: unbind_users")),
            }
        }
    }

    #[async_trait]
    impl RoleRepository for TestRoleRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<Role>> {
            (self.find_by_id_fn)(id)
        }
        async fn find_by_code(&self, code: &str) -> AppResult<Option<Role>> {
            (self.find_by_code_fn)(code)
        }
        async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Role>> {
            (self.find_by_ids_fn)(ids)
        }
        async fn find_page(&self, query: &RoleQuery, page: Page<Role>) -> AppResult<Page<Role>> {
            (self.find_page_fn)(query, page)
        }
        async fn find_all(&self, query: &RoleQuery) -> AppResult<Vec<Role>> {
            (self.find_all_fn)(query)
        }
        async fn insert(&self, role: &Role) -> AppResult<()> {
            (self.insert_fn)(role)
        }
        async fn update(&self, role: &Role) -> AppResult<()> {
            (self.update_fn)(role)
        }
        async fn soft_delete(&self, _id: u64) -> AppResult<()> {
            Ok(())
        }
        async fn exists_by_code(&self, code: &str) -> AppResult<bool> {
            (self.exists_by_code_fn)(code)
        }
        async fn bind_menus(&self, role_id: u64, menu_ids: &[u64]) -> AppResult<()> {
            (self.bind_menus_fn)(role_id, menu_ids)
        }
        async fn get_menu_ids(&self, _role_id: u64) -> AppResult<Vec<u64>> {
            Ok(vec![])
        }
        async fn get_user_ids(&self, _role_id: u64) -> AppResult<Vec<u64>> {
            Ok(vec![])
        }
        async fn find_users_by_role_id(&self, role_id: u64) -> AppResult<Vec<User>> {
            (self.find_users_by_role_id_fn)(role_id)
        }
        async fn bind_users(&self, role_id: u64, user_ids: &[u64]) -> AppResult<()> {
            (self.bind_users_fn)(role_id, user_ids)
        }
        async fn unbind_users(&self, role_id: u64, user_ids: &[u64]) -> AppResult<()> {
            (self.unbind_users_fn)(role_id, user_ids)
        }
    }

    // ----------------------------------------------------------
    // Helper: create a sample Role for testing
    // ----------------------------------------------------------

    fn make_role() -> Role {
        Role::restore(
            1,
            "Admin".into(),
            "admin".into(),
            1,
            4,
            None,
            0,         // active
            None,
            0,
            AuditFields::default(),
            vec![],
        )
    }

    // ==========================================================
    // create_role
    // ==========================================================

    #[tokio::test]
    async fn test_create_role_success() {
        let mut repo = TestRoleRepo::new();
        repo.exists_by_code_fn = Box::new(|_| Ok(false));
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.create_role("Editor".into(), "editor".into(), 2, Some("admin".into())).await;
        assert!(result.is_ok());
        let role = result.unwrap();
        assert_eq!(role.name, "Editor");
        assert_eq!(role.code, "editor");
        assert_eq!(role.sort, 2);
    }

    #[tokio::test]
    async fn test_create_role_duplicate_code() {
        let mut repo = TestRoleRepo::new();
        repo.exists_by_code_fn = Box::new(|_| Ok(true));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.create_role("Dup".into(), "admin".into(), 1, None).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // update_role
    // ==========================================================

    #[tokio::test]
    async fn test_update_role_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_role())));
        // code not taken by another role
        repo.find_by_code_fn = Box::new(|_| Ok(Some(make_role())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.update_role(
            1,
            "SuperAdmin".into(),
            "admin".into(),
            0,
            1,
            Some("updated".into()),
            Some("admin".into()),
        ).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "SuperAdmin");
    }

    #[tokio::test]
    async fn test_update_role_not_found() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.update_role(999, "X".into(), "x".into(), 0, 4, None, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_role_duplicate_code() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_role())));
        // another role with different id owns that code
        let mut other = make_role();
        other.id = 2;
        repo.find_by_code_fn = Box::new(move |_| Ok(Some(other.clone())));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.update_role(1, "X".into(), "taken_code".into(), 0, 4, None, None).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // delete_role
    // ==========================================================

    #[tokio::test]
    async fn test_delete_role_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_role())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.delete_role(1, Some("admin".into())).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_role_not_found() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.delete_role(999, None).await.is_err());
    }

    // ==========================================================
    // change_status
    // ==========================================================

    #[tokio::test]
    async fn test_change_status_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_role())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.change_status(1, 1, Some("admin".into())).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, 1);
    }

    #[tokio::test]
    async fn test_change_status_not_found() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.change_status(999, 1, None).await.is_err());
    }

    // ==========================================================
    // assign_menus
    // ==========================================================

    #[tokio::test]
    async fn test_assign_menus_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_role())));
        repo.bind_menus_fn = Box::new(|_, _| Ok(()));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.assign_menus(1, vec![10, 20, 30]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().menu_ids, vec![10, 20, 30]);
    }

    #[tokio::test]
    async fn test_assign_menus_role_not_found() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.assign_menus(999, vec![1]).await.is_err());
    }

    // ==========================================================
    // get_role_page
    // ==========================================================

    #[tokio::test]
    async fn test_get_role_page_success() {
        let mut repo = TestRoleRepo::new();
        let roles = vec![make_role()];
        repo.find_page_fn = Box::new(move |_, _| {
            Ok(Page::new(roles.clone(), 1, 10, 1))
        });

        let svc = RoleService::new(Arc::new(repo));
        let query = RoleQuery::default();
        let result = svc.get_role_page(&query, Page::request(1, 10)).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.list.len(), 1);
    }

    // ==========================================================
    // get_role
    // ==========================================================

    #[tokio::test]
    async fn test_get_role_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_role())));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.get_role(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().code, "admin");
    }

    #[tokio::test]
    async fn test_get_role_not_found() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.get_role(999).await.is_err());
    }

    // ==========================================================
    // get_roles_by_ids
    // ==========================================================

    #[tokio::test]
    async fn test_get_roles_by_ids_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_ids_fn = Box::new(|_| Ok(vec![make_role()]));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.get_roles_by_ids(&[1, 2]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_get_roles_by_ids_empty() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_ids_fn = Box::new(|_| Ok(vec![]));

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.get_roles_by_ids(&[999]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ==========================================================
    // get_all_roles
    // ==========================================================

    #[tokio::test]
    async fn test_get_all_roles_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_all_fn = Box::new(|_| Ok(vec![make_role()]));

        let svc = RoleService::new(Arc::new(repo));
        let query = RoleQuery::default();
        let result = svc.get_all_roles(&query).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    // ==========================================================
    // get_role_users
    // ==========================================================

    #[tokio::test]
    async fn test_get_role_users_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_role())));
        repo.find_users_by_role_id_fn = Box::new(|_| {
            Ok(vec![User::create(1, "user1".into(), "pwd".into(), "User One".into(), None)])
        });

        let svc = RoleService::new(Arc::new(repo));
        let result = svc.get_role_users(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_get_role_users_role_not_found() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.get_role_users(999).await.is_err());
    }

    // ==========================================================
    // add_users_to_role
    // ==========================================================

    #[tokio::test]
    async fn test_add_users_to_role_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_role())));
        repo.bind_users_fn = Box::new(|_, _| Ok(()));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.add_users_to_role(1, vec![10, 20]).await.is_ok());
    }

    #[tokio::test]
    async fn test_add_users_to_role_not_found() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.add_users_to_role(999, vec![1]).await.is_err());
    }

    // ==========================================================
    // remove_users_from_role
    // ==========================================================

    #[tokio::test]
    async fn test_remove_users_from_role_success() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_role())));
        repo.unbind_users_fn = Box::new(|_, _| Ok(()));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.remove_users_from_role(1, vec![10, 20]).await.is_ok());
    }

    #[tokio::test]
    async fn test_remove_users_from_role_not_found() {
        let mut repo = TestRoleRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = RoleService::new(Arc::new(repo));
        assert!(svc.remove_users_from_role(999, vec![1]).await.is_err());
    }
}
