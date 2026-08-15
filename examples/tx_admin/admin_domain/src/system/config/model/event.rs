use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::shared::model::event::Event;

/// 配置域领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigEvent {
    ConfigCreated { config_id: u64 },
    ConfigUpdated { config_id: u64 },
    ConfigDeleted { config_id: u64 },
}

impl Event for ConfigEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
