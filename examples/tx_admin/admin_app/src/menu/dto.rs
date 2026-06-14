use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMenuCommand {
    pub name: String,
    pub permission: String,
    pub types: i32,
    pub sort: i32,
    pub parent_id: u64,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub path: Option<String>,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub icon: Option<String>,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub component: Option<String>,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub component_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMenuCommand {
    pub menu_id: u64,
    pub name: String,
    pub permission: String,
    pub types: i32,
    pub sort: i32,
    pub parent_id: u64,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub path: Option<String>,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub icon: Option<String>,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub component: Option<String>,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub component_name: Option<String>,
    pub visible: i32,
    pub keep_alive: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuQueryRequest {
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub name: Option<String>,
    pub status: Option<i32>,
    pub types: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MenuResponse {
    pub id: u64,
    pub name: String,
    pub permission: String,
    pub types: i32,
    pub sort: i32,
    pub parent_id: u64,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub component: Option<String>,
    pub component_name: Option<String>,
    pub status: i32,
    pub visible: i32,
    pub keep_alive: i32,
}

impl From<admin_domain::menu::model::aggregate::Menu> for MenuResponse {
    fn from(menu: admin_domain::menu::model::aggregate::Menu) -> Self {
        Self {
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
}
