//! 文件管理 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::file::file_service_server::FileService;
use admin_proto::admin::file::{
    DeleteFileRequest, DownloadFileRequest, DownloadFileResponse, FileResponse, GetFileRequest,
    ListFilesRequest, ListFilesResponse, UploadFileRequest,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use tx_di_core::App;

use super::auth_interceptor::{self, get_login_id};
use super::err;

#[derive(Clone)]
pub struct FileGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl FileService for FileGrpcService {
    async fn upload_file(
        &self,
        request: Request<UploadFileRequest>,
    ) -> Result<Response<FileResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "file:upload").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::file::app_service::FileAppService> = self.app.inject();

        // gRPC 上传通过元数据描述文件，使用空 reader 上传
        let content_type = req.file_type.clone().unwrap_or_default();
        let mut empty_reader: &[u8] = &[];
        let r = svc
            .upload_file_stream(
                req.name,
                content_type,
                &mut empty_reader,
                req.config_id,
                Some(login_id),
            )
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "file:delete").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::file::app_service::FileAppService> = self.app.inject();
        svc.delete_file(req.file_id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_file(
        &self,
        request: Request<GetFileRequest>,
    ) -> Result<Response<FileResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "file:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::file::app_service::FileAppService> = self.app.inject();
        let r = svc.get_file(req.file_id).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "file:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::file::app_service::FileAppService> = self.app.inject();
        let p = svc.get_file_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListFilesResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }

    async fn download_file(
        &self,
        request: Request<DownloadFileRequest>,
    ) -> Result<Response<DownloadFileResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "file:download").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::file::app_service::FileAppService> = self.app.inject();

        // 获取预览 URL 作为下载链接
        let preview = svc
            .get_preview_url(req.file_id)
            .await
            .map_err(err::to_status)?;

        // 获取文件元数据
        let file = svc.get_file(req.file_id).await.map_err(err::to_status)?;

        Ok(Response::new(DownloadFileResponse {
            url: preview.url,
            filename: file.name,
            size: file.size as u64,
            content_type: file.file_type.unwrap_or_else(|| "application/octet-stream".to_string()),
        }))
    }
}
