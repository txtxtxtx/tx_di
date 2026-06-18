//! 种子数据定义
//!
//! 定义系统初始化时的默认数据。
//! 所有权限码在此统一管理，handler 通过 `#[sa_check_permission("code")]` 注解引用。

use crate::common::{Sex, Status, Deleted};
use crate::user::model::SysUser;
use crate::role::model::{SysRole, SysRoleMenu};
use crate::user::model::SysUserRole;
use crate::department::model::SysDepartment;
use crate::dictionary::model::{SysDictType, SysDictData};
use crate::menu::model::SysMenu;
use tx_di_toasty::ToastyDb;
use tx_error::AppResult;
use tracing::info;

/// 默认管理员密码（明文，由调用方负责哈希）
pub const DEFAULT_ADMIN_PASSWORD: &str = "admin123";

/// 字典种子数据：(dict_type, type_name, data_items)
/// data_items: (sort, label, value, color_type)
const DICT_SEEDS: &[(&str, &str, &[(i32, &str, &str, &str)])] = &[
    ("sys_status", "通用状态", &[
        (1, "正常", "0", "success"),
        (2, "停用", "1", "danger"),
    ]),
    ("sys_user_status", "用户状态", &[
        (1, "正常", "0", "success"),
        (2, "停用", "1", "danger"),
        (3, "锁定", "2", "warning"),
    ]),
    ("sys_sex", "性别", &[
        (1, "未知", "0", "info"),
        (2, "男", "1", ""),
        (3, "女", "2", ""),
    ]),
    ("sys_menu_type", "菜单类型", &[
        (1, "目录", "0", ""),
        (2, "菜单", "1", ""),
        (3, "按钮", "2", ""),
    ]),
    ("sys_permission_type", "权限类型", &[
        (1, "菜单", "0", ""),
        (2, "按钮", "1", ""),
        (3, "API", "2", ""),
    ]),
    ("sys_config_type", "配置类型", &[
        (1, "系统", "1", ""),
        (2, "自定义", "2", ""),
    ]),
    ("sys_visible", "可见性", &[
        (1, "显示", "0", "success"),
        (2, "隐藏", "1", "danger"),
    ]),
    ("sys_data_scope", "数据范围", &[
        (1, "全部数据", "1", ""),
        (2, "自定义数据", "2", ""),
        (3, "本部门数据", "3", ""),
        (4, "本部门及以下", "4", ""),
        (5, "仅本人数据", "5", ""),
    ]),
    ("sys_operate_result", "操作结果", &[
        (1, "成功", "0", "success"),
        (2, "失败", "1", "danger"),
    ]),
    ("sys_keep_alive", "缓存策略", &[
        (1, "不缓存", "0", ""),
        (2, "缓存", "1", ""),
    ]),
];

