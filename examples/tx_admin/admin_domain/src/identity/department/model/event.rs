use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::shared::model::event::Event;

/// 部门域领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepartmentEvent {
    DepartmentCreated { dept_id: u64 },
    DepartmentUpdated { dept_id: u64 },
    DepartmentDeleted { dept_id: u64 },
}

impl Event for DepartmentEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
