// ============================================================
// UNIT TESTS: FileService (domain service, mocked repos)
// ============================================================

#[cfg(test)]
mod file_service_tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_common::page::Page;
    use tx_error::AppResult;
    use crate::shared::model::AuditFields;
    use crate::file::model::aggregate::{File, FileConfig};
    use crate::file::model::value_object::{FileDownloadInfo, FileQuery, FileUploadCommand};
    use crate::file::repository::{FileConfigRepository, FileRepository};
    use crate::file::service::FileService;
    use pretty_assertions::assert_eq;

    // ----------------------------------------------------------
    // TestFileRepo: function-closure based mock
    // ----------------------------------------------------------

    struct TestFileRepo {
        find_by_id_fn: Box<dyn Fn(u64) -> AppResult<Option<File>> + Send + Sync>,
        find_page_fn: Box<dyn Fn(&FileQuery, Page<File>) -> AppResult<Page<File>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&File) -> AppResult<()> + Send + Sync>,
        soft_delete_fn: Box<dyn Fn(u64) -> AppResult<()> + Send + Sync>,
        find_file_path_fn: Box<dyn Fn(u64) -> AppResult<String> + Send + Sync>,
    }

    impl TestFileRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_page_fn: Box::new(|_, _| panic!("unexpected call: find_page")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                soft_delete_fn: Box::new(|_| panic!("unexpected call: soft_delete")),
                find_file_path_fn: Box::new(|_| panic!("unexpected call: find_file_path")),
            }
        }
    }

    #[async_trait]
    impl FileRepository for TestFileRepo {
        async fn find_by_id(&self, id: u64) -> AppResult<Option<File>> {
            (self.find_by_id_fn)(id)
        }
        async fn find_page(&self, query: &FileQuery, page: Page<File>) -> AppResult<Page<File>> {
            (self.find_page_fn)(query, page)
        }
        async fn insert(&self, file: &File) -> AppResult<()> {
            (self.insert_fn)(file)
        }
        async fn update(&self, _file: &File) -> AppResult<()> {
            Ok(())
        }
        async fn soft_delete(&self, id: u64) -> AppResult<()> {
            (self.soft_delete_fn)(id)
        }
        async fn find_file_path(&self, id: u64) -> AppResult<String> {
            (self.find_file_path_fn)(id)
        }
    }

    // ----------------------------------------------------------
    // TestFileConfigRepo: function-closure based mock
    // ----------------------------------------------------------

    struct TestFileConfigRepo {
        find_by_id_fn: Box<dyn Fn(i32) -> AppResult<Option<FileConfig>> + Send + Sync>,
        find_master_fn: Box<dyn Fn() -> AppResult<Option<FileConfig>> + Send + Sync>,
        find_all_fn: Box<dyn Fn() -> AppResult<Vec<FileConfig>> + Send + Sync>,
        insert_fn: Box<dyn Fn(&FileConfig) -> AppResult<()> + Send + Sync>,
        update_fn: Box<dyn Fn(&FileConfig) -> AppResult<()> + Send + Sync>,
        soft_delete_fn: Box<dyn Fn(i32) -> AppResult<()> + Send + Sync>,
    }

    impl TestFileConfigRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_master_fn: Box::new(|| panic!("unexpected call: find_master")),
                find_all_fn: Box::new(|| panic!("unexpected call: find_all")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                update_fn: Box::new(|_| panic!("unexpected call: update")),
                soft_delete_fn: Box::new(|_| panic!("unexpected call: soft_delete")),
            }
        }
    }

    #[async_trait]
    impl FileConfigRepository for TestFileConfigRepo {
        async fn find_by_id(&self, id: i32) -> AppResult<Option<FileConfig>> {
            (self.find_by_id_fn)(id)
        }
        async fn find_master(&self) -> AppResult<Option<FileConfig>> {
            (self.find_master_fn)()
        }
        async fn find_all(&self) -> AppResult<Vec<FileConfig>> {
            (self.find_all_fn)()
        }
        async fn insert(&self, config: &FileConfig) -> AppResult<()> {
            (self.insert_fn)(config)
        }
        async fn update(&self, config: &FileConfig) -> AppResult<()> {
            (self.update_fn)(config)
        }
        async fn soft_delete(&self, id: i32) -> AppResult<()> {
            (self.soft_delete_fn)(id)
        }
    }

    // ----------------------------------------------------------
    // Helpers
    // ----------------------------------------------------------

    fn make_file() -> File {
        File::restore(
            1,
            Some(1),
            "test.txt".into(),
            "/uploads/test.txt".into(),
            "https://example.com/test.txt".into(),
            Some("txt".into()),
            1024,
            AuditFields::default(),
        )
    }

    fn make_upload_cmd() -> FileUploadCommand {
        FileUploadCommand {
            name: "test.txt".into(),
            path: "/uploads/test.txt".into(),
            url: "https://example.com/test.txt".into(),
            file_type: Some("txt".into()),
            size: 1024,
            config_id: Some(1),
        }
    }

    // ==========================================================
    // upload_file
    // ==========================================================

    #[tokio::test]
    async fn test_upload_file_success() {
        let mut file_repo = TestFileRepo::new();
        file_repo.insert_fn = Box::new(|_| Ok(()));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.upload_file(make_upload_cmd(), Some("admin".into())).await;
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.name, "test.txt");
        assert_eq!(file.path, "/uploads/test.txt");
        assert_eq!(file.size, 1024);
    }

    #[tokio::test]
    async fn test_upload_file_insert_error() {
        let mut file_repo = TestFileRepo::new();
        file_repo.insert_fn = Box::new(|_| Err(crate::shared::repository::RepositoryError::DatabaseFile.into()));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.upload_file(make_upload_cmd(), None).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // delete_file
    // ==========================================================

    #[tokio::test]
    async fn test_delete_file_success() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file())));
        file_repo.soft_delete_fn = Box::new(|_| Ok(()));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.delete_file(1, Some("admin".into())).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_file_not_found() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(None));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.delete_file(999, None).await.is_err());
    }

    // ==========================================================
    // get_file_page
    // ==========================================================

    #[tokio::test]
    async fn test_get_file_page_success() {
        let mut file_repo = TestFileRepo::new();
        let items = vec![make_file()];
        file_repo.find_page_fn = Box::new(move |_, _| {
            Ok(Page::new(items.clone(), 1, 10, 1))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let query = FileQuery::default();
        let result = svc.get_file_page(&query, Page::request(1, 10)).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.list.len(), 1);
    }

    // ==========================================================
    // get_file
    // ==========================================================

    #[tokio::test]
    async fn test_get_file_success() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file())));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_file(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "test.txt");
    }

    #[tokio::test]
    async fn test_get_file_not_found() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(None));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.get_file(999).await.is_err());
    }

    // ==========================================================
    // download_file
    // ==========================================================

    #[tokio::test]
    async fn test_download_file_txt() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file())));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.filename, "test.txt");
        assert_eq!(info.content_type, "text/plain");
        assert_eq!(info.size, 1024);
    }

    #[tokio::test]
    async fn test_download_file_pdf() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "doc.pdf".into(), "/uploads/doc.pdf".into(),
                "https://example.com/doc.pdf".into(), None, 2048,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content_type, "application/pdf");
    }

    #[tokio::test]
    async fn test_download_file_png() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "img.png".into(), "/uploads/img.png".into(),
                "https://example.com/img.png".into(), None, 512,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content_type, "image/png");
    }

    #[tokio::test]
    async fn test_download_file_unknown_extension() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "data.xyz".into(), "/uploads/data.xyz".into(),
                "https://example.com/data.xyz".into(), None, 256,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content_type, "application/octet-stream");
    }

    #[tokio::test]
    async fn test_download_file_not_found() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(None));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.download_file(999).await.is_err());
    }
}
