use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::job::model::value_object::JobStatus;
use crate::shared::model::event::Event;

/// 任务域领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobEvent {
    JobCreated { job_id: u64 },
    JobUpdated { job_id: u64 },
    JobDeleted { job_id: u64 },
    JobStatusChanged { job_id: u64, status: JobStatus },
    JobLogCreated { log_id: u64, job_id: u64 },
    JobLogFinished { log_id: u64, job_id: u64 },
}

impl Event for JobEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
