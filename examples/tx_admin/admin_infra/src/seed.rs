//! 种子数据定义
//!
//! 定义系统初始化时的默认数据。
//! 所有权限码在此统一管理，handler 通过 `#[sa_check_permission("code")]` 注解引用。

use crate::common::{Sex, Status, Deleted};
use crate::user::model::SysUser;
use crate::role::model::SysRole;
use crate::user::model::SysUserRole;
use crate::department::model::SysDepartment;
use crate::permission::model::SysPermission;
use tx_di_toasty::ToastyDb;
use tx_error::AppResult;
use tracing::info;

/// 默认管理员密码（明文，由调用方负责哈希）
pub const DEFAULT_ADMIN_PASSWORD: &str = "admin123";

/// 权限定义：(name, permission_code, type, parent_id, sort, description)
///
/// type: 0=目录, 1=菜单, 2=按钮/接口
pub const PERMISSIONS: &[(&str, &str, i32, i64, i32, &str)] = &[
    // ── 系统管理 ──
    ("系统管理",     "system:view",       0, 0, 1,  "系统管理模块"),

    // ── 用户管理 ──
    ("用户管理",     "user:view",         1, 1, 1,  "查看用户列表"),
    ("创建用户",     "user:create",       2, 2, 1,  "创建新用户"),
    ("编辑用户",     "user:update",       2, 2, 2,  "编辑用户信息"),
    ("删除用户",     "user:delete",       2, 2, 3,  "删除用户"),
    ("用户状态",     "user:status",       2, 2, 4,  "启用/禁用/锁定用户"),
    ("重置密码",     "user:password",     2, 2, 5,  "修改用户密码"),
    ("分配角色",     "user:assign_role",  2, 2, 6,  "分配用户角色"),
    ("分配部门",     "user:assign_dept",  2, 2, 7,  "分配用户部门"),

    // ── 角色管理 ──
    ("角色管理",     "role:view",         1, 1, 2,  "查看角色列表"),
    ("创建角色",     "role:create",       2, 10, 1, "创建新角色"),
    ("编辑角色",     "role:update",       2, 10, 2, "编辑角色信息"),
    ("删除角色",     "role:delete",       2, 10, 3, "删除角色"),
    ("分配菜单",     "role:assign_menu",  2, 10, 4, "分配角色菜单"),

    // ── 菜单管理 ──
    ("菜单管理",     "menu:view",         1, 1, 3,  "查看菜单列表"),
    ("创建菜单",     "menu:create",       2, 15, 1, "创建新菜单"),
    ("编辑菜单",     "menu:update",       2, 15, 2, "编辑菜单信息"),
    ("删除菜单",     "menu:delete",       2, 15, 3, "删除菜单"),

    // ── 部门管理 ──
    ("部门管理",     "dept:view",         1, 1, 4,  "查看部门列表"),
    ("创建部门",     "dept:create",       2, 19, 1, "创建新部门"),
    ("编辑部门",     "dept:update",       2, 19, 2, "编辑部门信息"),
    ("删除部门",     "dept:delete",       2, 19, 3, "删除部门"),

    // ── 权限管理 ──
    ("权限管理",     "permission:view",   1, 1, 5,  "查看权限列表"),
    ("创建权限",     "permission:create", 2, 22, 1, "创建新权限"),
    ("编辑权限",     "permission:update", 2, 22, 2, "编辑权限信息"),
    ("删除权限",     "permission:delete", 2, 22, 3, "删除权限"),

    // ── 配置管理 ──
    ("配置管理",     "config:view",       1, 1, 6,  "查看配置列表"),
    ("创建配置",     "config:create",     2, 25, 1, "创建新配置"),
    ("编辑配置",     "config:update",     2, 25, 2, "编辑配置信息"),
    ("删除配置",     "config:delete",     2, 25, 3, "删除配置"),

    // ── 字典管理 ──
    ("字典管理",     "dict:view",         1, 1, 7,  "查看字典列表"),
    ("创建字典",     "dict:create",       2, 28, 1, "创建字典类型/数据"),
    ("编辑字典",     "dict:update",       2, 28, 2, "编辑字典信息"),
    ("删除字典",     "dict:delete",       2, 28, 3, "删除字典"),

    // ── 文件管理 ──
    ("文件管理",     "file:view",         1, 1, 8,  "查看文件列表"),
    ("上传文件",     "file:upload",       2, 32, 1, "上传文件"),
    ("删除文件",     "file:delete",       2, 32, 2, "删除文件"),
    ("下载文件",     "file:download",     2, 32, 3, "下载文件"),

    // ── 日志管理 ──
    ("日志管理",     "log:view",          1, 1, 9,  "查看日志"),
    ("删除日志",     "log:delete",        2, 36, 1, "删除日志"),
    ("清空日志",     "log:clean",         2, 36, 2, "清空日志"),

    // ── 认证 ──
    ("认证管理",     "auth:view",         1, 1, 10, "认证相关"),
    ("用户信息",     "auth:info",         2, 40, 1, "查看当前用户信息"),
    ("登出",         "auth:logout",       2, 40, 2, "退出登录"),
];

