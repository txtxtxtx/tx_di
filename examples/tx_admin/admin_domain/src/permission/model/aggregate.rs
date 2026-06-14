use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent};
use crate::shared::model::value_object::DeletedStatus;
use crate::permission::model::value_object::PermissionType;
use crate::AggregateRoot;

/// Permission aggregate root
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
pub struct Permission {
    pub id: u64,
    pub name: String,
    pub permission_code: String,
    pub permission_type: PermissionType,
    pub parent_id: u64,
    pub sort: i32,
    pub description: Option<String>,
    pub status: i32,
    pub audit: AuditFields,
    events: Vec<DomainEvent>,
}

impl Permission {
    /// 从持久化层恢复权限（不触发领域事件）
    pub fn restore(
        id: u64,
        name: String,
        permission_code: String,
        permission_type: PermissionType,
        parent_id: u64,
        sort: i32,
        description: Option<String>,
        status: i32,
        audit: AuditFields,
    ) -> Self {
        Self {
            id,
            name,
            permission_code,
            permission_type,
            parent_id,
            sort,
            description,
            status,
            audit,
            events: Vec::new(),
        }
    }

    /// Create a new permission
    pub fn create(
        id: u64,
        name: String,
        permission_code: String,
        permission_type: PermissionType,
        parent_id: u64,
        sort: i32,
        description: Option<String>,
        creator: Option<String>,
    ) -> Self {
        let mut perm = Self {
            id,
            name,
            permission_code,
            permission_type,
            parent_id,
            sort,
            description,
            status: 0,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Timestamp::now(),
                updater: creator,
                update_time: Timestamp::now(),
                deleted: DeletedStatus::Normal,
            },
            events: Vec::new(),
        };
        perm.add_event(DomainEvent::PermissionCreated { permission_id: id });
        perm
    }

    /// Update permission info
    pub fn update_info(
        &mut self,
        name: String,
        permission_code: String,
        permission_type: PermissionType,
        parent_id: u64,
        sort: i32,
        description: Option<String>,
        updater: Option<String>,
    ) {
        self.name = name;
        self.permission_code = permission_code;
        self.permission_type = permission_type;
        self.parent_id = parent_id;
        self.sort = sort;
        self.description = description;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::PermissionUpdated { permission_id: self.id });
    }

    /// Soft delete
    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = DeletedStatus::Deleted;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::PermissionDeleted { permission_id: self.id });
    }

    /// Check if permission is active
    pub fn is_active(&self) -> bool {
        self.status == 0 && self.audit.deleted == DeletedStatus::Normal
    }
}
