//! 基础设施层插件 - 模型注册与数据初始化
//!
//! 职责：
//! 1. `InfraPlugin` — 注册所有 toasty 模型（在 DB 连接之前）
//! 2. `DbInitPlugin` — 检测首次启动，执行数据初始化（在 DB 连接之后）

use tx_di_core::{tx_comp, App, CancellationToken, CompInit, RIE, async_method, get_sys_config, CONFIG_PATH};
use tx_di_toasty::{ToastyPlugin, ToastyDb, ToastyConfig};
use std::sync::Arc;
use tracing::{info, debug};

/// 模型注册插件
///
/// 在 `ToastyPlugin` 连接数据库之前执行，将模型注册到 `ModelSet`。
#[tx_comp(init)]
pub struct InfraPlugin;

impl CompInit for InfraPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let toasty_plugin = ctx.inject::<ToastyPlugin>();
            toasty_plugin.register_models(crate::register_models());
            info!("infra: toasty 模型已注册");
            Ok(())
        }
    );

    fn init_sort() -> i32 {
        // 必须在 ToastyPlugin（MAX-50）之前，确保模型在 DB 连接前注册
        i32::MAX - 200
    }
}

/// 数据库初始化插件
///
/// 在 `ToastyPlugin` 连接数据库之后执行，检测空数据库并初始化基础数据。
#[tx_comp(init)]
pub struct DbInitPlugin;

/// 默认管理员密码
const DEFAULT_ADMIN_PASSWORD: &str = "admin123";

impl CompInit for DbInitPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let toasty_plugin = ctx.inject::<ToastyPlugin>();
            let toasty_config = ctx.inject::<ToastyConfig>();
            let db = toasty_plugin.db();
            if toasty_config.auto_schema {
                info!("infra: 检测到空数据库，开始初始化数据...");
                init_data(db).await?;
            } else{
                debug!("infra: 数据库已有数据，跳过初始化");
            }
            Ok(())
        }
    );

    fn init_sort() -> i32 {
        // 在 ToastyPlugin（MAX-50）之后，确保 DB 已连接
        i32::MAX - 25
    }
}

/// 检测数据库是否需要初始化
///
/// 通过查询 sys_user 表是否有数据来判断
async fn needs_init(db: &ToastyDb) -> bool {
    use crate::user::model::SysUser;

    let mut db = db.clone();
    match SysUser::all().count().exec(&mut db).await {
        Ok(count) => count == 0,
        Err(_) => true, // 表不存在也算需要初始化
    }
}

/// 执行数据初始化
///
/// 创建默认管理员账号、角色、权限等基础数据
#[allow(dead_code)]
async fn init_data(db: &ToastyDb) -> RIE<()> {
    use crate::user::model::SysUser;
    use crate::role::model::SysRole;
    use crate::permission::model::SysPermission;
    use crate::user::model::SysUserRole;

    let mut db = db.clone();
    let now = jiff::Timestamp::now().to_string();

    // 1. 创建默认管理员用户
    let password_hash = admin_domain::password::hash_password(DEFAULT_ADMIN_PASSWORD)
        .map_err(|e| anyhow::anyhow!("密码哈希失败: {}", e))?;
    let _admin_user = SysUser::create()
        .id(1)
        .username("admin".to_string())
        .password_hash(password_hash)
        .nickname("超级管理员".to_string())
        .email("admin@example.com".to_string())
        .mobile("13800000000".to_string())
        .sex(0)
        .avatar("".to_string())
        .status(0) // 正常
        .tenant_id(0)
        .creator("system".to_string())
        .created_at(now.clone())
        .updater("system".to_string())
        .updated_at(now.clone())
        .deleted(0)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("创建管理员用户失败: {}", e))?;
    info!("infra: 已创建默认管理员 admin/admin123");

    // 2. 创建默认角色
    let _admin_role = SysRole::create()
        .id(1)
        .code("admin".to_string())
        .name("超级管理员".to_string())
        .sort(1)
        .data_scope(1) // 全部数据权限
        .status(0)
        .remark("系统默认角色，拥有全部权限".to_string())
        .creator("system".to_string())
        .created_at(now.clone())
        .updater("system".to_string())
        .updated_at(now.clone())
        .deleted(0)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("创建管理员角色失败: {}", e))?;

    let _normal_role = SysRole::create()
        .id(2)
        .code("user".to_string())
        .name("普通用户".to_string())
        .sort(2)
        .data_scope(5) // 仅本人数据
        .status(0)
        .remark("普通用户角色".to_string())
        .creator("system".to_string())
        .created_at(now.clone())
        .updater("system".to_string())
        .updated_at(now.clone())
        .deleted(0)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("创建普通用户角色失败: {}", e))?;
    info!("infra: 已创建默认角色: 超级管理员, 普通用户");

    // 3. 关联管理员用户与角色
    SysUserRole::create()
        .id(1)
        .user_id(1)
        .role_id(1)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("关联管理员角色失败: {}", e))?;

    // 4. 创建基础权限
    let permissions = vec![
        (1, "system", "system:view", 0, "系统管理", 1),
        (2, "user", "user:manage", 1, "用户管理", 1),
        (3, "role", "role:manage", 1, "角色管理", 1),
        (4, "menu", "menu:manage", 1, "菜单管理", 1),
        (5, "dept", "dept:manage", 1, "部门管理", 1),
        (6, "config", "config:manage", 1, "配置管理", 1),
        (7, "dict", "dict:manage", 1, "字典管理", 1),
        (8, "file", "file:manage", 1, "文件管理", 1),
        (9, "log", "log:view", 1, "日志查看", 1),
    ];

    for (id, name, code, perm_type, desc, sort) in permissions {
        SysPermission::create()
            .id(id)
            .name(name.to_string())
            .permission_code(code.to_string())
            .permission_type(perm_type)
            .parent_id(0)
            .sort(sort)
            .description(desc.to_string())
            .status(0)
            .creator("system".to_string())
            .created_at(now.clone())
            .updater("system".to_string())
            .updated_at(now.clone())
            .deleted(0)
            .exec(&mut db)
            .await
            .map_err(|e| anyhow::anyhow!("创建权限 {} 失败: {}", code, e))?;
    }
    info!("infra: 已创建 {} 个基础权限", 9);

    Ok(())
}
