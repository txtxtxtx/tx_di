//! 用户模型 — 系统用户和权限管理

use toasty::Model;

/// 系统用户
#[derive(Debug, Clone, Model)]
pub struct User {
    /// 主键 ID（自增）
    #[key]
    #[auto]
    pub id: i64,

    /// 用户名（唯一）
    #[unique]
    pub username: String,

    /// 密码哈希（bcrypt/argon2）
    pub password_hash: String,

    /// 显示昵称
    #[default("".to_string())]
    pub nickname: String,

    /// 邮箱
    #[default("".to_string())]
    pub email: String,

    /// 手机号
    #[default("".to_string())]
    pub phone: String,

    /// 头像 URL
    #[default("".to_string())]
    pub avatar: String,

    /// 角色列表（JSON 数组存储）
    #[default(Vec::new())]
    #[serialize(json)]
    pub roles: Vec<String>,

    /// 权限列表（JSON 数组存储）
    #[default(Vec::new())]
    #[serialize(json)]
    pub permissions: Vec<String>,

    /// 状态：0-禁用 1-启用
    #[default(1)]
    pub status: i32,

    /// 创建时间
    #[auto]
    pub created_at: jiff::Timestamp,

    /// 更新时间
    #[update(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
}
