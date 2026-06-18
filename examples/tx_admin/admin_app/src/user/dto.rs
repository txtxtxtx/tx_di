use admin_domain::user::model::aggregate::User;

// 统一使用 proto 定义的 UserResponse，无需中间层转换
pub type UserResponse = admin_proto::admin::user::UserResponse;

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
