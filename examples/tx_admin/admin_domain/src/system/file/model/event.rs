use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::shared::model::event::Event;

/// 文件域领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileEvent {
    FileUploaded { file_id: u64 },
    FileDeleted { file_id: u64 },
}

impl Event for FileEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