/// 菜单种子数据
/// (id, name, permission, types, sort, parent_id, route_path, icon, component, component_name, visible, keep_alive)
///
/// types: 0=目录, 1=菜单, 2=按钮/权限
/// visible: 0=显示, 1=隐藏
/// keep_alive: 0=不缓存, 1=缓存
const MENU_SEEDS: &[(i64, &str, &str, i32, i32, i64, &str, &str, &str, &str, i32, i32)] = &[
    // ── 仪表盘 ──
    (1, "仪表盘", "", 1, 0, 0, "dashboard", "Odometer", "dashboard/index", "Dashboard", 0, 0),

    // ── 系统管理 ──
    (2, "系统管理", "system:view", 0, 1, 0, "system", "Setting", "", "", 0, 0),

    // 用户管理
    (3, "用户管理", "user:view", 1, 1, 2, "user", "User", "system/user/index", "User", 0, 1),
    (101, "创建用户", "user:create", 2, 1, 3, "", "", "", "", 0, 0),
    (102, "编辑用户", "user:update", 2, 2, 3, "", "", "", "", 0, 0),
    (103, "删除用户", "user:delete", 2, 3, 3, "", "", "", "", 0, 0),
    (104, "用户状态", "user:status", 2, 4, 3, "", "", "", "", 0, 0),
    (105, "重置密码", "user:password", 2, 5, 3, "", "", "", "", 0, 0),
    (106, "分配角色", "user:assign_role", 2, 6, 3, "", "", "", "", 0, 0),
    (107, "分配部门", "user:assign_dept", 2, 7, 3, "", "", "", "", 0, 0),

    // 角色管理
    (4, "角色管理", "role:view", 1, 2, 2, "role", "UserFilled", "system/role/index", "Role", 0, 1),
    (108, "创建角色", "role:create", 2, 1, 4, "", "", "", "", 0, 0),
    (109, "编辑角色", "role:update", 2, 2, 4, "", "", "", "", 0, 0),
    (110, "删除角色", "role:delete", 2, 3, 4, "", "", "", "", 0, 0),
    (111, "分配菜单", "role:assign_menu", 2, 4, 4, "", "", "", "", 0, 0),

    // 菜单管理
    (5, "菜单管理", "menu:view", 1, 3, 2, "menu", "Menu", "system/menu/index", "Menu", 0, 1),
    (112, "创建菜单", "menu:create", 2, 1, 5, "", "", "", "", 0, 0),
    (113, "编辑菜单", "menu:update", 2, 2, 5, "", "", "", "", 0, 0),
    (114, "删除菜单", "menu:delete", 2, 3, 5, "", "", "", "", 0, 0),

    // 部门管理
    (6, "部门管理", "dept:view", 1, 4, 2, "dept", "OfficeBuilding", "system/dept/index", "Dept", 0, 1),
    (115, "创建部门", "dept:create", 2, 1, 6, "", "", "", "", 0, 0),
    (116, "编辑部门", "dept:update", 2, 2, 6, "", "", "", "", 0, 0),
    (117, "删除部门", "dept:delete", 2, 3, 6, "", "", "", "", 0, 0),

    // ── 系统配置 ──
    (8, "系统配置", "config:view", 0, 2, 0, "config", "Tools", "", "", 0, 0),
    (9, "参数设置", "config:view", 1, 1, 8, "index", "Document", "config/config/index", "Config", 0, 1),
    (121, "创建配置", "config:create", 2, 1, 9, "", "", "", "", 0, 0),
    (122, "编辑配置", "config:update", 2, 2, 9, "", "", "", "", 0, 0),
    (123, "删除配置", "config:delete", 2, 3, 9, "", "", "", "", 0, 0),

    (10, "字典类型", "dict:view", 1, 2, 8, "dict-type", "Collection", "config/dict/type", "DictType", 0, 1),
    (124, "创建字典类型", "dict:create", 2, 1, 10, "", "", "", "", 0, 0),
    (125, "编辑字典类型", "dict:update", 2, 2, 10, "", "", "", "", 0, 0),
    (126, "删除字典类型", "dict:delete", 2, 3, 10, "", "", "", "", 0, 0),

    (11, "字典数据", "dict:view", 1, 3, 8, "dict-data", "Tickets", "config/dict/data", "DictData", 0, 1),
    (127, "创建字典数据", "dict:create", 2, 1, 11, "", "", "", "", 0, 0),
    (128, "编辑字典数据", "dict:update", 2, 2, 11, "", "", "", "", 0, 0),
    (129, "删除字典数据", "dict:delete", 2, 3, 11, "", "", "", "", 0, 0),

    // ── 日志管理 ──
    (12, "日志管理", "log:view", 0, 3, 0, "log", "Notebook", "", "", 0, 0),
    (13, "操作日志", "log:view", 1, 1, 12, "operate", "List", "log/operate", "OperateLog", 0, 1),
    (130, "删除操作日志", "log:delete", 2, 1, 13, "", "", "", "", 0, 0),
    (131, "清空操作日志", "log:clean", 2, 2, 13, "", "", "", "", 0, 0),
    (14, "登录日志", "log:view", 1, 2, 12, "login", "Promotion", "log/login", "LoginLog", 0, 1),
    (132, "删除登录日志", "log:delete", 2, 1, 14, "", "", "", "", 0, 0),
    (133, "清空登录日志", "log:clean", 2, 2, 14, "", "", "", "", 0, 0),

    // ── 文件管理 ──
    (15, "文件管理", "file:view", 1, 4, 0, "file", "FolderOpened", "file/index", "File", 0, 0),
    (134, "上传文件", "file:upload", 2, 1, 15, "", "", "", "", 0, 0),
    (135, "删除文件", "file:delete", 2, 2, 15, "", "", "", "", 0, 0),
    (136, "下载文件", "file:download", 2, 3, 15, "", "", "", "", 0, 0),

    // ── 系统监控 ──
    (16, "系统监控", "", 0, 5, 0, "monitor", "Monitor", "", "", 0, 0),
    (17, "服务器信息", "", 1, 1, 16, "server", "Cpu", "monitor/server", "Server", 0, 1),
    (18, "在线用户", "", 1, 2, 16, "online", "Connection", "monitor/online", "Online", 0, 1),

    // ── 认证 ──
    (137, "用户信息", "auth:info", 2, 1, 1, "", "", "", "", 0, 0),
    (138, "登出", "auth:logout", 2, 2, 1, "", "", "", "", 0, 0),
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

    // 5. 创建字典数据
    for (type_id, (dict_type, type_name, items)) in DICT_SEEDS.iter().enumerate() {
        SysDictType::create()
            .id((type_id + 1) as i64)
            .name(type_name.to_string())
            .dict_type(dict_type.to_string())
            .status(Status::Enabled)
            .remark("".to_string())
            .creator("system".to_string())
            .created_at(now.clone())
            .updater("system".to_string())
            .updated_at(now.clone())
            .deleted(Deleted::No)
            .exec(&mut db)
            .await
            .map_err(|e| anyhow::anyhow!("创建字典类型 {} 失败: {}", dict_type, e))?;

        for (item_idx, (sort, label, value, color_type)) in items.iter().enumerate() {
            let data_id = (type_id * 100 + item_idx + 1) as i64;
            SysDictData::create()
                .id(data_id)
                .sort(*sort)
                .label(label.to_string())
                .value(value.to_string())
                .dict_type(dict_type.to_string())
                .status(Status::Enabled)
                .color_type(color_type.to_string())
                .css_class("".to_string())
                .remark("".to_string())
                .creator("system".to_string())
                .created_at(now.clone())
                .updater("system".to_string())
                .updated_at(now.clone())
                .deleted(Deleted::No)
                .exec(&mut db)
                .await
                .map_err(|e| anyhow::anyhow!("创建字典数据 {}/{} 失败: {}", dict_type, label, e))?;
        }
    }
    info!("已创建 {} 个字典类型", DICT_SEEDS.len());

    // 6. 创建菜单
    for &(id, name, permission, types, sort, parent_id, route_path, icon, component, component_name, visible, keep_alive) in MENU_SEEDS {
        SysMenu::create()
            .id(id)
            .name(name.to_string())
            .permission(permission.to_string())
            .types(types)
            .sort(sort)
            .parent_id(parent_id)
            .route_path(route_path.to_string())
            .icon(icon.to_string())
            .component(component.to_string())
            .component_name(component_name.to_string())
            .status(Status::Enabled)
            .visible(visible)
            .keep_alive(keep_alive)
            .tenant_id(0)
            .creator("system".to_string())
            .created_at(now.clone())
            .updater("system".to_string())
            .updated_at(now.clone())
            .deleted(Deleted::No)
            .exec(&mut db)
            .await
            .map_err(|e| anyhow::anyhow!("创建菜单 {} 失败: {}", name, e))?;
    }
    info!("已创建 {} 个菜单", MENU_SEEDS.len());

    // 7. 超级管理员关联所有菜单
    let all_menu_ids: Vec<u64> = MENU_SEEDS.iter().map(|&(id, ..)| id as u64).collect();
    for &menu_id in &all_menu_ids {
        crate::role::model::SysRoleMenu::create()
            .id(menu_id as i64)
            .role_id(1)
            .menu_id(menu_id as i64)
            .exec(&mut db)
            .await
            .map_err(|e| anyhow::anyhow!("关联管理员菜单 {} 失败: {}", menu_id, e))?;
    }
    info!("已为超级管理员关联 {} 个菜单", all_menu_ids.len());

    Ok(())
}
