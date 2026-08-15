use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::shared::model::value_object::DeletedStatus;
use crate::AggregateRoot;
use crate::shared::model::{AggregateRoot, AuditFields};
use crate::system::config::model::event::ConfigEvent;

/// System config aggregate root
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
#[aggregate_root(event = crate::system::config::model::event::ConfigEvent)]
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
    events: Vec<ConfigEvent>,
}

impl Config {
    /// 从持久化层恢复配置（不触发领域事件）
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: u64,
        category: String,
        config_type: i32,
        name: String,
        config_key: String,
        value: String,
        visible: i32,
        remark: Option<String>,
        audit: AuditFields,
    ) -> Self {
        Self {
            id,
            category,
            config_type,
            name,
            config_key,
            value,
            visible,
            remark,
            audit,
            events: Vec::new(),
        }
    }

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
                create_time: Timestamp::now(),
                updater: creator,
                update_time: Timestamp::now(),
                deleted: DeletedStatus::Normal,
            },
            events: Vec::new(),
        };
        config.add_event(ConfigEvent::ConfigCreated { config_id: id });
        config
    }

    #[allow(clippy::too_many_arguments)]
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
        self.audit.update_time = Timestamp::now();
        self.add_event(ConfigEvent::ConfigUpdated { config_id: self.id });
    }

    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = DeletedStatus::Deleted;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(ConfigEvent::ConfigDeleted { config_id: self.id });
    }
}
