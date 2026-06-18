pub mod value_object;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use crate::shared::model::value_object::DeletedStatus;
use crate::user::model::value_object::UserStatus;

/// Base trait for all entities
pub trait Entity {
    type Id: Copy + Eq + std::hash::Hash;
    fn id(&self) -> Self::Id;
}

/// Base trait for aggregate roots
pub trait AggregateRoot: Entity {
    /// Get pending domain events
    fn events(&self) -> &[DomainEvent];
    /// Clear all pending domain events
    fn clear_events(&mut self);
    /// Add a domain event
    fn add_event(&mut self, event: DomainEvent);
}

/// Domain event - unified event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    // User events
    UserCreated { user_id: u64, username: String },
    UserUpdated { user_id: u64 },
    UserDeleted { user_id: u64 },
    UserStatusChanged { user_id: u64, status: UserStatus },
    UserPasswordChanged { user_id: u64 },
    UserLoggedIn { user_id: u64, ip: String },

    // Role events
    RoleCreated { role_id: u64 },
    RoleUpdated { role_id: u64 },
    RoleDeleted { role_id: u64 },
    RolePermissionsChanged { role_id: u64 },

    // Menu events
    MenuCreated { menu_id: u64 },
    MenuUpdated { menu_id: u64 },
    MenuDeleted { menu_id: u64 },

    // Department events
    DepartmentCreated { dept_id: u64 },
    DepartmentUpdated { dept_id: u64 },
    DepartmentDeleted { dept_id: u64 },

    // File events
    FileUploaded { file_id: u64 },
    FileDeleted { file_id: u64 },

    // Config events
    ConfigCreated { config_id: u64 },
    ConfigUpdated { config_id: u64 },
    ConfigDeleted { config_id: u64 },

    // Dictionary events
    DictTypeCreated { dict_type_id: u64 },
    DictTypeUpdated { dict_type_id: u64 },
    DictTypeDeleted { dict_type_id: u64 },
    DictDataCreated { dict_data_id: u64 },
    DictDataUpdated { dict_data_id: u64 },
    DictDataDeleted { dict_data_id: u64 },

    // Log events
    OperateLogCreated { log_id: u64 },
    LoginLogCreated { log_id: u64 },
}

/// Audit fields that all entities share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFields {
    pub creator: Option<String>,
    pub create_time: Timestamp,
    pub updater: Option<String>,
    pub update_time: Timestamp,
    pub deleted: DeletedStatus,
}

impl AuditFields {
    pub fn is_deleted(&self) -> bool {
        self.deleted == DeletedStatus::Deleted
    }

    pub fn delete(&mut self,updater: Option<String>) {
    // 将deleted字段设置为DeletedStatus::Deleted，表示对象已被删除
        self.deleted = DeletedStatus::Deleted;
        self.updater = updater;
        self.update_time = Timestamp::now();
    }
}
impl Default for AuditFields {
    fn default() -> Self {
        let now = Timestamp::now();
        Self {
            creator: None,
            create_time: now,
            updater: None,
            update_time: now,
            deleted: DeletedStatus::Normal,
        }
    }
}
