//! 管理后台错误码定义
//!
//! 使用 tx_error 框架，零堆分配、无虚表。
//! 错误码格式：[ADMIN:xxxx]

use tx_error::CodeMsg;

/// 管理后台业务错误码
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("ADMIN")]
#[ie(tx_di_core::IE)] 
pub enum AdminErr {
    // ── 通用 ─────────────────────────────────────────────
    #[err(1000, "记录不存在")]
    NotFound,
    #[err(1001, "数据已存在")]
    Duplicate,
    #[err(1002, "参数校验失败")]
    Validation,
    #[err(1003, "操作被拒绝")]
    Forbidden,

    // ── 认证 ─────────────────────────────────────────────
    #[err(2000, "用户名或密码错误")]
    BadCredentials,
    #[err(2001, "用户已被禁用")]
    UserDisabled,
    #[err(2002, "租户已被禁用或已过期")]
    TenantDisabled,
    #[err(2003, "Token 无效或已过期")]
    TokenInvalid,

    // ── 用户 ─────────────────────────────────────────────
    #[err(3000, "用户不存在")]
    UserNotFound,
    #[err(3001, "用户名已存在")]
    UsernameDuplicate,
    #[err(3002, "密码校验失败")]
    PasswordVerifyFailed,

    // ── 角色 ─────────────────────────────────────────────
    #[err(4000, "角色不存在")]
    RoleNotFound,
    #[err(4001, "角色编码已存在")]
    RoleCodeDuplicate,
    #[err(4002, "内置角色不可删除")]
    RoleBuiltIn,

    // ── 租户 ─────────────────────────────────────────────
    #[err(5000, "租户不存在")]
    TenantNotFound,
    #[err(5001, "租户编码已存在")]
    TenantCodeDuplicate,

    // ── 部门 ─────────────────────────────────────────────
    #[err(6000, "部门不存在")]
    DeptNotFound,
    #[err(6001, "存在子部门，不可删除")]
    DeptHasChildren,

    // ── 菜单 / 权限 ─────────────────────────────────────
    #[err(7000, "菜单不存在")]
    MenuNotFound,
    #[err(7001, "权限不存在")]
    PermissionNotFound,

    // ── 文件 ─────────────────────────────────────────────
    #[err(8000, "文件上传失败")]
    FileUploadFailed,
    #[err(8001, "文件不存在")]
    FileNotFound,

    // ── 字典 ─────────────────────────────────────────────
    #[err(9000, "字典类型不存在")]
    DictTypeNotFound,
    #[err(9001, "字典数据不存在")]
    DictDataNotFound,

    // ── 岗位 ─────────────────────────────────────────────
    #[err(10000, "岗位不存在")]
    PostNotFound,

    // ── 基础设施 ─────────────────────────────────────────
    #[err(90000, "数据库错误")]
    Database,
    #[err(90001, "外部服务错误")]
    ExternalService,
    #[err(99999, "未知错误")]
    Unknown,
}

/// 带上下文信息的错误包装
///
/// `AppError` 本身是零堆分配的，但业务场景经常需要携带
/// 动态上下文（如 "用户 123 不存在"）。
/// 用这个结构体包装，保持核心错误码的高效性。
#[derive(Debug)]
pub struct AdminError {
    pub err: tx_error::AppError,
    /// 动态上下文信息（如实体 ID、字段值等）
    pub context: Option<String>,
}

impl AdminError {
    /// 从错误码创建（无上下文）
    pub fn from_code(err: AdminErr) -> Self {
        Self {
            err: tx_error::AppError::from_code(err),
            context: None,
        }
    }

    /// 从错误码 + 上下文创建
    pub fn with_context(err: AdminErr, context: impl Into<String>) -> Self {
        Self {
            err: tx_error::AppError::from_code(err),
            context: Some(context.into()),
        }
    }

    /// 获取完整错误消息（含上下文）
    pub fn message(&self) -> String {
        match &self.context {
            Some(ctx) => format!("{}: {}", self.err.message(), ctx),
            None => self.err.message().to_string(),
        }
    }

    /// 获取 HTTP 状态码
    pub fn status_code(&self) -> u16 {
        match self.err.code() {
            // 404
            1000 | 3000 | 4000 | 5000 | 6000 | 7000 | 7001 | 8001 | 9000 | 9001 | 10000 => 404,
            // 409 冲突
            1001 | 3001 | 4001 | 5001 => 409,
            // 400 参数错误
            1002 | 2000 | 3002 | 4002 | 6001 | 8000 => 400,
            // 401 未授权
            2001 | 2002 | 2003 => 401,
            // 403 禁止
            1003 => 403,
            // 500 服务器错误
            90000 | 90001 | 99999 => 500,
            // 默认 500
            _ => 500,
        }
    }
}


// ── 从 AdminErr 错误码转换（支持 `?` 操作符）──────────────
impl From<AdminErr> for AdminError {
    fn from(err: AdminErr) -> Self {
        Self::from_code(err)
    }
}
