use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent};
use crate::shared::model::value_object::DeletedStatus;
use crate::AggregateRoot;

/// Department aggregate root
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
pub struct Department {
    pub id: u64,
    pub name: String,
    pub parent_id: u64,
    pub sort: i32,
    pub leader_user_id: Option<u64>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: i32,
    pub tenant_id: i32,
    pub audit: AuditFields,
    pub children: Vec<Department>,
    events: Vec<DomainEvent>,
}

impl Department {
    pub fn create(
        id: u64,
        name: String,
        parent_id: u64,
        sort: i32,
        creator: Option<String>,
    ) -> Self {
        let mut dept = Self {
            id,
            name,
            parent_id,
            sort,
            leader_user_id: None,
            phone: None,
            email: None,
            status: 0,
            tenant_id: 0,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Timestamp::now(),
                updater: creator,
                update_time: Timestamp::now(),
                deleted: DeletedStatus::Normal,
            },
            children: Vec::new(),
            events: Vec::new(),
        };
        dept.add_event(DomainEvent::DepartmentCreated { dept_id: id });
        dept
    }

    pub fn update_info(
        &mut self,
        name: String,
        parent_id: u64,
        sort: i32,
        leader_user_id: Option<u64>,
        phone: Option<String>,
        email: Option<String>,
        updater: Option<String>,
    ) {
        self.name = name;
        self.parent_id = parent_id;
        self.sort = sort;
        self.leader_user_id = leader_user_id;
        self.phone = phone;
        self.email = email;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::DepartmentUpdated { dept_id: self.id });
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
        self.add_event(DomainEvent::DepartmentDeleted { dept_id: self.id });
    }

    pub fn is_root(&self) -> bool {
        self.parent_id == 0
    }
}
