use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent, Entity};
use crate::shared::model::value_object::DeletedStatus;

/// Dictionary type aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictType {
    pub id: u64,
    pub name: String,
    pub dict_type: String,
    pub status: i32,
    pub remark: Option<String>,
    pub audit: AuditFields,
    events: Vec<DomainEvent>,
}

impl Entity for DictType {
    type Id = u64;
    fn id(&self) -> Self::Id {
        self.id
    }
}

impl AggregateRoot for DictType {
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

impl DictType {
    pub fn create(
        id: u64,
        name: String,
        dict_type: String,
        creator: Option<String>,
    ) -> Self {
        let mut dt = Self {
            id,
            name,
            dict_type,
            status: 0,
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
        dt.add_event(DomainEvent::DictTypeCreated { dict_type_id: id });
        dt
    }

    pub fn update_info(&mut self, name: String, dict_type: String, remark: Option<String>, updater: Option<String>) {
        self.name = name;
        self.dict_type = dict_type;
        self.remark = remark;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::DictTypeUpdated { dict_type_id: self.id });
    }

    pub fn change_status(&mut self, status: i32, updater: Option<String>) {
        self.status = status;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
    }

    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = DeletedStatus::Deleted;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::DictTypeDeleted { dict_type_id: self.id });
    }
}

/// Dictionary data aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictData {
    pub id: u64,
    pub sort: i32,
    pub label: String,
    pub value: String,
    pub dict_type: String,
    pub status: i32,
    pub color_type: Option<String>,
    pub css_class: Option<String>,
    pub remark: Option<String>,
    pub audit: AuditFields,
    events: Vec<DomainEvent>,
}

impl Entity for DictData {
    type Id = u64;
    fn id(&self) -> Self::Id {
        self.id
    }
}

impl AggregateRoot for DictData {
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

impl DictData {
    pub fn create(
        id: u64,
        sort: i32,
        label: String,
        value: String,
        dict_type: String,
        creator: Option<String>,
    ) -> Self {
        let mut dd = Self {
            id,
            sort,
            label,
            value,
            dict_type,
            status: 0,
            color_type: None,
            css_class: None,
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
        dd.add_event(DomainEvent::DictDataCreated { dict_data_id: id });
        dd
    }

    pub fn update_info(
        &mut self,
        sort: i32,
        label: String,
        value: String,
        dict_type: String,
        color_type: Option<String>,
        css_class: Option<String>,
        remark: Option<String>,
        updater: Option<String>,
    ) {
        self.sort = sort;
        self.label = label;
        self.value = value;
        self.dict_type = dict_type;
        self.color_type = color_type;
        self.css_class = css_class;
        self.remark = remark;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::DictDataUpdated { dict_data_id: self.id });
    }

    pub fn change_status(&mut self, status: i32, updater: Option<String>) {
        self.status = status;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
    }

    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = DeletedStatus::Deleted;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::DictDataDeleted { dict_data_id: self.id });
    }
}
