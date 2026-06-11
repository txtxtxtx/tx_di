//! 菜单管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::menu::menu_service_server::MenuService;
use admin_proto::admin::menu::{
    CreateMenuRequest, MenuResponse, UpdateMenuRequest, DeleteMenuRequest,
    GetMenuRequest, ListMenusRequest, ListMenusResponse,
};
use admin_proto::Empty;
use crate::services;

#[derive(Debug, Default)]
pub struct MenuGrpcService;

fn map_menu(m: admin_app::menu::dto::MenuResponse) -> MenuResponse {
    MenuResponse {
        id: m.id, name: m.name, permission: m.permission,
        types: m.types, sort: m.sort, parent_id: m.parent_id,
        path: m.path, icon: m.icon, component: m.component,
        component_name: m.component_name, status: m.status,
        visible: m.visible, keep_alive: m.keep_alive,
    }
}

#[tonic::async_trait]
impl MenuService for MenuGrpcService {
    async fn create_menu(&self, request: Request<CreateMenuRequest>) -> Result<Response<MenuResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::menu::dto::CreateMenuCommand {
            name: req.name, permission: req.permission, types: req.types,
            sort: req.sort, parent_id: req.parent_id, path: req.path,
            icon: req.icon, component: req.component, component_name: req.component_name,
        };
        services::get().menu.create_menu(cmd, None).await
            .map(|r| Response::new(map_menu(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_menu(&self, request: Request<UpdateMenuRequest>) -> Result<Response<MenuResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::menu::dto::UpdateMenuCommand {
            menu_id: req.menu_id, name: req.name, permission: req.permission,
            types: req.types, sort: req.sort, parent_id: req.parent_id,
            path: req.path, icon: req.icon, component: req.component,
            component_name: req.component_name, visible: req.visible, keep_alive: req.keep_alive,
        };
        services::get().menu.update_menu(cmd, None).await
            .map(|r| Response::new(map_menu(r)))
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
        let query = admin_app::menu::dto::MenuQueryRequest { name: None, status: None, types: None };
        services::get().menu.get_menu_list(query).await
            .map(|list| {
                let found = list.into_iter().find(|m| m.id == req.menu_id)
                    .expect("menu not found");
                Response::new(map_menu(found))
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_menus(&self, request: Request<ListMenusRequest>) -> Result<Response<ListMenusResponse>, Status> {
        let req = request.into_inner();
        let query = admin_app::menu::dto::MenuQueryRequest {
            name: req.name, status: req.status, types: req.types,
        };
        services::get().menu.get_menu_list(query).await
            .map(|list| Response::new(ListMenusResponse {
                items: list.into_iter().map(map_menu).collect(),
            }))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
