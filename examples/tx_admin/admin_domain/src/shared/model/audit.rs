//! 审计字段

use crate::shared::model::value_object::DeletedStatus;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

/// 所有实体共享的审计字段
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

    pub fn delete(&mut self, updater: Option<String>) {
        // 将 deleted 字段设置为 DeletedStatus::Deleted，表示对象已被删除
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
