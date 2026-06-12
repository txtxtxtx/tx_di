use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent, Entity};

/// Operate log aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperateLog {
    pub id: u64,
    pub trace_id: String,
    pub user_id: u64,
    pub user_type: i32,
    pub log_type: String,
    pub sub_type: String,
    pub biz_id: u64,
    pub action: String,
    pub success: i32,
    pub extra: String,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub user_ip: Option<String>,
    pub user_agent: Option<String>,
    pub tenant_id: i32,
    pub audit: AuditFields,
    events: Vec<DomainEvent>,
}

impl Entity for OperateLog {
    type Id = u64;
    fn id(&self) -> Self::Id {
        self.id
    }
}

impl AggregateRoot for OperateLog {
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

impl OperateLog {
    pub fn create(
        id: u64,
        trace_id: String,
        user_id: u64,
        user_type: i32,
        log_type: String,
        sub_type: String,
        biz_id: u64,
        action: String,
        success: i32,
        extra: String,
    ) -> Self {
        let mut log = Self {
            id,
            trace_id,
            user_id,
            user_type,
            log_type,
            sub_type,
            biz_id,
            action,
            success,
            extra,
            request_method: None,
            request_url: None,
            user_ip: None,
            user_agent: None,
            tenant_id: 0,
            audit: AuditFields::default(),
            events: Vec::new(),
        };
        log.add_event(DomainEvent::OperateLogCreated { log_id: id });
        log
    }

    pub fn with_request(
        mut self,
        method: Option<String>,
        url: Option<String>,
        ip: Option<String>,
        agent: Option<String>,
    ) -> Self {
        self.request_method = method;
        self.request_url = url;
        self.user_ip = ip;
        self.user_agent = agent;
        self
    }
}

/// Login log aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginLog {
    pub id: u64,
    pub user_id: u64,
    pub user_type: i32,
    pub username: String,
    pub login_ip: String,
    pub login_location: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub login_type: String,
    pub result: i32,
    pub msg: Option<String>,
    pub login_time: Timestamp,
    pub tenant_id: i32,
    pub audit: AuditFields,
    events: Vec<DomainEvent>,
}

impl Entity for LoginLog {
    type Id = u64;
    fn id(&self) -> Self::Id {
        self.id
    }
}

impl AggregateRoot for LoginLog {
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

impl LoginLog {
    pub fn create(
        id: u64,
        user_id: u64,
        user_type: i32,
        username: String,
        login_ip: String,
        login_type: String,
        result: i32,
    ) -> Self {
        let mut log = Self {
            id,
            user_id,
            user_type,
            username,
            login_ip,
            login_location: None,
            browser: None,
            os: None,
            login_type,
            result,
            msg: None,
            login_time: Timestamp::now(),
            tenant_id: 0,
            audit: AuditFields::default(),
            events: Vec::new(),
        };
        log.add_event(DomainEvent::LoginLogCreated { log_id: id });
        log
    }
}
