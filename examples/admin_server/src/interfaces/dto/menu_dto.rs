//! 菜单 DTO

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MenuDto {
    pub id: u64, pub name: String, pub permission: Option<String>, pub menu_type: String,
    pub sort: i32, pub parent_id: u64, pub route_path: Option<String>, pub icon: Option<String>,
    pub component: Option<String>, pub visible: bool, pub created_at: String,
}

impl From<&crate::domain::menu::Menu> for MenuDto {
    fn from(m: &crate::domain::menu::Menu) -> Self {
        Self { id: m.id, name: m.name.clone(), permission: m.permission.clone(), menu_type: m.menu_type.to_string(),
            sort: m.sort, parent_id: m.parent_id, route_path: m.route_path.clone(), icon: m.icon.clone(),
            component: m.component.clone(), visible: m.visible,
            created_at: m.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string() }
    }
}

#[derive(Debug, Serialize)]
pub struct PermissionTreeNode {
    pub id: u64, pub parent_id: u64, pub name: String, pub permission: Option<String>,
    pub menu_type: String, pub sort: i32, pub icon: Option<String>, pub route_path: Option<String>,
    pub component: Option<String>, pub visible: bool, pub children: Vec<PermissionTreeNode>,
}
