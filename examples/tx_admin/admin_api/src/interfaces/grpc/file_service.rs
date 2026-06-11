//! 文件管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::file::file_service_server::FileService;
use admin_proto::admin::file::{
    UploadFileRequest, FileResponse, DeleteFileRequest, GetFileRequest,
    ListFilesRequest, ListFilesResponse,
};
use admin_proto::Empty;

/// 文件 gRPC 服务
#[derive(Debug, Default)]
pub struct FileGrpcService;

#[tonic::async_trait]
impl FileService for FileGrpcService {
    async fn upload_file(
        &self,
        request: Request<UploadFileRequest>,
    ) -> Result<Response<FileResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 FileAppService::upload
        let resp = FileResponse {
            id: 1,
            config_id: req.config_id,
            name: req.name.clone(),
            path: req.path.clone(),
            url: req.url.clone(),
            file_type: req.file_type.clone(),
            size: req.size,
        };
        Ok(Response::new(resp))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        // TODO: 调用 FileAppService::delete
        let _ = req.file_id;
        Ok(Response::new(Empty {}))
    }

    async fn get_file(
        &self,
        request: Request<GetFileRequest>,
    ) -> Result<Response<FileResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 FileAppService::get_by_id
        let resp = FileResponse {
            id: req.file_id,
            config_id: None,
            name: "placeholder".into(),
            path: String::new(),
            url: String::new(),
            file_type: None,
            size: 0,
        };
        Ok(Response::new(resp))
    }

    async fn list_files(
        &self,
        _request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        // TODO: 调用 FileAppService::list
        let resp = ListFilesResponse {
            items: vec![],
            page_info: None,
        };
        Ok(Response::new(resp))
    }
}
