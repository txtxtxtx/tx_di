//! 菜单管理 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::menu::menu_service_server::MenuService;
use admin_proto::admin::menu::{
    CreateMenuRequest, DeleteMenuRequest, GetMenuRequest, ListMenusRequest, ListMenusResponse,
    MenuResponse, UpdateMenuRequest,
};
use admin_proto::Empty;
use tx_di_core::App;

use super::auth_interceptor::{self, get_login_id};
use super::err;

#[derive(Clone)]
pub struct MenuGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl MenuService for MenuGrpcService {
    async fn create_menu(
        &self,
        request: Request<CreateMenuRequest>,
    ) -> Result<Response<MenuResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "menu:create").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::menu::app_service::MenuAppService> = self.app.inject();
        let r = svc.create_menu(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn update_menu(
        &self,
        request: Request<UpdateMenuRequest>,
    ) -> Result<Response<MenuResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "menu:update").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::menu::app_service::MenuAppService> = self.app.inject();
        let r = svc.update_menu(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn delete_menu(
        &self,
        request: Request<DeleteMenuRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "menu:delete").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::menu::app_service::MenuAppService> = self.app.inject();
        svc.delete_menu(req.menu_id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_menu(
        &self,
        request: Request<GetMenuRequest>,
    ) -> Result<Response<MenuResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "menu:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::menu::app_service::MenuAppService> = self.app.inject();
        let query = ListMenusRequest {
            name: None,
            status: None,
            types: None,
        };
        let list = svc.get_menu_list(query).await.map_err(err::to_status)?;
        let found = list
            .into_iter()
            .find(|m| m.id == req.menu_id)
            .ok_or_else(|| Status::not_found("menu not found"))?;
        Ok(Response::new(found))
    }

    async fn list_menus(
        &self,
        request: Request<ListMenusRequest>,
    ) -> Result<Response<ListMenusResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "menu:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::menu::app_service::MenuAppService> = self.app.inject();
        let list = svc.get_menu_list(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListMenusResponse { items: list }))
    }
}
