//! 用户聚合根

// use crate::domain::data_permission::DataScope;
use async_trait::async_trait;
use std::{fmt::Display, net::{IpAddr, Ipv4Addr}};

use crate::domain::DeletedStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum UserStatus {
    #[column(variant = 0_u8)]
    Active,
    #[column(variant = 1_u8)]
    Disabled,
}
impl Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserStatus::Active => write!(f, "active"),
            UserStatus::Disabled => write!(f, "disabled"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum Sex {
    #[column(variant = 0_u8)]
    Unknown,
    #[column(variant = 1_u8)]
    Male,
    #[column(variant = 2_u8)]
    Female,
}

impl Display for Sex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sex::Unknown => write!(f, "unknown"),
            Sex::Male => write!(f, "male"),
            Sex::Female => write!(f, "female"),
        }
    }
}

/**
 * 用户模型结构体
 *
 * 该结构体用于表示系统中的用户实体，包含了用户的基本信息、组织架构关联、
 * 认证信息以及系统的审计字段。派生了 Debug 和 Clone trait 以便调试和复制。
 */
#[derive(Debug, Clone, toasty::Model)]
#[table = "tx_users"]
pub struct User {
    #[key]
    #[auto]
    /// 用户唯一标识符
    pub id: u64,
    /// 所属租户的唯一标识符（用于多租户系统隔离）
    #[index]
    pub tenant_id: u64,
    /// 用户名（通常用于登录，具有唯一性约束）
    #[unique]
    #[column(type = varchar(50))]
    pub username: String,
    /// 用户密码的哈希值（切勿明文存储密码）
    #[column(type = varchar(100))]
    pub password_hash: String,
    /// 用户昵称（用于前端展示）
    #[column(type = varchar(50))]
    pub nickname: String,
    /// 备注信息（可选）
    #[default("".to_string())]
    #[column(type = varchar(500))]
    pub remark: String,
    /// 用户所属的部门ID列表（支持一人多部门）
    #[default(Vec::new())]
    pub dept_id: Vec<u64>,
    /// 用户拥有的岗位ID列表（支持一人多岗位）
    #[default(Vec::new())]
    pub post_ids: Vec<u64>,
    /// 邮箱地址（可选，可用于通知或找回密码）
    #[default("".to_string())]
    pub email: String,
    /// 手机号码（可选，可用于登录或双因素认证）
    #[default("".to_string())]
    pub mobile: String,
    /// 性别枚举
    #[default(Sex::Unknown)]
    pub sex: Sex,
    /// 用户头像的URL地址或文件路径（可选）
    #[column(type = varchar(1000))]
    #[default("".to_string())]
    pub avatar: String,
    /// 用户状态枚举（如：正常、禁用等）
    #[default(UserStatus::Active)]
    pub status: UserStatus,
    /// 最近一次登录的IP地址（可选）
    #[serialize(json)]
    pub login_ip: IpAddr,
    /// 最近一次登录的时间戳（可选）
    pub login_date: Option<jiff::Timestamp>,
    /// 创建人标识（可选，记录是谁创建了该用户）
    #[column(type = varchar(64))]
    pub creator: Option<String>,
    /// 更新人标识（可选，记录最近一次修改该用户信息的人）
    #[column(type = varchar(64))]
    pub updater: Option<String>,
    /// 记录创建时间的时间戳
    #[default(jiff::Timestamp::now())]
    pub created_at: jiff::Timestamp,
    /// 记录最近一次更新时间的时间戳
    #[update(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    /// 逻辑删除状态枚举（如：未删除、已删除）
    #[default(DeletedStatus::Normal)]
    pub deleted: DeletedStatus,
}

impl User {
    pub fn new(tenant_id: u64, username: String, password_hash: String, nickname: String) -> Self {
        Self {
            id: 0,
            tenant_id,
            username,
            password_hash,
            nickname,
            remark: String::new(),
            dept_id: vec![],
            post_ids: vec![],
            email: String::new(),
            mobile: String::new(),
            sex: Sex::Unknown,
            avatar: String::new(),
            status: UserStatus::Active,
            login_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            login_date: None,
            creator: None,
            updater: None,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
            deleted: DeletedStatus::Normal,
        }
    }
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active && self.deleted == DeletedStatus::Normal
    }
    pub fn disable(&mut self) {
        self.status = UserStatus::Disabled;
    }
    pub fn enable(&mut self) {
        self.status = UserStatus::Active;
    }
    pub fn change_password(&mut self, new_hash: String) {
        self.password_hash = new_hash;
    }
    pub fn record_login(&mut self, ip: IpAddr) {
        self.login_ip = ip;
        self.login_date = Some(jiff::Timestamp::now());
    }
    pub fn update_profile(
        &mut self,
        nickname: Option<String>,
        email: Option<String>,
        mobile: Option<String>,
        avatar: Option<String>,
    ) {
        if let Some(n) = nickname { self.nickname = n; }
        if let Some(e) = email { self.email = e; }
        if let Some(m) = mobile { self.mobile = m; }
        if let Some(a) = avatar { self.avatar = a; }
    }
    pub fn mark_deleted(&mut self) {
        self.deleted = DeletedStatus::Deleted;
    }
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, anyhow::Error>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<User>, anyhow::Error>;
    async fn find_page(
        &self,
        tenant_id: u64,
        keyword: Option<&str>,
        status: Option<UserStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<User>, u64), anyhow::Error>;
    async fn save(&self, user: &User) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
