//! 管理后台错误码定义
//!
//! 直接使用 tx_error::AppError 作为统一错误类型。
//! 错误码格式：[ADMIN:xxxx]

use tx_error::{AppResult, CodeMsg};

/// 管理后台业务错误码
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("ADMIN")]
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

/// 类型别名
pub type AdminResult<T> = AppResult<T>;
