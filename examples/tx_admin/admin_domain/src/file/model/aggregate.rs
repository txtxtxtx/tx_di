use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent, Entity};
use crate::shared::model::value_object::DeletedStatus;

/// File aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: u64,
    pub config_id: Option<u64>,
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
        config_id: Option<u64>,
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
        config_id: Option<u64>,
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
    pub id: u64,
    pub name: String,
    pub storage: i32,
    pub remark: Option<String>,
    pub master: i32,
    pub config: String,
    pub audit: AuditFields,
    events: Vec<DomainEvent>,
}

impl Entity for FileConfig {
    type Id = u64;
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
        id: u64,
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
        id: u64,
        name: String,
        storage: i32,
        remark: Option<String>,
        config: String,
        creator: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            storage,
            remark,
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

    /// 更新配置信息
    pub fn update_info(
        &mut self,
        name: String,
        storage: i32,
        remark: Option<String>,
        config: String,
        updater: Option<String>,
    ) {
        self.name = name;
        self.storage = storage;
        self.remark = remark;
        self.config = config;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
    }

    /// 设为主配置
    pub fn set_master(&mut self, updater: Option<String>) {
        self.master = 1;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
    }

    /// 取消主配置
    pub fn unset_master(&mut self, updater: Option<String>) {
        self.master = 0;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
    }

    /// 软删除
    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = DeletedStatus::Deleted;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
    }
}
