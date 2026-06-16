use tx_error::CodeMsg;

/// Repository 锛氬眰閿欒绫诲瀷
///
/// 姣忎釜閿欒鏈夌嫭涓€缂栫爜 锛屾柟渚垮墠绔牴鎹牴鎹爜鍋氬浗闄呭寲銆€
/// 缁勭粐: 1xxxx = 搴撳簱 锛?xxxx = 涓嶅瓨鍦 锛?xxxx = 閲嶅 锛?xxxx = 鏍  骞
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("REPOSITORY")]
pub enum RepositoryError {
    // ── 搴撳簱閿欒 (10001-10009) ──
    #[err(10001, "数据库异常")]
    DatabaseUser,
    #[err(10002, "数据库异常")]
    DatabaseRole,
    #[err(10003, "数据库异常")]
    DatabaseDept,
    #[err(10004, "数据库异常")]
    DatabaseMenu,
    #[err(10005, "数据库异常")]
    DatabasePerm,
    #[err(10006, "数据库异常")]
    DatabaseConfig,
    #[err(10007, "数据库异常")]
    DatabaseDict,
    #[err(10008, "数据库异常")]
    DatabaseFile,
    #[err(10009, "数据库异常")]
    DatabaseLog,

    // ── 涓嶅瓨鍦 (10101-10109) ──
    #[err(10101, "记录不存在")]
    NotFoundUser,
    #[err(10102, "记录不存在")]
    NotFoundRole,
    #[err(10103, "记录不存在")]
    NotFoundDept,
    #[err(10104, "记录不存在")]
    NotFoundMenu,
    #[err(10105, "记录不存在")]
    NotFoundPerm,
    #[err(10106, "记录不存在")]
    NotFoundConfig,
    #[err(10107, "记录不存在")]
    NotFoundDict,
    #[err(10108, "记录不存在")]
    NotFoundFile,
    #[err(10109, "记录不存在")]
    NotFoundLog,

    // ── 閲嶅 棰 (10201-10205) ──
    #[err(10201, "用户名已存在")]
    DuplicateUsername,
    #[err(10202, "角色编码已存在")]
    DuplicateRoleCode,
    #[err(10203, "权限编码已存在")]
    DuplicatePermCode,
    #[err(10204, "配置键已存在")]
    DuplicateConfigKey,
    #[err(10205, "字典类型已存在")]
    DuplicateDictType,

    // ── 鏍  骞 (10301-10306) ──
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
}
