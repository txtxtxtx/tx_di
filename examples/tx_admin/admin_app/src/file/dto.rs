use std::pin::Pin;
use tokio::io::AsyncRead;
use admin_proto::FileResponse;
use admin_domain::file::model::aggregate::File;

/// 领域模型 → Proto 响应：文件
pub fn file_to_response(file: File) -> FileResponse {
    FileResponse {
        id: file.id,
        config_id: file.config_id,
        name: file.name,
        path: file.path,
        url: file.url,
        file_type: file.file_type,
        size: file.size,
    }
}

/// 流式下载结果（不缓冲文件内容到内存）
pub struct DownloadFileStream {
    /// 可异步读取的文件数据流
    pub reader: Pin<Box<dyn AsyncRead + Send + Unpin>>,
    /// 原始文件名
    pub filename: String,
    /// MIME 类型
    pub content_type: String,
    /// 文件字节大小
    pub size: u64,
}
