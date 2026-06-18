use admin_domain::menu::model::aggregate::Menu;

// Re-export proto types directly (no hand-written DTOs)
pub use admin_proto::{CreateMenuRequest, UpdateMenuRequest, ListMenusRequest, MenuResponse};

/// 将领域层的 Menu 聚合根转换为 proto 的 MenuResponse
pub fn menu_to_response(menu: Menu) -> MenuResponse {
    MenuResponse {
        id: menu.id,
        name: menu.name,
        permission: menu.permission,
        types: menu.types,
        sort: menu.sort,
        parent_id: menu.parent_id,
        path: menu.path,
        icon: menu.icon,
        component: menu.component,
        component_name: menu.component_name,
        status: menu.status,
        visible: menu.visible,
        keep_alive: menu.keep_alive,
    }
}
