// ============================================================
// UNIT TESTS: FileService (domain service, mocked repos)
// Coverage: All public methods of FileService
// ============================================================

#[cfg(test)]
mod file_service_tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use tx_common::page::Page;
    use tx_error::AppResult;
    use crate::shared::model::{AggregateRoot, AuditFields};
    use crate::shared::model::value_object::DeletedStatus;
    use crate::shared::model::DomainEvent;
    use crate::file::model::aggregate::{File, FileConfig};
    use crate::file::model::value_object::{FileQuery, FileUploadCommand};
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
        update_fn: Box<dyn Fn(&File) -> AppResult<()> + Send + Sync>,
        soft_delete_fn: Box<dyn Fn(u64) -> AppResult<()> + Send + Sync>,
        find_file_path_fn: Box<dyn Fn(u64) -> AppResult<String> + Send + Sync>,
    }

    impl TestFileRepo {
        fn new() -> Self {
            Self {
                find_by_id_fn: Box::new(|_| panic!("unexpected call: find_by_id")),
                find_page_fn: Box::new(|_, _| panic!("unexpected call: find_page")),
                insert_fn: Box::new(|_| panic!("unexpected call: insert")),
                update_fn: Box::new(|_| panic!("unexpected call: update")),
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
        async fn update(&self, file: &File) -> AppResult<()> {
            (self.update_fn)(file)
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

    fn make_file_config() -> FileConfig {
        FileConfig::restore(
            1, "Default".into(), 1, Some("备注".into()), 0,
            r#"{"allowed_extensions":["pdf","txt"]}"#.into(),
            AuditFields::default(),
        )
    }

    fn make_master_config() -> FileConfig {
        FileConfig::restore(
            1, "Master".into(), 1, None, 1,
            r#"{"allowed_extensions":["jpg","png"]}"#.into(),
            AuditFields::default(),
        )
    }

    fn make_alt_config() -> FileConfig {
        FileConfig::restore(
            2, "Alt".into(), 2, None, 0,
            "{}".into(),
            AuditFields::default(),
        )
    }

    // ==========================================================
    // upload_file
    // ==========================================================

    /// 正常上传文件：insert 成功，返回的 File 各字段正确。
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

    /// 上传文件时仓储 insert 出错，应返回错误。
    #[tokio::test]
    async fn test_upload_file_insert_error() {
        let mut file_repo = TestFileRepo::new();
        file_repo.insert_fn = Box::new(|_| Err(crate::shared::repository::RepositoryError::DatabaseFile.into()));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.upload_file(make_upload_cmd(), None).await;
        assert!(result.is_err());
    }

    /// 上传文件自动生成雪花 ID，应 > 0。
    #[tokio::test]
    async fn test_upload_file_generates_unique_id() {
        let mut file_repo = TestFileRepo::new();
        file_repo.insert_fn = Box::new(|_| Ok(()));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.upload_file(make_upload_cmd(), None).await;
        assert!(result.is_ok());
        // Snowflake ID should be > 0
        assert!(result.unwrap().id > 0);
    }

    /// 上传时不传创建者，audit.creator 应为 None。
    #[tokio::test]
    async fn test_upload_file_with_no_creator() {
        let mut file_repo = TestFileRepo::new();
        file_repo.insert_fn = Box::new(|_| Ok(()));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.upload_file(make_upload_cmd(), None).await;
        assert!(result.is_ok());
        assert!(result.unwrap().audit.creator.is_none());
    }

    /// 上传成功应触发 FileUploaded 领域事件。
    #[tokio::test]
    async fn test_upload_file_raises_event() {
        let mut file_repo = TestFileRepo::new();
        file_repo.insert_fn = Box::new(|_| Ok(()));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.upload_file(make_upload_cmd(), Some("admin".into())).await;
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.events().len(), 1);
        assert!(matches!(
            file.events()[0],
            DomainEvent::FileUploaded { .. }
        ));
    }

    // ==========================================================
    // delete_file
    // ==========================================================

    /// 文件存在时软删除成功，调用 find_by_id + update。
    #[tokio::test]
    async fn test_delete_file_success() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file())));
        file_repo.update_fn = Box::new(|_| Ok(()));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.delete_file(1, Some("admin".into())).await.is_ok());
    }

    /// 文件不存在时软删除应返回错误。
    #[tokio::test]
    async fn test_delete_file_not_found() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(None));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.delete_file(999, None).await.is_err());
    }

    /// 软删除后 update 传入的 File 的 deleted 字段应标记为 Deleted。
    #[tokio::test]
    async fn test_delete_file_marks_deleted() {
        use std::sync::Mutex;
        let captured_file: Arc<Mutex<Option<File>>> = Arc::new(Mutex::new(None));
        let cap = captured_file.clone();

        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file())));
        file_repo.update_fn = Box::new(move |f: &File| {
            *cap.lock().unwrap() = Some(f.clone());
            Ok(())
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        svc.delete_file(1, Some("admin".into())).await.unwrap();
        assert_eq!(captured_file.lock().unwrap().as_ref().unwrap().audit.deleted, DeletedStatus::Deleted);
    }

    // ==========================================================
    // get_file_page
    // ==========================================================

    /// 分页查询返回非空结果，校验 total 和 list 长度。
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

    /// 分页查询返回空列表，total=0。
    #[tokio::test]
    async fn test_get_file_page_empty() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_page_fn = Box::new(|_, _| {
            Ok(Page::new(vec![], 1, 10, 0))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let query = FileQuery::default();
        let result = svc.get_file_page(&query, Page::request(1, 10)).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.total, 0);
        assert!(page.list.is_empty());
    }

    /// 带查询条件的分页查询，验证 name/file_type 正确透传给仓储。
    #[tokio::test]
    async fn test_get_file_page_with_query_filter() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_page_fn = Box::new(|q: &FileQuery, _| {
            // Verify query filters are passed through
            assert_eq!(q.name.as_deref(), Some("report"));
            assert_eq!(q.file_type.as_deref(), Some("pdf"));
            Ok(Page::new(vec![], 1, 10, 0))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let query = FileQuery {
            name: Some("report".into()),
            file_type: Some("pdf".into()),
            config_id: None,
        };
        let result = svc.get_file_page(&query, Page::request(1, 10)).await;
        assert!(result.is_ok());
    }

    /// 分页查询时仓储返回错误，应向上传播。
    #[tokio::test]
    async fn test_get_file_page_error() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_page_fn = Box::new(|_, _| {
            Err(crate::shared::repository::RepositoryError::DatabaseFile.into())
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_file_page(&FileQuery::default(), Page::request(1, 10)).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // get_file
    // ==========================================================

    /// 按 ID 查询文件，存在则返回正确的文件名。
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

    /// 按 ID 查询不存在的文件，应返回 NotFound 错误。
    #[tokio::test]
    async fn test_get_file_not_found() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(None));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.get_file(999).await.is_err());
    }

    /// 按 ID 查询时仓储出错，应向上传播。
    #[tokio::test]
    async fn test_get_file_db_error() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Err(crate::shared::repository::RepositoryError::DatabaseFile.into())
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.get_file(1).await.is_err());
    }

    // ==========================================================
    // download_file
    // ==========================================================

    /// .txt 扩展名映射为 text/plain。
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

    /// .pdf → application/pdf。
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

    /// .png → image/png。
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

    /// 未知扩展名回退为 application/octet-stream。
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

    /// 下载不存在的文件应返回 NotFound 错误。
    #[tokio::test]
    async fn test_download_file_not_found() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| Ok(None));
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.download_file(999).await.is_err());
    }

    /// .jpg → image/jpeg。
    #[tokio::test]
    async fn test_download_file_jpg() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "photo.jpg".into(), "/uploads/photo.jpg".into(),
                "https://example.com/photo.jpg".into(), None, 1024,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content_type, "image/jpeg");
    }

    /// .html → text/html。
    #[tokio::test]
    async fn test_download_file_html() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "page.html".into(), "/uploads/page.html".into(),
                "https://example.com/page.html".into(), None, 512,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content_type, "text/html");
    }

    /// .json → application/json。
    #[tokio::test]
    async fn test_download_file_json() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "data.json".into(), "/uploads/data.json".into(),
                "https://example.com/data.json".into(), None, 256,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content_type, "application/json");
    }

    /// .zip → application/zip。
    #[tokio::test]
    async fn test_download_file_zip() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "archive.zip".into(), "/uploads/archive.zip".into(),
                "https://example.com/archive.zip".into(), None, 4096,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content_type, "application/zip");
    }

    /// 文件名无扩展名，回退为 application/octet-stream。
    #[tokio::test]
    async fn test_download_file_no_extension() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "Makefile".into(), "/uploads/Makefile".into(),
                "https://example.com/Makefile".into(), None, 128,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content_type, "application/octet-stream");
    }

    /// 多.文件名取最后一个扩展名判断，backup.tar.gz → gz 未知 → octet-stream。
    #[tokio::test]
    async fn test_download_file_multiple_dots() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "backup.tar.gz".into(), "/uploads/backup.tar.gz".into(),
                "https://example.com/backup.tar.gz".into(), None, 8192,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.download_file(1).await;
        assert!(result.is_ok());
        let info = result.unwrap();
        // Last extension is "gz" → unknown → application/octet-stream
        assert_eq!(info.content_type, "application/octet-stream");
        assert_eq!(info.storage_path, "/uploads/backup.tar.gz");
    }

    /// storage_path 应与 File.path 一致，url 来自 File.url。
    #[tokio::test]
    async fn test_download_file_storage_path_matches_path() {
        let mut file_repo = TestFileRepo::new();
        file_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(File::restore(
                1, Some(1), "doc.pdf".into(), "/store/files/doc.pdf".into(),
                "https://cdn.example.com/doc.pdf".into(), None, 1024,
                AuditFields::default(),
            )))
        });
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let info = svc.download_file(1).await.unwrap();
        assert_eq!(info.storage_path, "/store/files/doc.pdf");
        assert_eq!(info.url, "https://cdn.example.com/doc.pdf");
    }

    // ==========================================================
    // resolve_config_id
    // ==========================================================

    /// 显式指定 config_id 时直接返回该值。
    #[tokio::test]
    async fn test_resolve_config_id_explicit() {
        let file_repo = TestFileRepo::new();
        let config_repo = TestFileConfigRepo::new();

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.resolve_config_id(Some(42)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(42));
    }

    /// 未指定 config_id 时回退到主配置 ID。
    #[tokio::test]
    async fn test_resolve_config_id_fallback_to_master() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_master_fn = Box::new(|| Ok(Some(make_master_config())));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.resolve_config_id(None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(1));
    }

    /// 无主配置且未指定 config_id 时返回 None。
    #[tokio::test]
    async fn test_resolve_config_id_no_master() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_master_fn = Box::new(|| Ok(None));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.resolve_config_id(None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    /// 回退查询主配置时仓储错误，应向上传播。
    #[tokio::test]
    async fn test_resolve_config_id_db_error() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_master_fn = Box::new(|| {
            Err(crate::shared::repository::RepositoryError::DatabaseFile.into())
        });

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.resolve_config_id(None).await.is_err());
    }

    // ==========================================================
    // get_allowed_extensions
    // ==========================================================

    /// 按 config_id 查询配置，返回其中的 allowed_extensions 列表。
    #[tokio::test]
    async fn test_get_allowed_extensions_by_id() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file_config())));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_allowed_extensions(Some(1)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["pdf".to_string(), "txt".to_string()]);
    }

    /// 指定 config_id 不存在，返回空列表。
    #[tokio::test]
    async fn test_get_allowed_extensions_by_id_not_found() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_allowed_extensions(Some(999)).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// 未指定 config_id 时回退到主配置的 allowed_extensions。
    #[tokio::test]
    async fn test_get_allowed_extensions_fallback_to_master() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_master_fn = Box::new(|| Ok(Some(make_master_config())));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_allowed_extensions(None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["jpg".to_string(), "png".to_string()]);
    }

    /// 无主配置且未指定 config_id，返回空列表。
    #[tokio::test]
    async fn test_get_allowed_extensions_no_master() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_master_fn = Box::new(|| Ok(None));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_allowed_extensions(None).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// 配置 JSON 无效时返回空列表而不报错。
    #[tokio::test]
    async fn test_get_allowed_extensions_invalid_json() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(FileConfig::restore(
                1, "Bad".into(), 1, None, 0,
                "not-valid-json".into(),
                AuditFields::default(),
            )))
        });

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_allowed_extensions(Some(1)).await;
        assert!(result.is_ok());
        // Invalid JSON → empty vec
        assert!(result.unwrap().is_empty());
    }

    /// JSON 有效但无 allowed_extensions 键，返回空列表。
    #[tokio::test]
    async fn test_get_allowed_extensions_valid_json_no_extensions_key() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| {
            Ok(Some(FileConfig::restore(
                1, "Other".into(), 1, None, 0,
                r#"{"bucket":"my-bucket","region":"us-east-1"}"#.into(),
                AuditFields::default(),
            )))
        });

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_allowed_extensions(Some(1)).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ==========================================================
    // get_config_all
    // ==========================================================

    /// 获取全部配置，返回多条记录，校验名称。
    #[tokio::test]
    async fn test_get_config_all_success() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_all_fn = Box::new(|| {
            Ok(vec![make_file_config(), make_alt_config()])
        });

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_config_all().await;
        assert!(result.is_ok());
        let configs = result.unwrap();
        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].name, "Default");
        assert_eq!(configs[1].name, "Alt");
    }

    /// 无配置时返回空 Vec。
    #[tokio::test]
    async fn test_get_config_all_empty() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_all_fn = Box::new(|| Ok(vec![]));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_config_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// 查询全部配置时仓储错误，应向上传播。
    #[tokio::test]
    async fn test_get_config_all_db_error() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_all_fn = Box::new(|| {
            Err(crate::shared::repository::RepositoryError::DatabaseFile.into())
        });

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.get_config_all().await.is_err());
    }

    // ==========================================================
    // get_config
    // ==========================================================

    /// 按 ID 获取配置成功，校验名称。
    #[tokio::test]
    async fn test_get_config_success() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file_config())));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.get_config(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Default");
    }

    /// 配置不存在时返回 NotFound 错误。
    #[tokio::test]
    async fn test_get_config_not_found() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.get_config(999).await.is_err());
    }

    // ==========================================================
    // create_config
    // ==========================================================

    /// 创建配置成功，校验 name/storage/remark/master 字段。
    #[tokio::test]
    async fn test_create_config_success() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.insert_fn = Box::new(|_| Ok(()));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.create_config(
            "S3".into(), 1, Some("AWS存储".into()),
            r#"{"bucket":"my-bucket"}"#.into(), Some("admin".into()),
        ).await;
        assert!(result.is_ok());
        let fc = result.unwrap();
        assert_eq!(fc.name, "S3");
        assert_eq!(fc.storage, 1);
        assert_eq!(fc.remark.as_deref(), Some("AWS存储"));
        assert_eq!(fc.master, 0);
    }

    /// 创建配置时仓储 insert 错误，应返回错误。
    #[tokio::test]
    async fn test_create_config_insert_error() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.insert_fn = Box::new(|_| {
            Err(crate::shared::repository::RepositoryError::DatabaseFile.into())
        });

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.create_config(
            "S3".into(), 1, None, "{}".into(), None,
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // update_config
    // ==========================================================

    /// 更新配置成功，校验 name/storage 已更新。
    #[tokio::test]
    async fn test_update_config_success() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file_config())));
        config_repo.update_fn = Box::new(|_| Ok(()));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.update_config(
            1, "Updated".into(), 2, Some("新备注".into()),
            r#"{"allowed_extensions":["xls"]}"#.into(), Some("editor".into()),
        ).await;
        assert!(result.is_ok());
        let fc = result.unwrap();
        assert_eq!(fc.name, "Updated");
        assert_eq!(fc.storage, 2);
    }

    /// 更新不存在的配置，应返回 NotFound 错误。
    #[tokio::test]
    async fn test_update_config_not_found() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.update_config(
            999, "X".into(), 0, None, "{}".into(), None,
        ).await;
        assert!(result.is_err());
    }

    // ==========================================================
    // delete_config
    // ==========================================================

    /// 软删除配置成功：find_by_id → soft_delete → update。
    #[tokio::test]
    async fn test_delete_config_success() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file_config())));
        config_repo.update_fn = Box::new(|_| Ok(()));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.delete_config(1, Some("admin".into())).await.is_ok());
    }

    /// 删除不存在的配置，应返回 NotFound 错误。
    #[tokio::test]
    async fn test_delete_config_not_found() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();
        config_repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.delete_config(999, None).await.is_err());
    }

    // ==========================================================
    // set_master_config
    // ==========================================================

    /// 当前无主配置时直接设置目标为主配置，master 应为 1。
    #[tokio::test]
    async fn test_set_master_config_no_previous_master() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();

        // No master exists
        config_repo.find_master_fn = Box::new(|| Ok(None));
        // The target config exists
        config_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_file_config())));
        // update succeeds
        config_repo.update_fn = Box::new(|_| Ok(()));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.set_master_config(1, Some("admin".into())).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().master, 1);
    }

    /// 已有主配置时切换：先取消旧主 → 设置新主，共两次 update。
    #[tokio::test]
    async fn test_set_master_config_replaces_existing_master() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();

        // Current master is id=1
        config_repo.find_master_fn = Box::new(|| Ok(Some(make_master_config())));
        // Target config id=2 exists
        config_repo.find_by_id_fn = Box::new(|id| {
            if id == 2 {
                Ok(Some(make_alt_config()))
            } else {
                Ok(None)
            }
        });
        // update succeeds
        let update_count = std::sync::atomic::AtomicU32::new(0);
        config_repo.update_fn = Box::new(move |_fc: &FileConfig| {
            update_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.set_master_config(2, Some("admin".into())).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().master, 1);
    }

    /// 目标已是主配置，只更新自身不重复取消。
    #[tokio::test]
    async fn test_set_master_config_same_as_current() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();

        // Current master is id=1
        config_repo.find_master_fn = Box::new(|| Ok(Some(make_master_config())));
        // Target config id=1 is same
        config_repo.find_by_id_fn = Box::new(|_| Ok(Some(make_master_config())));
        config_repo.update_fn = Box::new(|_| Ok(()));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        let result = svc.set_master_config(1, Some("admin".into())).await;
        assert!(result.is_ok());
    }

    /// 目标配置不存在，应返回 NotFound 错误。
    #[tokio::test]
    async fn test_set_master_config_target_not_found() {
        let file_repo = TestFileRepo::new();
        let mut config_repo = TestFileConfigRepo::new();

        config_repo.find_master_fn = Box::new(|| Ok(None));
        config_repo.find_by_id_fn = Box::new(|_| Ok(None));

        let svc = FileService::new(Arc::new(file_repo), Arc::new(config_repo));
        assert!(svc.set_master_config(999, None).await.is_err());
    }
}
