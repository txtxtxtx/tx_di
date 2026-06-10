use admin_macros::AggregateRoot;
use chrono::Utc; // 引入时间处理库
use serde::{Deserialize, Serialize}; // 引入序列化和反序列化库

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent}; // 引入共享模块中的模型定义

/// Role aggregate root
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
/// 角色实体结构体，用于存储角色相关的信息
/// 包含角色的基本属性、权限范围、审计信息等
pub struct Role {
    /// 角色ID，使用u64类型保证唯一性
    pub id: u64,
    /// 角色名称，使用String类型存储
    pub name: String,
    /// 角色代码，使用String类型存储，通常用于系统标识
    pub code: String,
    /// 角色排序，使用i32类型，用于控制显示顺序
    pub sort: i32,
    /// 数据范围，使用i32类型，控制角色的数据访问权限
    pub data_scope: i32,
    /// 数据范围部门ID列表，使用Option<String>，可能为空
    pub data_scope_dept_ids: Option<String>,
    /// 状态，使用i32类型，表示角色是否启用/禁用等
    pub status: i32,
    /// 备注，使用Option<String>，可能为空
    pub remark: Option<String>,
    /// 租户ID，使用i32类型，用于多租户系统
    pub tenant_id: i32,
    /// 审计字段，包含创建时间、更新时间等信息
    pub audit: AuditFields,
    /// 菜单ID列表，使用Vec<u64>存储角色关联的菜单权限
    pub menu_ids: Vec<u64>,
    // 领域事件列表，使用Vec<DomainEvent>存储，用于领域事件处理，不对外暴露
    events: Vec<DomainEvent>,
}

impl Role {
    /// Create a new role
    pub fn create(
        id: u64,
        name: String,
        code: String,
        sort: i32,
        creator: Option<String>,
    ) -> Self {
        let mut role = Self {
            id,
            name,
            code,
            sort,
            data_scope: 4,
            data_scope_dept_ids: None,
            status: 0,
            remark: None,
            tenant_id: 0,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Utc::now(),
                updater: creator,
                update_time: Utc::now(),
                deleted: 0,
            },
            menu_ids: Vec::new(),
            events: Vec::new(),
        };
        role.add_event(DomainEvent::RoleCreated { role_id: id });
        role
    }

    /// Update basic info
    pub fn update_info(
        &mut self,
        name: String,
        code: String,
        sort: i32,
        data_scope: i32,
        remark: Option<String>,
        updater: Option<String>,
    ) {
        self.name = name;
        self.code = code;
        self.sort = sort;
        self.data_scope = data_scope;
        self.remark = remark;
        self.audit.updater = updater;
        self.audit.update_time = Utc::now();
        self.add_event(DomainEvent::RoleUpdated { role_id: self.id });
    }

    /// Change status
    pub fn change_status(&mut self, status: i32, updater: Option<String>) {
        self.status = status;
        self.audit.updater = updater;
        self.audit.update_time = Utc::now();
    }

    /// Set menu permissions
    pub fn set_menus(&mut self, menu_ids: Vec<u64>) {
        self.menu_ids = menu_ids;
        self.audit.update_time = Utc::now();
        self.add_event(DomainEvent::RolePermissionsChanged { role_id: self.id });
    }

    /// Soft delete
    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = 1;
        self.audit.updater = updater;
        self.audit.update_time = Utc::now();
        self.add_event(DomainEvent::RoleDeleted { role_id: self.id });
    }

    /// Check if role is active
    pub fn is_active(&self) -> bool {
        self.status == 0 && self.audit.deleted == 0
    }
}
