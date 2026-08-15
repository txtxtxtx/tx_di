use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::shared::model::event::Event;

/// 菜单域领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MenuEvent {
    MenuCreated { menu_id: u64 },
    MenuUpdated { menu_id: u64 },
    MenuDeleted { menu_id: u64 },
}

impl Event for MenuEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
