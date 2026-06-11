use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use crate::shared::model::value_object::TenantId;

/// User query filters
/// 用户查询结构体，用于封装用户查询相关的参数
/// 所有字段都是可选的，允许灵活的查询条件组合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserQuery {
    /// 用户名，可选参数
    pub username: Option<String>,
    /// 昵称，可选参数
    pub nickname: Option<String>,
    /// 手机号码，可选参数
    pub mobile: Option<String>,
    /// 用户状态，可选参数
    pub status: Option<UserStatus>,
    /// 部门ID，可选参数，使用u64类型存储
    pub dept_id: Option<u64>,
    /// 开始时间，可选参数，用于时间范围查询
    pub begin_time: Option<String>,
    /// 结束时间，可选参数，用于时间范围查询
    pub end_time: Option<String>,
}

/// User info for display (without password)
/// 用户展示信息结构体，用于存储和传输用户的基本信息
/// 包含了用户的基本属性和关联信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDisplayInfo {
    /// 用户唯一标识符
    pub id: u64,
    /// 用户登录名
    pub username: String,
    /// 用户昵称/显示名称
    pub nickname: String,
    /// 用户邮箱地址，可选字段
    pub email: Option<String>,
    /// 用户手机号码，可选字段
    pub mobile: Option<String>,
    /// 用户性别
    pub sex: Sex,
    /// 用户头像URL，可选字段
    pub avatar: Option<String>,
    /// 用户状态
    pub status: UserStatus,
    /// 用户所属部门名称列表
    pub dept_names: Vec<String>,
    /// 用户角色名称列表
    pub role_names: Vec<String>,
}

/// User login info
#[derive(Debug, Clone, Serialize, Deserialize)]
/// 登录用户信息结构体，用于存储用户登录后的相关信息
/// 包含用户ID、用户名、昵称、租户ID、角色ID列表、权限列表和部门ID列表
pub struct LoginUser {
    /// 用户ID，使用u64类型存储
    pub user_id: u64,
    /// 用户名，使用String类型存储
    pub username: String,
    /// 用户昵称，使用String类型存储
    pub nickname: String,
    /// 租户ID
    pub tenant_id: TenantId,
    /// 角色ID列表，使用Vec<u64>类型存储
    pub role_ids: Vec<u64>,
    /// 权限列表，使用Vec<String>类型存储
    pub permissions: HashSet<String>,
    /// 部门ID列表，使用Vec<u64>类型存储
    pub dept_ids: Vec<u64>,
}

/// 用户状态
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum UserStatus {
    /// 用户状态：激活
    #[default]
    Active = 0,
    /// 用户状态：禁用
    Disabled = 1,
    /// 用户状态：锁定
    Locked = 2,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Sex {
    /// 性别：未知
    #[default]
    Unknown = 0,
    /// 性别：男
    Male = 1,
    /// 性别：女
    Female = 2,
}

impl From<i32> for Sex {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Male,
            2 => Self::Female,
            _ => Self::Unknown,
        }
    }
}

/// UserStatus 转换错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidUserStatusError(pub i32);

impl std::fmt::Display for InvalidUserStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid user status value: {}", self.0)
    }
}

impl UserStatus {
    /// 从 i32 安全转换，未知值返回错误
    pub fn try_from_i32(value: i32) -> Result<Self, InvalidUserStatusError> {
        match value {
            0 => Ok(Self::Active),
            1 => Ok(Self::Disabled),
            2 => Ok(Self::Locked),
            _ => Err(InvalidUserStatusError(value)),
        }
    }
}

impl From<i32> for UserStatus {
    fn from(value: i32) -> Self {
        Self::try_from_i32(value).unwrap_or(Self::Active)
    }
}