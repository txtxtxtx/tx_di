// ============================================================
// UNIT TESTS: MenuService (domain service, mocked repos)
// ============================================================

#[cfg(test)]
mod menu_service_tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_error::AppResult;
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::repository::RepositoryError;
    use crate::menu::model::aggregate::Menu;
    use crate::menu::model::value_object::MenuQuery;
    use crate::menu::repository::MenuRepository;
    use crate::menu::service::MenuService;
    use pretty_assertions::assert_eq;

    struct TestMenuRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<Menu>> + Send + Sync>,
        find_all_fn: Box<dyn Fn(&MenuQuery) -> AppResult<Vec<Menu>> + Send + Sync>,
        find_by_ids_fn: Box<dyn Fn(&[u64]) -> AppResult<Vec<Menu>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&Menu) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&Menu) -> AppResult<()> + Send + Sync>,
        has_children_fn: Box<dyn Fn(u64) -> AppResult<bool> + Send + Sync>,
    }

    impl TestMenuRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call")),
                find_all_fn: Box::new(|_| panic!("unexpected call")),
                find_by_ids_fn: Box::new(|_| panic!("unexpected call")),
                insert_fn: Box::new(|_| panic!("unexpected call")),
                update_fn: Box::new(|_| panic!("unexpected call")),
                has_children_fn: Box::new(|_| panic!("unexpected call")),
            }
        }
    }

    #[async_trait]
    impl MenuRepository for TestMenuRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<Menu>> { (self.find_by_id_fn)(id) }
        async fn find_all(&self, query: &MenuQuery) -> AppResult<Vec<Menu>> { (self.find_all_fn)(query) }
        async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Menu>> { (self.find_by_ids_fn)(ids) }
        async fn find_by_parent_id(&self, _: u64) -> AppResult<Vec<Menu>> { Ok(vec![]) }
        async fn insert(&self, menu: &Menu) -> AppResult<()> { (self.insert_fn)(menu) }
        async fn update(&self, menu: &Menu) -> AppResult<()> { (self.update_fn)(menu) }
        async fn soft_delete(&self, _: u64) -> AppResult<()> { Ok(()) }
        async fn has_children(&self, parent_id: u64) -> AppResult<bool> { (self.has_children_fn)(parent_id) }
    }

    fn make_menu() -> Menu {
        Menu::create(1, "Dashboard".into(), "dashboard:view".into(), 1, 1, 0, Some("admin".into()))
    }

    fn make_root_menu(id: u64, name: &str, parent_id: u64) -> Menu {
        Menu::create(id, name.into(), format!("{}:view", name.to_lowercase()), 1, 1, parent_id, Some("admin".into()))
    }

    // ---- create_menu ----

    #[tokio::test]
    async fn test_create_menu_success() {
        let mut repo = TestMenuRepo::new();
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = MenuService::new(Arc::new(repo));
        let r = svc.create_menu(
            "System".into(), "system:view".into(), 0, 1, 0,
            Some("/system".into()), Some("setting".into()), None, None, Some("admin".into()),
        ).await;
        assert!(r.is_ok());
        let menu = r.unwrap();
        assert_eq!(menu.name, "System");
        assert_eq!(menu.permission, "system:view");
        assert_eq!(menu.path, Some("/system".into()));
        assert_eq!(menu.icon, Some("setting".into()));
    }

    #[tokio::test]
    async fn test_create_menu_insert_error() {
        let mut repo = TestMenuRepo::new();
        repo.insert_fn = Box::new(|_| Err(RepositoryError::DatabaseMenu.into()));

        let svc = MenuService::new(Arc::new(repo));
        assert!(svc.create_menu(
            "Fail".into(), "fail:view".into(), 0, 1, 0,
            None, None, None, None, None,
        ).await.is_err());
    }

    // ---- update_menu ----

    #[tokio::test]
    async fn test_update_menu_success() {
        let mut repo = TestMenuRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_menu())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = MenuService::new(Arc::new(repo));
        let r = svc.update_menu(
            1, "NewName".into(), "new:perm".into(), 1, 2, 0,
            Some("/new".into()), None, None, None, 1, 1, Some("updater".into()),
        ).await;
        assert!(r.is_ok());
        assert_eq!(r.unwrap().name, "NewName");
    }

    #[tokio::test]
    async fn test_update_menu_not_found() {
        let mut repo = TestMenuRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = MenuService::new(Arc::new(repo));
        assert!(svc.update_menu(
            999, "X".into(), "x".into(), 0, 0, 0,
            None, None, None, None, 0, 0, None,
        ).await.is_err());
    }

    #[tokio::test]
    async fn test_update_menu_self_parent() {
        let mut repo = TestMenuRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_menu())));

        let svc = MenuService::new(Arc::new(repo));
        // parent_id == menu_id should fail
        assert!(svc.update_menu(
            1, "Name".into(), "perm".into(), 0, 0, 1,
            None, None, None, None, 0, 0, None,
        ).await.is_err());
    }

    // ---- delete_menu ----

    #[tokio::test]
    async fn test_delete_menu_success() {
        let mut repo = TestMenuRepo::new();
        repo.has_children_fn = Box::new(|_| Ok(false));
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_menu())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = MenuService::new(Arc::new(repo));
        assert!(svc.delete_menu(1, Some("admin".into())).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_menu_has_children() {
        let mut repo = TestMenuRepo::new();
        repo.has_children_fn = Box::new(|_| Ok(true));

        let svc = MenuService::new(Arc::new(repo));
        assert!(svc.delete_menu(1, None).await.is_err());
    }

    #[tokio::test]
    async fn test_delete_menu_not_found() {
        let mut repo = TestMenuRepo::new();
        repo.has_children_fn = Box::new(|_| Ok(false));
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = MenuService::new(Arc::new(repo));
        assert!(svc.delete_menu(999, None).await.is_err());
    }

    // ---- get_all_menus ----

    #[tokio::test]
    async fn test_get_all_menus() {
        let mut repo = TestMenuRepo::new();
        repo.find_all_fn = Box::new(|_| Ok(vec![make_menu()]));

        let svc = MenuService::new(Arc::new(repo));
        let menus = svc.get_all_menus(&MenuQuery::default()).await.unwrap();
        assert_eq!(menus.len(), 1);
        assert_eq!(menus[0].name, "Dashboard");
    }

    // ---- get_menus_by_ids ----

    #[tokio::test]
    async fn test_get_menus_by_ids() {
        let mut repo = TestMenuRepo::new();
        repo.find_by_ids_fn = Box::new(|_| Ok(vec![make_menu()]));

        let svc = MenuService::new(Arc::new(repo));
        let menus = svc.get_menus_by_ids(&[1, 2]).await.unwrap();
        assert_eq!(menus.len(), 1);
    }

    #[tokio::test]
    async fn test_get_menus_by_ids_empty() {
        let mut repo = TestMenuRepo::new();
        repo.find_by_ids_fn = Box::new(|_| Ok(vec![]));

        let svc = MenuService::new(Arc::new(repo));
        let menus = svc.get_menus_by_ids(&[999]).await.unwrap();
        assert!(menus.is_empty());
    }

    // ---- get_menu_tree ----

    #[tokio::test]
    async fn test_get_menu_tree_simple() {
        let mut repo = TestMenuRepo::new();
        repo.find_all_fn = Box::new(|_| {
            Ok(vec![
                make_root_menu(1, "System", 0),
                make_root_menu(2, "User", 1),
            ])
        });

        let svc = MenuService::new(Arc::new(repo));
        let tree = svc.get_menu_tree(&MenuQuery::default()).await.unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "System");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].name, "User");
    }

    #[tokio::test]
    async fn test_get_menu_tree_empty() {
        let mut repo = TestMenuRepo::new();
        repo.find_all_fn = Box::new(|_| Ok(vec![]));

        let svc = MenuService::new(Arc::new(repo));
        let tree = svc.get_menu_tree(&MenuQuery::default()).await.unwrap();
        assert!(tree.is_empty());
    }

    #[tokio::test]
    async fn test_get_menu_tree_skips_deleted() {
        let mut repo = TestMenuRepo::new();
        repo.find_all_fn = Box::new(|_| {
            let mut deleted_menu = make_root_menu(2, "Deleted", 0);
            deleted_menu.audit.deleted = DeletedStatus::Deleted;
            Ok(vec![
                make_root_menu(1, "Active", 0),
                deleted_menu,
            ])
        });

        let svc = MenuService::new(Arc::new(repo));
        let tree = svc.get_menu_tree(&MenuQuery::default()).await.unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "Active");
    }
}
