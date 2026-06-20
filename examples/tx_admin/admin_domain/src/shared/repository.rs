use tx_error::{ CodeMsg};

/// 重导出 `tx_error::log_err`，保持向后兼容
///
/// 日志格式: `[DOMAIN:CODE] MESSAGE: 原始错误信息`
///
/// # 用法
/// ```ignore
/// .map_err(|e| db_err(e, RepositoryError::DatabaseUser))?
/// ```
pub use tx_error::log_err as db_err;

/// Repository 层错误类型
///
/// 每个错误有唯一编码，方便前端根据编码做国际化
/// 编码规则: 1xxxx = 数据库, 2xxxx = 不存在, 3xxxx = 重复, 4xxxx = 校验
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("REPOSITORY")]
pub enum RepositoryError {
    // ── 数据库异常 (10001-10009) ──
    #[err(10001, "数据库异常")]
    DatabaseUser,
    #[err(10002, "数据库异常")]
    DatabaseRole,
    #[err(10003, "数据库异常")]
    DatabaseDept,
    #[err(10004, "数据库异常")]
    DatabaseMenu,
    #[err(10006, "数据库异常")]
    DatabaseConfig,
    #[err(10007, "数据库异常")]
    DatabaseDict,
    #[err(10008, "数据库异常")]
    DatabaseFile,
    #[err(10009, "数据库异常")]
    DatabaseLog,
    #[err(10010, "数据库异常")]
    DatabaseJob,
    #[err(10011, "数据库异常")]
    DatabaseJobLog,

    // ── 记录不存在 (10101-10109) ──
    #[err(10101, "记录不存在")]
    NotFoundUser,
    #[err(10102, "记录不存在")]
    NotFoundRole,
    #[err(10103, "记录不存在")]
    NotFoundDept,
    #[err(10104, "记录不存在")]
    NotFoundMenu,
    #[err(10106, "记录不存在")]
    NotFoundConfig,
    #[err(10107, "记录不存在")]
    NotFoundDict,
    #[err(10108, "记录不存在")]
    NotFoundFile,
    #[err(10109, "记录不存在")]
    NotFoundLog,
    #[err(10110, "记录不存在")]
    NotFoundJob,
    #[err(10111, "记录不存在")]
    NotFoundJobLog,

    // ── 重复 (10201-10207) ──
    #[err(10201, "用户名已存在")]
    DuplicateUsername,
    #[err(10202, "角色编码已存在")]
    DuplicateRoleCode,
    #[err(10204, "配置键已存在")]
    DuplicateConfigKey,
    #[err(10205, "字典类型已存在")]
    DuplicateDictType,
    #[err(10206, "邮箱已存在")]
    DuplicateEmail,
    #[err(10207, "手机号已存在")]
    DuplicateMobile,

    // ── 校验 (10301-10312) ──
    #[err(10301, "用户状态异常，无法操作")]
    ValidationUserStatus,
    #[err(10302, "角色已禁用，无法分配")]
    ValidationRoleDisabled,
    #[err(10303, "部门已禁用，无法分配")]
    ValidationDeptDisabled,
    #[err(10304, "密码验证失败")]
    ValidationPassword,
    #[err(10305, "用户名或密码错误")]
    ValidationLogin,
    #[err(10306, "登录状态已过期")]
    ValidationToken,
    #[err(10307, "菜单下存在子菜单，无法删除")]
    ValidationMenuHasChildren,
    #[err(10308, "菜单不能将自身设为上级菜单")]
    ValidationMenuSelfParent,
    #[err(10309, "部门下存在子部门，无法删除")]
    ValidationDeptHasChildren,
    #[err(10310, "部门下存在用户，无法删除")]
    ValidationDeptHasUsers,
    #[err(10311, "部门不能将自身设为上级部门")]
    ValidationDeptSelfParent,
}
