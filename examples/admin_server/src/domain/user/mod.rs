//! 用户聚合根

use std::fmt::Display;
use async_trait::async_trait;
use crate::domain::data_permission::DataScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus { Active, Disabled }
impl Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { UserStatus::Active => write!(f, "active"), UserStatus::Disabled => write!(f, "disabled") }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sex { Unknown, Male, Female }
impl Display for Sex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Sex::Unknown => write!(f, "unknown"), Sex::Male => write!(f, "male"), Sex::Female => write!(f, "female") }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: u64, pub tenant_id: u64, pub username: String, pub password_hash: String,
    pub nickname: String, pub remark: Option<String>, pub dept_id: Option<u64>,
    pub post_ids: Vec<u64>, pub email: Option<String>, pub mobile: Option<String>,
    pub sex: Sex, pub avatar: Option<String>, pub status: UserStatus,
    pub login_ip: Option<String>, pub login_date: Option<jiff::Timestamp>,
    pub creator: Option<String>, pub updater: Option<String>,
    pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8,
}

impl User {
    pub fn new(tenant_id: u64, username: String, password_hash: String, nickname: String) -> Self {
        Self {
            id: 0, tenant_id, username, password_hash, nickname,
            remark: None, dept_id: None, post_ids: vec![],
            email: None, mobile: None, sex: Sex::Unknown, avatar: None,
            status: UserStatus::Active, login_ip: None, login_date: None,
            creator: None, updater: None,
            created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0,
        }
    }
    pub fn is_active(&self) -> bool { self.status == UserStatus::Active && self.deleted == 0 }
    pub fn disable(&mut self) { self.status = UserStatus::Disabled; }
    pub fn enable(&mut self) { self.status = UserStatus::Active; }
    pub fn change_password(&mut self, new_hash: String) { self.password_hash = new_hash; }
    pub fn record_login(&mut self, ip: String) { self.login_ip = Some(ip); self.login_date = Some(jiff::Timestamp::now()); }
    pub fn update_profile(&mut self, nickname: Option<String>, email: Option<String>, mobile: Option<String>, avatar: Option<String>) {
        if let Some(n) = nickname { self.nickname = n; }
        if let Some(e) = email { self.email = Some(e); }
        if let Some(m) = mobile { self.mobile = Some(m); }
        if let Some(a) = avatar { self.avatar = Some(a); }
    }
    pub fn mark_deleted(&mut self) { self.deleted = 1; }
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, anyhow::Error>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<User>, anyhow::Error>;
    async fn find_page(&self, tenant_id: u64, keyword: Option<&str>, status: Option<UserStatus>, page: u64, page_size: u64) -> Result<(Vec<User>, u64), anyhow::Error>;
    async fn save(&self, user: &User) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
