//! 用户聚合根
//!
//! 用户是系统的核心聚合根，与角色、部门、岗位关联。
//! 参考 RuoYi-Vue-Pro 的 system_users 表结构。

use std::fmt::Display;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use toasty::Model;

use super::data_permission::DataScope;

// ─── 枚举定义（toasty::Embed，存储为整数判别值）────────────────

/// 用户状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum UserStatus {
    /// 正常
    #[column(variant = 0)]
    Active,
    /// 禁用
    #[column(variant = 1)]
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

/// 用户性别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum Sex {
    /// 未知
    #[column(variant = 0)]
    Unknown,
    /// 男
    #[column(variant = 1)]
    Male,
    /// 女
    #[column(variant = 2)]
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

// ─── 用户聚合根 ──────────────────────────────────────────────

/// 用户实体（聚合根）
///
/// 职责：
/// - 维护用户基本信息
/// - 管理用户状态（启用/禁用）
/// - 关联角色、部门、岗位
/// - 记录登录信息
#[derive(Debug, Clone, Model)]
#[table = "system_users"]
pub struct User {
    /// 用户 ID
    #[key]
    #[auto]
    pub id: u64,

    /// 所属租户 ID
    pub tenant_id: u64,

    /// 用户名（全局唯一）
    #[unique]
    pub username: String,

    /// 密码哈希（bcrypt）
    #[column("password")]
    pub password_hash: String,

    /// 昵称
    pub nickname: String,

    /// 备注
    pub remark: Option<String>,

    /// 部门 ID
    pub dept_id: Option<u64>,

    /// 岗位 ID 列表（JSON 数组）
    pub post_ids: Vec<u64>,

    /// 邮箱
    pub email: Option<String>,

    /// 手机号
    pub mobile: Option<String>,

    /// 性别
    pub sex: Sex,

    /// 头像 URL
    pub avatar: Option<String>,

    /// 用户状态
    pub status: UserStatus,

    /// 最后登录 IP
    pub login_ip: Option<String>,

    /// 最后登录时间
    pub login_date: Option<jiff::Timestamp>,

    /// 创建者
    pub creator: Option<String>,

    /// 更新者
    pub updater: Option<String>,

    /// 创建时间
    #[auto]
    pub created_at: jiff::Timestamp,

    /// 更新时间
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,

    /// 软删除标记（0=正常, 1=已删除）
    #[default(0u8)]
    pub deleted: u8,
}

// ─── 领域行为 ──────────────────────────────────────────────

impl User {
    /// 创建新用户（领域工厂方法）
    pub fn new(
        tenant_id: u64,
        username: String,
        password_hash: String,
        nickname: String,
    ) -> Self {
        Self {
            id: 0, // #[auto] 会自动填充
            tenant_id,
            username,
            password_hash,
            nickname,
            remark: None,
            dept_id: None,
            post_ids: vec![],
            email: None,
            mobile: None,
            sex: Sex::Unknown,
            avatar: None,
            status: UserStatus::Active,
            login_ip: None,
            login_date: None,
            creator: None,
            updater: None,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }

    /// 检查用户是否可用
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active && self.deleted == 0
    }

    /// 禁用用户
    pub fn disable(&mut self) {
        self.status = UserStatus::Disabled;
    }

    /// 启用用户
    pub fn enable(&mut self) {
        self.status = UserStatus::Active;
    }

    /// 更新密码哈希
    pub fn change_password(&mut self, new_hash: String) {
        self.password_hash = new_hash;
    }

    /// 记录登录信息
    pub fn record_login(&mut self, ip: String) {
        self.login_ip = Some(ip);
        self.login_date = Some(jiff::Timestamp::now());
    }

    /// 更新个人资料
    pub fn update_profile(
        &mut self,
        nickname: Option<String>,
        email: Option<String>,
        mobile: Option<String>,
        avatar: Option<String>,
    ) {
        if let Some(n) = nickname {
            self.nickname = n;
        }
        if let Some(e) = email {
            self.email = Some(e);
        }
        if let Some(m) = mobile {
            self.mobile = Some(m);
        }
        if let Some(a) = avatar {
            self.avatar = Some(a);
        }
    }

    /// 软删除
    pub fn mark_deleted(&mut self) {
        self.deleted = 1;
    }
}

// ─── 仓储 trait ──────────────────────────────────────────────

/// 用户仓储 trait（领域层定义，基础设施层实现）
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// 根据 ID 查找用户
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, anyhow::Error>;

    /// 根据用户名查找用户
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, anyhow::Error>;

    /// 查找租户下所有用户
    async fn find_by_tenant(
        &self,
        tenant_id: u64,
        data_scope: &DataScope,
        current_user_id: u64,
    ) -> Result<Vec<User>, anyhow::Error>;

    /// 分页查询用户列表
    async fn find_page(
        &self,
        tenant_id: u64,
        keyword: Option<&str>,
        status: Option<UserStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<User>, u64), anyhow::Error>;

    /// 保存用户（新增或更新）
    async fn save(&self, user: &User) -> Result<(), anyhow::Error>;

    /// 删除用户（软删除）
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
