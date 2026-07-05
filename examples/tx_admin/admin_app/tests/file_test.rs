//! 文件管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第6节）:
//!   6.1 文件上传     ✅ 流式上传 + 字节校验
//!   6.5 文件元数据   ✅ 分页 / 详情 / 按类型查询
//!   6.6 文件删除     ✅ 物理文件 + DB 软删除

mod common;

use std::io::Cursor;

use admin_proto::ListFilesRequest;
use admin_app::file::app_service::FileAppService;

/// 便捷上传辅助函数 — 从字节切片创建流式上传
async fn upload_bytes(
    app: &FileAppService,
    filename: &str,
    content_type: &str,
    data: &[u8],
    config_id: Option<u64>,
) -> admin_proto::FileResponse {
    let mut cursor = Cursor::new(data);
    app.upload_file_stream(
        filename.to_string(),
        content_type.to_string(),
        &mut cursor,
        config_id,
        Some("admin".into()),
    )
    .await
    .unwrap()
}

// ── 文件上传 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn upload_file_success() {
    let (app, _, _, _temp) = common::create_file_app().await;
    let data = b"This is a test PDF content for upload verification.";
    let file = upload_bytes(&app, "report.pdf", "application/pdf", data, Some(1)).await;
    assert_eq!(file.name, "report.pdf");
    assert!(file.size > 0);
    assert_eq!(file.file_type, Some("application/pdf".into()));
    assert!(!file.path.is_empty());
}

#[tokio::test]
async fn upload_file_auto_mime() {
    let (app, _, _, _temp) = common::create_file_app().await;
    let data = b"PNG image placeholder";
    // 传 application/octet-stream，期望自动推断
    let file = upload_bytes(&app, "photo.png", "application/octet-stream", data, None).await;
    assert_eq!(file.name, "photo.png");
    // 应从扩展名推断 MIME
    assert_eq!(file.file_type, Some("image/png".into()));
}

// ── 文件查询 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn paginate_files() {
    let (app, _, _, _temp) = common::create_file_app().await;
    for i in 0..5 {
        let data = format!("file{} content", i).into_bytes();
        upload_bytes(&app, &format!("file{}.txt", i), "text/plain", &data, None).await;
    }
    let page = app
        .get_file_page(ListFilesRequest {
            name: None,
            file_type: None,
            config_id: None,
            page: 1,
            page_size: 2,
        })
        .await
        .unwrap();
    assert_eq!(page.list.len(), 2);
    assert_eq!(page.total, 5);
}

#[tokio::test]
async fn get_file_detail() {
    let (app, _, _, _temp) = common::create_file_app().await;
    let data = b"detail file";
    let f = upload_bytes(&app, "detail.txt", "text/plain", data, Some(2)).await;

    let found = app.get_file(f.id).await.unwrap();
    assert_eq!(found.name, "detail.txt");
    assert!(found.size > 0);
    assert_eq!(found.config_id, Some(2));
}

#[tokio::test]
async fn query_files_by_type() {
    let (app, _, _, _temp) = common::create_file_app().await;
    upload_bytes(&app, "doc.pdf", "application/pdf", b"pdf data", None).await;
    upload_bytes(&app, "img.png", "image/png", b"png data", None).await;

    let page = app
        .get_file_page(ListFilesRequest {
            name: None,
            file_type: Some("application/pdf".into()),
            config_id: None,
            page: 1,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].name, "doc.pdf");
}

// ── 文件删除 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_file() {
    let (app, _, _, _temp) = common::create_file_app().await;
    let data = b"file to be deleted";
    let f = upload_bytes(&app, "todelete.txt", "text/plain", data, None).await;

    app.delete_file(f.id, Some("admin".into())).await.unwrap();
    // 软删除后 find_by_id 返回 None
    assert!(app.get_file(f.id).await.is_err());
}

// ── 流式下载 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn download_file_stream_roundtrip() {
    let (app, _, _, _temp) = common::create_file_app().await;
    let data = b"hello world from streaming test!";
    let f = upload_bytes(&app, "stream_test.txt", "text/plain", data, None).await;

    let stream = app.download_file_stream(f.id).await.unwrap();
    assert_eq!(stream.filename, "stream_test.txt");
    assert_eq!(stream.content_type, "text/plain");
    assert_eq!(stream.size, data.len() as u64);

    // 读取全部内容并校验
    use tokio::io::AsyncReadExt;
    let mut reader = stream.reader;
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.unwrap();
    assert_eq!(buf, data);
}
