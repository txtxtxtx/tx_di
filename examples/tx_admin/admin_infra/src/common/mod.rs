//! 公共嵌入枚举定义
//!
//! 为多个模型共享的 `sex`、`status`、`deleted` 字段提供统一的枚举类型。

use toasty::Embed;

/// 性别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Embed)]
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

impl Default for Sex {
    fn default() -> Self {
        Sex::Unknown
    }
}

/// 通用启用状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Embed)]
pub enum Status {
    /// 启用
    #[column(variant = 0)]
    Enabled,
    /// 停用
    #[column(variant = 1)]
    Disabled,
    #[column(variant = 2)]
    Locked

}

impl Default for Status {
    fn default() -> Self {
        Status::Enabled
    }
}

/// 软删除标记
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Embed)]
pub enum Deleted {
    /// 未删除
    #[column(variant = 0)]
    No,
    /// 已删除
    #[column(variant = 1)]
    Yes,
}

impl Default for Deleted {
    fn default() -> Self {
        Deleted::No
    }
}

// ── i32 互转 ──

impl From<Sex> for i32 {
    fn from(v: Sex) -> Self {
        match v {
            Sex::Unknown => 0,
            Sex::Male => 1,
            Sex::Female => 2,
        }
    }
}

impl From<i32> for Sex {
    fn from(v: i32) -> Self {
        match v {
            1 => Sex::Male,
            2 => Sex::Female,
            _ => Sex::Unknown,
        }
    }
}

impl From<Status> for i32 {
    fn from(v: Status) -> Self {
        match v {
            Status::Enabled => 0,
            Status::Disabled => 1,
            Status::Locked => 2,
        }
    }
}

impl From<i32> for Status {
    fn from(v: i32) -> Self {
        match v {
            0 => Status::Enabled,
            1 => Status::Disabled,
            2 => Status::Locked,
            _ => Status::Enabled,
        }
    }
}

impl From<Deleted> for i32 {
    fn from(v: Deleted) -> Self {
        match v {
            Deleted::No => 0,
            Deleted::Yes => 1,
        }
    }
}

impl From<i32> for Deleted {
    fn from(v: i32) -> Self {
        match v {
            1 => Deleted::Yes,
            _ => Deleted::No,
        }
    }
}

// ── domain 层枚举互转 ──

impl From<admin_domain::user::model::value_object::Sex> for Sex {
    fn from(v: admin_domain::user::model::value_object::Sex) -> Self {
        match v {
            admin_domain::user::model::value_object::Sex::Unknown => Sex::Unknown,
            admin_domain::user::model::value_object::Sex::Male => Sex::Male,
            admin_domain::user::model::value_object::Sex::Female => Sex::Female,
        }
    }
}

impl From<Sex> for admin_domain::user::model::value_object::Sex {
    fn from(v: Sex) -> Self {
        match v {
            Sex::Unknown => admin_domain::user::model::value_object::Sex::Unknown,
            Sex::Male => admin_domain::user::model::value_object::Sex::Male,
            Sex::Female => admin_domain::user::model::value_object::Sex::Female,
        }
    }
}

impl From<admin_domain::user::model::value_object::UserStatus> for Status {
    fn from(v: admin_domain::user::model::value_object::UserStatus) -> Self {
        match v {
            admin_domain::user::model::value_object::UserStatus::Active => Status::Enabled,
            admin_domain::user::model::value_object::UserStatus::Disabled => Status::Disabled,
            admin_domain::user::model::value_object::UserStatus::Locked => Status::Locked,
        }
    }
}

impl From<Status> for admin_domain::user::model::value_object::UserStatus {
    fn from(v: Status) -> Self {
        match v {
            Status::Enabled => admin_domain::user::model::value_object::UserStatus::Active,
            Status::Disabled => admin_domain::user::model::value_object::UserStatus::Disabled,
            Status::Locked => admin_domain::user::model::value_object::UserStatus::Locked,
        }
    }
}

impl From<admin_domain::shared::model::value_object::DeletedStatus> for Deleted {
    fn from(v: admin_domain::shared::model::value_object::DeletedStatus) -> Self {
        match v {
            admin_domain::shared::model::value_object::DeletedStatus::Normal => Deleted::No,
            admin_domain::shared::model::value_object::DeletedStatus::Deleted => Deleted::Yes,
        }
    }
}
