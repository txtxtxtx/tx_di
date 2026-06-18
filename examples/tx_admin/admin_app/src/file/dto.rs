use admin_proto::{FileResponse, DownloadFileResponse};

/// 领域模型 → Proto 响应：文件
pub fn file_to_response(file: admin_domain::file::model::aggregate::File) -> FileResponse {
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

/// 领域模型 → Proto 响应：文件下载
pub fn file_download_to_response(info: admin_domain::file::model::value_object::FileDownloadInfo) -> DownloadFileResponse {
    DownloadFileResponse {
        url: info.url,
        filename: info.filename,
        size: info.size as u64,
        content_type: info.content_type,
    }
}
