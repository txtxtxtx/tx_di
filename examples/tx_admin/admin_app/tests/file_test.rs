//! 文件管理集成测试
//!
//! 覆盖功能（参照 04-功能清单.md 第6节）:
//!   6.1 文件上传     ✅
//!   6.5 文件元数据   ✅

mod common;
use admin_app::file::dto::*;

// ── 文件上传 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn upload_file_success() {
    let (app, _, _) = common::create_file_app();
    let cmd = UploadFileCommand {
        name: "report.pdf".into(),
        path: "/uploads/2024/report.pdf".into(),
        url: "https://cdn.example.com/uploads/2024/report.pdf".into(),
        file_type: Some("application/pdf".into()),
        size: 102400,
        config_id: Some(1),
    };
    let file = app.upload_file(cmd, Some("admin".into())).await.unwrap();
    assert_eq!(file.name, "report.pdf");
    assert_eq!(file.size, 102400);
    assert_eq!(file.file_type, Some("application/pdf".into()));
    assert_eq!(file.path, "/uploads/2024/report.pdf");
}

#[tokio::test]
async fn upload_file_minimal() {
    let (app, _, _) = common::create_file_app();
    let file = app.upload_file(UploadFileCommand {
        name: "photo.jpg".into(),
        path: "/uploads/photos/1.jpg".into(),
        url: "/uploads/photos/1.jpg".into(),
        file_type: None,
        size: 51200,
        config_id: None,
    }, Some("admin".into())).await.unwrap();
    assert_eq!(file.name, "photo.jpg");
}

// ── 文件查询 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn paginate_files() {
    let (app, _, _) = common::create_file_app();
    for i in 0..5 {
        app.upload_file(UploadFileCommand {
            name: format!("file{}.txt", i),
            path: format!("/uploads/f{}.txt", i),
            url: format!("/uploads/f{}.txt", i),
            file_type: Some("text/plain".into()),
            size: 100,
            config_id: None,
        }, Some("admin".into())).await.unwrap();
    }
    let page = app.get_file_page(FileQueryRequest {
        name: None, file_type: None, config_id: None, page: 1, page_size: 2,
    }).await.unwrap();
    assert_eq!(page.list.len(), 2);
    assert_eq!(page.total, 5);
}

#[tokio::test]
async fn get_file_detail() {
    let (app, _, _) = common::create_file_app();
    let f = app.upload_file(UploadFileCommand {
        name: "detail.txt".into(),
        path: "/uploads/detail.txt".into(),
        url: "https://cdn.example.com/detail.txt".into(),
        file_type: Some("text/plain".into()),
        size: 99,
        config_id: Some(2),
    }, Some("admin".into())).await.unwrap();

    let found = app.get_file(f.id).await.unwrap();
    assert_eq!(found.name, "detail.txt");
    assert_eq!(found.size, 99);
    assert_eq!(found.config_id, Some(2));
}

#[tokio::test]
async fn query_files_by_type() {
    let (app, _, _) = common::create_file_app();
    app.upload_file(UploadFileCommand {
        name: "doc.pdf".into(), path: "/p/doc.pdf".into(), url: "/p/doc.pdf".into(),
        file_type: Some("application/pdf".into()), size: 1000, config_id: None,
    }, Some("admin".into())).await.unwrap();
    app.upload_file(UploadFileCommand {
        name: "img.png".into(), path: "/p/img.png".into(), url: "/p/img.png".into(),
        file_type: Some("image/png".into()), size: 2000, config_id: None,
    }, Some("admin".into())).await.unwrap();

    let page = app.get_file_page(FileQueryRequest {
        name: None, file_type: Some("application/pdf".into()), config_id: None,
        page: 1, page_size: 10,
    }).await.unwrap();
    assert_eq!(page.list.len(), 1);
    assert_eq!(page.list[0].name, "doc.pdf");
}

// ── 文件删除 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_file() {
    let (app, _, _) = common::create_file_app();
    let f = app.upload_file(UploadFileCommand {
        name: "todelete.txt".into(),
        path: "/uploads/todel.txt".into(),
        url: "/uploads/todel.txt".into(),
        file_type: None,
        size: 10,
        config_id: None,
    }, Some("admin".into())).await.unwrap();

    app.delete_file(f.id, Some("admin".into())).await.unwrap();
    assert!(app.get_file(f.id).await.is_err());
}
