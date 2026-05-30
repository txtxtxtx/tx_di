//! 统一领域错误类型
//!
//! 所有业务层错误统一定义在此，application 层和 interfaces 层通过 `?` 传播，
//! 最终在 axum handler 中转为 HTTP 响应。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// 统一 API 错误响应
#[derive(Debug, Serialize)]
struct ErrorBody {
    code: u16,
    message: String,
}

/// 领域错误枚举
///
/// 每个变体对应一类业务错误，携带具体上下文信息。
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    // ── 通用 ─────────────────────────────────────────────
    #[error("记录不存在: {entity} id={id}")]
    NotFound { entity: &'static str, id: String },

    #[error("{field} 已存在: {value}")]
    Duplicate { entity: &'static str, field: &'static str, value: String },

    #[error("参数校验失败: {0}")]
    Validation(String),

    #[error("操作被拒绝: {0}")]
    Forbidden(String),

    // ── 认证 ─────────────────────────────────────────────
    #[error("用户名或密码错误")]
    BadCredentials,

    #[error("用户已被禁用")]
    UserDisabled,

    #[error("租户已被禁用或已过期")]
    TenantDisabled,

    #[error("Token 无效或已过期")]
    TokenInvalid,

    // ── 用户 ─────────────────────────────────────────────
    #[error("用户不存在: id={0}")]
    UserNotFound(String),

    #[error("用户名已存在: {0}")]
    UsernameDuplicate(String),

    #[error("密码校验失败")]
    PasswordVerifyFailed,

    // ── 角色 ─────────────────────────────────────────────
    #[error("角色不存在: id={0}")]
    RoleNotFound(String),

    #[error("角色编码已存在: {0}")]
    RoleCodeDuplicate(String),

    #[error("内置角色不可删除")]
    RoleBuiltIn,

    // ── 租户 ─────────────────────────────────────────────
    #[error("租户不存在: id={0}")]
    TenantNotFound(String),

    #[error("租户编码已存在: {0}")]
    TenantCodeDuplicate(String),

    // ── 部门 ─────────────────────────────────────────────
    #[error("部门不存在: id={0}")]
    DeptNotFound(String),

    #[error("存在子部门，不可删除")]
    DeptHasChildren,

    // ── 菜单 / 权限 ─────────────────────────────────────
    #[error("菜单不存在: id={0}")]
    MenuNotFound(String),

    #[error("权限不存在: id={0}")]
    PermissionNotFound(String),

    // ── 文件 ─────────────────────────────────────────────
    #[error("文件上传失败: {0}")]
    FileUploadFailed(String),

    #[error("文件不存在: {0}")]
    FileNotFound(String),

    // ── 字典 ─────────────────────────────────────────────
    #[error("字典类型不存在: id={0}")]
    DictTypeNotFound(String),

    #[error("字典数据不存在: id={0}")]
    DictDataNotFound(String),

    // ── 岗位 ─────────────────────────────────────────────
    #[error("岗位不存在: id={0}")]
    PostNotFound(String),

    // ── 基础设施 ─────────────────────────────────────────
    #[error("数据库错误: {0}")]
    Database(String),

    #[error("外部服务错误: {0}")]
    ExternalService(String),
}

// ── From 实现（让 `?` 自动转换）──────────────────────────

impl From<anyhow::Error> for AdminError {
    fn from(err: anyhow::Error) -> Self {
        // 尝试向下转型为 AdminError
        if let Some(admin_err) = err.downcast_ref::<AdminError>() {
            // 需要 clone，但 AdminError 包含 String，我们可以重新构造
            // 这里直接转为 Database 错误
            return AdminError::Database(admin_err.to_string());
        }
        AdminError::Database(err.to_string())
    }
}

// ── axum IntoResponse ──────────────────────────────────

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            // 404
            AdminError::NotFound { .. }
            | AdminError::UserNotFound(_)
            | AdminError::RoleNotFound(_)
            | AdminError::TenantNotFound(_)
            | AdminError::DeptNotFound(_)
            | AdminError::MenuNotFound(_)
            | AdminError::PermissionNotFound(_)
            | AdminError::FileNotFound(_)
            | AdminError::DictTypeNotFound(_)
            | AdminError::DictDataNotFound(_)
            | AdminError::PostNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),

            // 409 冲突
            AdminError::Duplicate { .. }
            | AdminError::UsernameDuplicate(_)
            | AdminError::RoleCodeDuplicate(_)
            | AdminError::TenantCodeDuplicate(_) => (StatusCode::CONFLICT, self.to_string()),

            // 400 参数错误
            AdminError::Validation(_)
            | AdminError::BadCredentials
            | AdminError::PasswordVerifyFailed
            | AdminError::FileUploadFailed(_)
            | AdminError::RoleBuiltIn
            | AdminError::DeptHasChildren => (StatusCode::BAD_REQUEST, self.to_string()),

            // 401 未授权
            AdminError::UserDisabled
            | AdminError::TenantDisabled
            | AdminError::TokenInvalid => (StatusCode::UNAUTHORIZED, self.to_string()),

            // 403 禁止
            AdminError::Forbidden(_) => (StatusCode::FORBIDDEN, self.to_string()),

            // 500 服务器内部错误
            AdminError::Database(_)
            | AdminError::ExternalService(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = ErrorBody {
            code: status.as_u16(),
            message,
        };
        (status, Json(body)).into_response()
    }
}
