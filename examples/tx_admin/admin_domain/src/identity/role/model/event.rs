use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::shared::model::event::Event;

/// 角色域领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoleEvent {
    RoleCreated { role_id: u64 },
    RoleUpdated { role_id: u64 },
    RoleDeleted { role_id: u64 },
    RolePermissionsChanged { role_id: u64 },
}

impl Event for RoleEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
