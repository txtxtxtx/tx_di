//! 菜单管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::menu::menu_service_server::MenuService;
use admin_proto::admin::menu::{
    CreateMenuRequest, MenuResponse, UpdateMenuRequest, DeleteMenuRequest,
    GetMenuRequest, ListMenusRequest, ListMenusResponse,
};
use admin_proto::Empty;

/// 菜单 gRPC 服务
#[derive(Debug, Default)]
pub struct MenuGrpcService;

#[tonic::async_trait]
impl MenuService for MenuGrpcService {
    async fn create_menu(
        &self,
        request: Request<CreateMenuRequest>,
    ) -> Result<Response<MenuResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 MenuAppService::create
        let resp = MenuResponse {
            id: 1,
            name: req.name.clone(),
            permission: req.permission.clone(),
            types: req.types,
            sort: req.sort,
            parent_id: req.parent_id,
            path: req.path.clone(),
            icon: req.icon.clone(),
            component: req.component.clone(),
            component_name: req.component_name.clone(),
            status: 1,
            visible: 1,
            keep_alive: 0,
        };
        Ok(Response::new(resp))
    }

    async fn update_menu(
        &self,
        request: Request<UpdateMenuRequest>,
    ) -> Result<Response<MenuResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 MenuAppService::update
        let resp = MenuResponse {
            id: req.menu_id,
            name: req.name.clone(),
            permission: req.permission.clone(),
            types: req.types,
            sort: req.sort,
            parent_id: req.parent_id,
            path: req.path.clone(),
            icon: req.icon.clone(),
            component: req.component.clone(),
            component_name: req.component_name.clone(),
            status: 1,
            visible: req.visible,
            keep_alive: req.keep_alive,
        };
        Ok(Response::new(resp))
    }

    async fn delete_menu(
        &self,
        request: Request<DeleteMenuRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        // TODO: 调用 MenuAppService::delete
        let _ = req.menu_id;
        Ok(Response::new(Empty {}))
    }

    async fn get_menu(
        &self,
        request: Request<GetMenuRequest>,
    ) -> Result<Response<MenuResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 MenuAppService::get_by_id
        let resp = MenuResponse {
            id: req.menu_id,
            name: "placeholder".into(),
            permission: String::new(),
            types: 0,
            sort: 0,
            parent_id: 0,
            path: None,
            icon: None,
            component: None,
            component_name: None,
            status: 1,
            visible: 1,
            keep_alive: 0,
        };
        Ok(Response::new(resp))
    }

    async fn list_menus(
        &self,
        _request: Request<ListMenusRequest>,
    ) -> Result<Response<ListMenusResponse>, Status> {
        // TODO: 调用 MenuAppService::list
        let resp = ListMenusResponse { items: vec![] };
        Ok(Response::new(resp))
    }
}
