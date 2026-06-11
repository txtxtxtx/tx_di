//! 文件管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::file::file_service_server::FileService;
use admin_proto::admin::file::{
    UploadFileRequest, FileResponse, DeleteFileRequest, GetFileRequest,
    ListFilesRequest, ListFilesResponse,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use crate::services;

#[derive(Debug, Default)]
pub struct FileGrpcService;

fn map_file(f: admin_app::file::dto::FileResponse) -> FileResponse {
    FileResponse {
        id: f.id, config_id: f.config_id, name: f.name,
        path: f.path, url: f.url, file_type: f.file_type, size: f.size,
    }
}

#[tonic::async_trait]
impl FileService for FileGrpcService {
    async fn upload_file(&self, request: Request<UploadFileRequest>) -> Result<Response<FileResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::file::dto::UploadFileCommand {
            name: req.name, path: req.path, url: req.url,
            file_type: req.file_type, size: req.size, config_id: req.config_id,
        };
        services::get().file.upload_file(cmd, None).await
            .map(|r| Response::new(map_file(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn delete_file(&self, request: Request<DeleteFileRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().file.delete_file(req.file_id, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_file(&self, request: Request<GetFileRequest>) -> Result<Response<FileResponse>, Status> {
        let req = request.into_inner();
        services::get().file.get_file(req.file_id).await
            .map(|r| Response::new(map_file(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_files(&self, request: Request<ListFilesRequest>) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();
        let query = admin_app::file::dto::FileQueryRequest {
            name: req.name, file_type: req.file_type,
            config_id: req.config_id, page: req.page, size: req.page_size,
        };
        services::get().file.get_file_page(query).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size; let total_pages = p.total_pages();
                let items = p.list.into_iter().map(map_file).collect();
                Response::new(ListFilesResponse { items, page_info: Some(PageResponse { total, page, size, total_pages }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }
}
