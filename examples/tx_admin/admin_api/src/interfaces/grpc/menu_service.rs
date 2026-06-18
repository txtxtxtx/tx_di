//! 菜单管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::menu::menu_service_server::MenuService;
use admin_proto::admin::menu::{
    CreateMenuRequest, MenuResponse, UpdateMenuRequest, DeleteMenuRequest,
    GetMenuRequest, ListMenusRequest, ListMenusResponse,
};
use admin_proto::Empty;

#[derive(Debug, Default)]
pub struct MenuGrpcService;

#[tonic::async_trait]
impl MenuService for MenuGrpcService {
    async fn create_menu(&self, request: Request<CreateMenuRequest>) -> Result<Response<MenuResponse>, Status> {
        let req = request.into_inner();
        services::get().menu.create_menu(req, None).await
            .map(|r| Response::new(r))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_menu(&self, request: Request<UpdateMenuRequest>) -> Result<Response<MenuResponse>, Status> {
        let req = request.into_inner();
        services::get().menu.update_menu(req, None).await
            .map(|r| Response::new(r))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn delete_menu(&self, request: Request<DeleteMenuRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().menu.delete_menu(req.menu_id, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_menu(&self, request: Request<GetMenuRequest>) -> Result<Response<MenuResponse>, Status> {
        let req = request.into_inner();
        let query = ListMenusRequest { name: None, status: None, types: None };
        services::get().menu.get_menu_list(query).await
            .map(|list| {
                let found = list.into_iter().find(|m| m.id == req.menu_id)
                    .expect("menu not found");
                Response::new(found)
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_menus(&self, request: Request<ListMenusRequest>) -> Result<Response<ListMenusResponse>, Status> {
        let req = request.into_inner();
        services::get().menu.get_menu_list(req).await
            .map(|list| Response::new(ListMenusResponse {
                items: list,
            }))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
