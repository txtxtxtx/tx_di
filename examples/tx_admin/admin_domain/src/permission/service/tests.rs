// ============================================================
// UNIT TESTS: PermissionService (domain service, mocked repos)
// ============================================================

#[cfg(test)]
mod permission_service_tests {
    use std::collections::HashSet;
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_error::AppResult;
    use crate::shared::model::AuditFields;
    use crate::permission::model::aggregate::Permission;
    use crate::permission::model::value_object::{PermissionCheck, PermissionType};
    use crate::permission::repository::PermissionRepository;
    use crate::permission::service::PermissionService;
    use pretty_assertions::assert_eq;

    // ----------------------------------------------------------
    // TestPermissionRepo: function-closure based mock
    // ----------------------------------------------------------

    struct TestPermissionRepo {
        find_by_role_ids_fn: Box<dyn Fn(&[u64]) -> AppResult<HashSet<String>> + Send + Sync>,
        find_by_user_id_fn: Box<dyn Fn(u64) -> AppResult<HashSet<String>> + Send + Sync>,
        find_all_fn: Box<dyn Fn() -> AppResult<HashSet<PermissionCheck>> + Send + Sync>,
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<Permission>> + Send + Sync>,
        find_by_code_fn: Box<dyn Fn(&str) -> AppResult<Option<Permission>> + Send + Sync>,
        find_all_permissions_fn: Box<dyn Fn() -> AppResult<Vec<Permission>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&Permission) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&Permission) -> AppResult<()> + Send + Sync>,
        exists_by_code_fn: Box<dyn Fn(&str) -> AppResult<bool> + Send + Sync>,
        find_by_codes_fn: Box<dyn Fn(&[String]) -> AppResult<Vec<Permission>> + Send + Sync>,
    }

    impl TestPermissionRepo {
        fn new() -> Self {
            Self {
                find_by_role_ids_fn: Box::new(|_| panic!("unexpected call: find_by_role_ids")),
                find_by_user_id_fn: Box::new(|_| panic!("unexpected call: find_by_user_id")),
                find_all_fn: Box::new(|| panic!("unexpected call: find_all")),
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_by_code_fn: Box::new(|_| panic!("unexpected call: find_by_code")),
                find_all_permissions_fn: Box::new(|| panic!("unexpected call: find_all_permissions")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                update_fn: Box::new(|_| panic!("unexpected call: update")),
                exists_by_code_fn: Box::new(|_| panic!("unexpected call: exists_by_code")),
                find_by_codes_fn: Box::new(|_| panic!("unexpected call: find_by_codes")),
            }
        }
    }

    #[async_trait]
    impl PermissionRepository for TestPermissionRepo {
        async fn find_by_role_ids(&self, role_ids: &[u64]) -> AppResult<HashSet<String>> {
            (self.find_by_role_ids_fn)(role_ids)
        }
        async fn find_by_user_id(&self, user_id: u64) -> AppResult<HashSet<String>> {
            (self.find_by_user_id_fn)(user_id)
        }
        async fn find_all(&self) -> AppResult<HashSet<PermissionCheck>> {
            (self.find_all_fn)()
        }
        async fn find_by_id(&self, id: u64) -> AppResult<Option<Permission>> {
            (self.find_by_id_fn)(id)
        }
        async fn find_by_code(&self, code: &str) -> AppResult<Option<Permission>> {
            (self.find_by_code_fn)(code)
        }
        async fn find_all_permissions(&self) -> AppResult<Vec<Permission>> {
            (self.find_all_permissions_fn)()
        }
        async fn insert(&self, permission: &Permission) -> AppResult<()> {
            (self.insert_fn)(permission)
        }
        async fn update(&self, permission: &Permission) -> AppResult<()> {
            (self.update_fn)(permission)
        }
        async fn soft_delete(&self, _id: u64) -> AppResult<()> {
            Ok(())
        }
        async fn exists_by_code(&self, code: &str) -> AppResult<bool> {
            (self.exists_by_code_fn)(code)
        }
        async fn find_by_codes(&self, codes: &[String]) -> AppResult<Vec<Permission>> {
            (self.find_by_codes_fn)(codes)
        }
    }

    // ----------------------------------------------------------
    // Helper: create a sample Permission for testing
    // ----------------------------------------------------------

    fn make_permission() -> Permission {
        Permission::restore(
            1,
            "View Users".into(),
            "system:user:view".into(),
            PermissionType::Menu,
            0,
            1,
            Some("View user list".into()),
            0,         // active
            AuditFields::default(),
        )
    }

    // ==========================================================
    // get_user_permissions
    // ==========================================================

    #[tokio::test]
    async fn test_get_user_permissions_success() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_user_id_fn = Box::new(move |_| {
            let mut p = HashSet::new();
            p.insert("system:user:view".into());
            p.insert("system:user:edit".into());
            Ok(p)
        });

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_user_permissions(1).await;
        assert!(result.is_ok());
        let perms = result.unwrap();
        assert_eq!(perms.len(), 2);
        assert!(perms.contains("system:user:view"));
        assert!(perms.contains("system:user:edit"));
    }

    #[tokio::test]
    async fn test_get_user_permissions_empty() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_user_id_fn = Box::new(|_| Ok(HashSet::new()));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_user_permissions(999).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ==========================================================
    // check_permission
    // ==========================================================

    #[tokio::test]
    async fn test_check_permission_granted() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_user_id_fn = Box::new(|_| {
            let mut p = HashSet::new();
            p.insert("system:user:view".into());
            p.insert("system:user:edit".into());
            Ok(p)
        });

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.check_permission(1, "system:user:view").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_check_permission_denied() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_user_id_fn = Box::new(|_| {
            let mut p = HashSet::new();
            p.insert("system:user:view".into());
            Ok(p)
        });

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.check_permission(1, "system:user:delete").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_check_permission_user_no_permissions() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_user_id_fn = Box::new(|_| Ok(HashSet::new()));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.check_permission(999, "system:user:view").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // ==========================================================
    // get_role_permissions
    // ==========================================================

    #[tokio::test]
    async fn test_get_role_permissions_success() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_role_ids_fn = Box::new(|_| {
            let mut p = HashSet::new();
            p.insert("system:user:view".into());
            p.insert("system:role:view".into());
            Ok(p)
        });

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_role_permissions(&[1, 2]).await;
        assert!(result.is_ok());
        let perms = result.unwrap();
        assert_eq!(perms.len(), 2);
    }

    #[tokio::test]
    async fn test_get_role_permissions_empty_roles() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_role_ids_fn = Box::new(|_| Ok(HashSet::new()));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_role_permissions(&[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ==========================================================
    // get_all_permissions
    // ==========================================================

    #[tokio::test]
    async fn test_get_all_permissions_success() {
        let mut repo = TestPermissionRepo::new();
        repo.find_all_fn = Box::new(|| {
            let mut p = HashSet::new();
            p.insert(PermissionCheck {
                code: "system:user:view".into(),
                name: "View Users".into(),
                permission_type: PermissionType::Menu,
            });
            p.insert(PermissionCheck {
                code: "system:user:edit".into(),
                name: "Edit Users".into(),
                permission_type: PermissionType::Button,
            });
            Ok(p)
        });

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_all_permissions().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_all_permissions_empty() {
        let mut repo = TestPermissionRepo::new();
        repo.find_all_fn = Box::new(|| Ok(HashSet::new()));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_all_permissions().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ==========================================================
    // create_permission
    // ==========================================================

    #[tokio::test]
    async fn test_create_permission_success() {
        let mut repo = TestPermissionRepo::new();
        repo.exists_by_code_fn = Box::new(|_| Ok(false));
        repo.insert_fn = Box::new(|_| Ok(()));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.create_permission(
            "Edit Users".into(),
            "system:user:edit".into(),
            PermissionType::Button,
            0,
            2,
            Some("Edit user info".into()),
            Some("admin".into()),
        ).await;
        assert!(result.is_ok());
        let perm = result.unwrap();
        assert_eq!(perm.name, "Edit Users");
        assert_eq!(perm.permission_code, "system:user:edit");
        assert_eq!(perm.permission_type, PermissionType::Button);
    }

    #[tokio::test]
    async fn test_create_permission_duplicate_code() {
        let mut repo = TestPermissionRepo::new();
        repo.exists_by_code_fn = Box::new(|_| Ok(true));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.create_permission(
            "Dup".into(),
            "system:user:view".into(),
            PermissionType::Menu,
            0,
            1,
            None,
            None,
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // update_permission
    // ==========================================================

    #[tokio::test]
    async fn test_update_permission_success() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_permission())));
        // code not taken by another permission
        repo.find_by_code_fn = Box::new(|_| Ok(Some(make_permission())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.update_permission(
            1,
            "View Users Updated".into(),
            "system:user:view".into(),
            PermissionType::Menu,
            0,
            1,
            Some("updated desc".into()),
            Some("admin".into()),
        ).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "View Users Updated");
    }

    #[tokio::test]
    async fn test_update_permission_not_found() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.update_permission(
            999,
            "X".into(),
            "x".into(),
            PermissionType::Menu,
            0,
            0,
            None,
            None,
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_permission_duplicate_code() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_permission())));
        // another permission with different id owns that code
        let mut other = make_permission();
        other.id = 2;
        repo.find_by_code_fn = Box::new(move |_| Ok(Some(other.clone())));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.update_permission(
            1,
            "X".into(),
            "taken_code".into(),
            PermissionType::Menu,
            0,
            0,
            None,
            None,
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // delete_permission
    // ==========================================================

    #[tokio::test]
    async fn test_delete_permission_success() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_permission())));
        repo.update_fn = Box::new(|_| Ok(()));

        let svc = PermissionService::new(Arc::new(repo));
        assert!(svc.delete_permission(1, Some("admin".into())).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_permission_not_found() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = PermissionService::new(Arc::new(repo));
        assert!(svc.delete_permission(999, None).await.is_err());
    }

    // ==========================================================
    // get_permission
    // ==========================================================

    #[tokio::test]
    async fn test_get_permission_success() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(Some(make_permission())));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_permission(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().permission_code, "system:user:view");
    }

    #[tokio::test]
    async fn test_get_permission_not_found() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = PermissionService::new(Arc::new(repo));
        assert!(svc.get_permission(999).await.is_err());
    }

    // ==========================================================
    // get_all_permission_details
    // ==========================================================

    #[tokio::test]
    async fn test_get_all_permission_details_success() {
        let mut repo = TestPermissionRepo::new();
        repo.find_all_permissions_fn = Box::new(|| Ok(vec![make_permission()]));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_all_permission_details().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_get_all_permission_details_empty() {
        let mut repo = TestPermissionRepo::new();
        repo.find_all_permissions_fn = Box::new(|| Ok(vec![]));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_all_permission_details().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ==========================================================
    // get_permissions_by_codes
    // ==========================================================

    #[tokio::test]
    async fn test_get_permissions_by_codes_success() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_codes_fn = Box::new(|_| Ok(vec![make_permission()]));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_permissions_by_codes(&["system:user:view".into()]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_get_permissions_by_codes_empty() {
        let mut repo = TestPermissionRepo::new();
        repo.find_by_codes_fn = Box::new(|_| Ok(vec![]));

        let svc = PermissionService::new(Arc::new(repo));
        let result = svc.get_permissions_by_codes(&["nonexistent".into()]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
