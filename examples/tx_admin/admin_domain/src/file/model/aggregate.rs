use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent, Entity};

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
                create_time: Utc::now(),
                updater: creator,
                update_time: Utc::now(),
                deleted: 0,
            },
            events: Vec::new(),
        };
        file.add_event(DomainEvent::FileUploaded { file_id: id });
        file
    }

    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = 1;
        self.audit.updater = updater;
        self.audit.update_time = Utc::now();
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
                create_time: Utc::now(),
                updater: creator,
                update_time: Utc::now(),
                deleted: 0,
            },
            events: Vec::new(),
        }
    }
}
