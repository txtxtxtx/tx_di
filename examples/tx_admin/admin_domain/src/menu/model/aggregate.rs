use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent};
use crate::AggregateRoot;

/// Menu aggregate root
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
/// 菜单结构体，用于存储菜单相关的信息
/// 包含菜单的基本信息、权限、样式配置以及子菜单等
pub struct Menu {
    /// 菜单ID，使用64位无符号整数表示
    pub id: u64,
    /// 菜单名称，使用字符串类型存储
    pub name: String,
    /// 菜单权限标识，使用字符串类型存储
    pub permission: String,
    /// 菜单类型，使用32位整数表示
    pub types: i32,
    /// 菜单排序，使用32位整数表示
    pub sort: i32,
    /// 父菜单ID，使用64位无符号整数表示
    pub parent_id: u64,
    /// 菜单路径，使用可选字符串类型存储，可能为空
    pub path: Option<String>,
    /// 菜单图标，使用可选字符串类型存储，可能为空
    pub icon: Option<String>,
    /// 组件路径，使用可选字符串类型存储，可能为空
    pub component: Option<String>,
    /// 组件名称，使用可选字符串类型存储，可能为空
    pub component_name: Option<String>,
    /// 菜单状态，使用32位整数表示
    pub status: i32,
    /// 是否可见，使用32位整数表示
    pub visible: i32,
    /// 是否缓存，使用32位整数表示
    pub keep_alive: i32,
    /// 租户ID，使用32位整数表示
    pub tenant_id: i32,
    /// 审计字段，包含创建时间、更新时间等审计相关信息
    pub audit: AuditFields,
    /// 子菜单列表，使用Menu类型的向量存储
    pub children: Vec<Menu>,
    /// 领域事件列表，使用DomainEvent类型的向量存储，外部不可见
    events: Vec<DomainEvent>,
}

impl Menu {
    /// Create a new menu
    pub fn create(
        id: u64,
        name: String,
        permission: String,
        types: i32,
        sort: i32,
        parent_id: u64,
        creator: Option<String>,
    ) -> Self {
        let mut menu = Self {
            id,
            name,
            permission,
            types,
            sort,
            parent_id,
            path: None,
            icon: None,
            component: None,
            component_name: None,
            status: 0,
            visible: 0,
            keep_alive: 0,
            tenant_id: 0,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Utc::now(),
                updater: creator,
                update_time: Utc::now(),
                deleted: 0,
            },
            children: Vec::new(),
            events: Vec::new(),
        };
        menu.add_event(DomainEvent::MenuCreated { menu_id: id });
        menu
    }

    /// Update menu info
    pub fn update_info(
        &mut self,
        name: String,
        permission: String,
        types: i32,
        sort: i32,
        parent_id: u64,
        path: Option<String>,
        icon: Option<String>,
        component: Option<String>,
        component_name: Option<String>,
        visible: i32,
        keep_alive: i32,
        updater: Option<String>,
    ) {
        self.name = name;
        self.permission = permission;
        self.types = types;
        self.sort = sort;
        self.parent_id = parent_id;
        self.path = path;
        self.icon = icon;
        self.component = component;
        self.component_name = component_name;
        self.visible = visible;
        self.keep_alive = keep_alive;
        self.audit.updater = updater;
        self.audit.update_time = Utc::now();
        self.add_event(DomainEvent::MenuUpdated { menu_id: self.id });
    }

    /// Change status
    pub fn change_status(&mut self, status: i32, updater: Option<String>) {
        self.status = status;
        self.audit.updater = updater;
        self.audit.update_time = Utc::now();
    }

    /// Soft delete
    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = 1;
        self.audit.updater = updater;
        self.audit.update_time = Utc::now();
        self.add_event(DomainEvent::MenuDeleted { menu_id: self.id });
    }

    /// Check if it's a directory
    pub fn is_directory(&self) -> bool {
        self.types == 0
    }

    /// Check if it's a menu (page)
    pub fn is_menu(&self) -> bool {
        self.types == 1
    }

    /// Check if it's a button (permission)
    pub fn is_button(&self) -> bool {
        self.types == 2
    }

    /// Check if it's a root menu
    pub fn is_root(&self) -> bool {
        self.parent_id == 0
    }
}
