use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent, Entity};

/// System config aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub id: u64,
    pub category: String,
    pub config_type: i32,
    pub name: String,
    pub config_key: String,
    pub value: String,
    pub visible: i32,
    pub remark: Option<String>,
    pub audit: AuditFields,
    events: Vec<DomainEvent>,
}

impl Entity for Config {
    type Id = u64;
    fn id(&self) -> Self::Id {
        self.id
    }
}

impl AggregateRoot for Config {
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

impl Config {
    pub fn create(
        id: u64,
        category: String,
        config_type: i32,
        name: String,
        config_key: String,
        value: String,
        creator: Option<String>,
    ) -> Self {
        let mut config = Self {
            id,
            category,
            config_type,
            name,
            config_key,
            value,
            visible: 1,
            remark: None,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Utc::now(),
                updater: creator,
                update_time: Utc::now(),
                deleted: 0,
            },
            events: Vec::new(),
        };
        config.add_event(DomainEvent::ConfigCreated { config_id: id });
        config
    }

    pub fn update_info(
        &mut self,
        category: String,
        config_type: i32,
        name: String,
        config_key: String,
        value: String,
        visible: i32,
        remark: Option<String>,
        updater: Option<String>,
    ) {
        self.category = category;
        self.config_type = config_type;
        self.name = name;
        self.config_key = config_key;
        self.value = value;
        self.visible = visible;
        self.remark = remark;
        self.audit.updater = updater;
        self.audit.update_time = Utc::now();
        self.add_event(DomainEvent::ConfigUpdated { config_id: self.id });
    }

    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = 1;
        self.audit.updater = updater;
        self.audit.update_time = Utc::now();
        self.add_event(DomainEvent::ConfigDeleted { config_id: self.id });
    }
}