/// 执行种子数据初始化
pub async fn seed_data(db: &ToastyDb) -> AppResult<()> {
    let mut db = db.clone();
    let now = jiff::Timestamp::now().to_string();

    // 1. 创建默认管理员用户（密码由调用方哈希后传入）
    let password_hash = admin_domain::password::hash_password(DEFAULT_ADMIN_PASSWORD)?;
    SysUser::create()
        .id(1)
        .username("admin".to_string())
        .password_hash(password_hash)
        .nickname("超级管理员".to_string())
        .email("admin@example.com".to_string())
        .mobile("13800000000".to_string())
        .sex(Sex::Unknown)
        .avatar("".to_string())
        .status(Status::Enabled)
        .tenant_id(0)
        .creator("system".to_string())
        .created_at(now.clone())
        .updater("system".to_string())
        .updated_at(now.clone())
        .deleted(Deleted::No)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("创建管理员用户失败: {}", e))?;
    info!("已创建默认管理员 admin/admin123");

    // 2. 创建角色
    SysRole::create()
        .id(1)
        .code("admin".to_string())
        .name("超级管理员".to_string())
        .sort(1)
        .data_scope(1)
        .status(Status::Enabled)
        .remark("系统默认角色，拥有全部权限".to_string())
        .creator("system".to_string())
        .created_at(now.clone())
        .updater("system".to_string())
        .updated_at(now.clone())
        .deleted(Deleted::No)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("创建管理员角色失败: {}", e))?;

    SysRole::create()
        .id(2)
        .code("user".to_string())
        .name("普通用户".to_string())
        .sort(2)
        .data_scope(5)
        .status(Status::Enabled)
        .remark("普通用户角色".to_string())
        .creator("system".to_string())
        .created_at(now.clone())
        .updater("system".to_string())
        .updated_at(now.clone())
        .deleted(Deleted::No)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("创建普通用户角色失败: {}", e))?;
    info!("已创建角色: 超级管理员, 普通用户");

    // 3. 关联管理员与角色
    SysUserRole::create()
        .id(1)
        .user_id(1)
        .role_id(1)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("关联管理员角色失败: {}", e))?;

    // 4. 创建根部门
    SysDepartment::create()
        .id(1)
        .name("总公司".to_string())
        .parent_id(0)
        .sort(1)
        .leader_user_id(1)
        .phone("".to_string())
        .email("".to_string())
        .status(Status::Enabled)
        .tenant_id(0)
        .creator("system".to_string())
        .created_at(now.clone())
        .updater("system".to_string())
        .updated_at(now.clone())
        .deleted(Deleted::No)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("创建根部门失败: {}", e))?;
    info!("已创建根部门: 总公司");

    // 5. 创建所有权限
    for (id, (name, code, perm_type, parent_id, sort, desc)) in PERMISSIONS.iter().enumerate() {
        SysPermission::create()
            .id((id + 1) as i64)
            .name(name.to_string())
            .permission_code(code.to_string())
            .permission_type(*perm_type)
            .parent_id(*parent_id)
            .sort(*sort)
            .description(desc.to_string())
            .status(Status::Enabled)
            .creator("system".to_string())
            .created_at(now.clone())
            .updater("system".to_string())
            .updated_at(now.clone())
            .deleted(Deleted::No)
            .exec(&mut db)
            .await
            .map_err(|e| anyhow::anyhow!("创建权限 {} 失败: {}", code, e))?;
    }
    info!("已创建 {} 个权限", PERMISSIONS.len());

    Ok(())
}
