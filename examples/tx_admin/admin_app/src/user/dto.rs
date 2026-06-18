use admin_domain::user::model::aggregate::User;
use admin_domain::user::model::value_object::{Sex, UserStatus};
use serde::{Deserialize, Serialize};

// 统一使用 proto 定义的 UserResponse，无需中间层转换
pub type UserResponse = admin_proto::admin::user::UserResponse;

// ── Command / Query DTOs ──────────────────────────────────────────────

/// 创建用户命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserCommand {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub sex: Option<Sex>,
    pub remark: Option<String>,
    pub role_ids: Option<Vec<u64>>,
    pub dept_ids: Option<Vec<u64>>,
}

/// 更新用户命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserCommand {
    pub user_id: u64,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub sex: Option<Sex>,
    pub status: Option<UserStatus>,
    pub remark: Option<String>,
}

/// 修改密码命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordCommand {
    pub user_id: u64,
    pub new_password: String,
}

/// 分配角色命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignRolesCommand {
    pub user_id: u64,
    pub role_ids: Vec<u64>,
}

/// 分配部门命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignDeptsCommand {
    pub user_id: u64,
    pub dept_ids: Vec<u64>,
}

/// 用户分页查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQueryRequest {
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub status: Option<UserStatus>,
    pub dept_id: Option<u64>,
    pub page: i64,
    pub size: i64,
}

// ── Conversion ────────────────────────────────────────────────────────

/// 将领域层的 User 聚合根转换为 proto 的 UserResponse
pub fn user_to_response(user: User) -> UserResponse {
    UserResponse {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        email: user.email,
        mobile: user.mobile,
        sex: user.sex as i32,
        status: user.status as i32,
        remark: user.remark,
        role_ids: user.role_ids,
        dept_ids: user.dept_ids,
        avatar: user.avatar,
        login_ip: user.login_ip,
        login_date: user.login_date.map(|d| d.as_millisecond()).unwrap_or(0),
        tenant_id: user.tenant_id.into_inner(),
        create_time: user.audit.create_time.as_millisecond(),
        update_time: user.audit.update_time.as_millisecond(),
    }
}
