use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::shared::model::event::Event;

/// 字典域领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DictionaryEvent {
    DictTypeCreated { dict_type_id: u64 },
    DictTypeUpdated { dict_type_id: u64 },
    DictTypeDeleted { dict_type_id: u64 },
    DictDataCreated { dict_data_id: u64 },
    DictDataUpdated { dict_data_id: u64 },
    DictDataDeleted { dict_data_id: u64 },
}

impl Event for DictionaryEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
