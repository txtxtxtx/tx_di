use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::shared::model::event::Event;

/// 日志域领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEvent {
    OperateLogCreated { log_id: u64 },
    LoginLogCreated { log_id: u64 },
}

impl Event for LogEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
