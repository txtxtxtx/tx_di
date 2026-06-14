use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent, Entity};
use crate::shared::model::value_object::DeletedStatus;

/// File aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: u64,
    pub config_id: Option<i32>,
    pub name: String,
    pub path: String,
    pub url: String,
    pub file_type: Option<String>,
    pub size: i32,
    pub audit: AuditFields,
    events: Vec<DomainEvent>,
}

impl Entity for File {
    type Id = u64;
    fn id(&self) -> Self::Id {
        self.id
    }
}

impl AggregateRoot for File {
    fn events(&self) -> &[DomainEvent] {
        &self.events
    }
    fn clear_events(&mut self) {
        self.events.clear();
    }
    fn add_event(&mut self, event: DomainEvent) {
        self.events.push(event);
    }
}

impl File {
    /// 从持久化层恢复文件（不触发领域事件）
    pub fn restore(
        id: u64,
        config_id: Option<i32>,
        name: String,
        path: String,
        url: String,
        file_type: Option<String>,
        size: i32,
        audit: AuditFields,
    ) -> Self {
        Self {
            id,
            config_id,
            name,
            path,
            url,
            file_type,
            size,
            audit,
            events: Vec::new(),
        }
    }

    pub fn create(
        id: u64,
        config_id: Option<i32>,
        name: String,
        path: String,
        url: String,
        file_type: Option<String>,
        size: i32,
        creator: Option<String>,
    ) -> Self {
        let mut file = Self {
            id,
            config_id,
            name,
            path,
            url,
            file_type,
            size,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Timestamp::now(),
                updater: creator,
                update_time: Timestamp::now(),
                deleted: DeletedStatus::Normal,
            },
            events: Vec::new(),
        };
        file.add_event(DomainEvent::FileUploaded { file_id: id });
        file
    }

    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = DeletedStatus::Deleted;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::FileDeleted { file_id: self.id });
    }
}

/// File config aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    pub id: i32,
    pub name: String,
    pub storage: i32,
    pub remark: Option<String>,
    pub master: i32,
    pub config: String,
    pub audit: AuditFields,
    events: Vec<DomainEvent>,
}

impl Entity for FileConfig {
    type Id = i32;
    fn id(&self) -> Self::Id {
        self.id
    }
}

impl AggregateRoot for FileConfig {
    fn events(&self) -> &[DomainEvent] {
        &self.events
    }
    fn clear_events(&mut self) {
        self.events.clear();
    }
    fn add_event(&mut self, event: DomainEvent) {
        self.events.push(event);
    }
}

impl FileConfig {
    /// 从持久化层恢复文件配置（不触发领域事件）
    pub fn restore(
        id: i32,
        name: String,
        storage: i32,
        remark: Option<String>,
        master: i32,
        config: String,
        audit: AuditFields,
    ) -> Self {
        Self {
            id,
            name,
            storage,
            remark,
            master,
            config,
            audit,
            events: Vec::new(),
        }
    }

    pub fn create(
        id: i32,
        name: String,
        storage: i32,
        config: String,
        creator: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            storage,
            remark: None,
            master: 0,
            config,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Timestamp::now(),
                updater: creator,
                update_time: Timestamp::now(),
                deleted: DeletedStatus::Normal,
            },
            events: Vec::new(),
        }
    }
}
