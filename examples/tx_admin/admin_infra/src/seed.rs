//! 种子数据定义
//!
//! 定义系统初始化时的默认数据。
//! 所有权限码在此统一管理，handler 通过 `#[sa_check_permission("code")]` 注解引用。

use crate::common::{Sex, Status, Deleted, StorageType};
use crate::user::model::SysUser;
use crate::role::model::{SysRole};
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
        (1, "成功", "1", "success"),
        (2, "失败", "0", "danger"),
    ]),
    ("sys_keep_alive", "缓存策略", &[
        (1, "不缓存", "0", ""),
        (2, "缓存", "1", ""),
    ]),
    ("sys_job_status", "任务状态", &[
        (1, "已暂停", "0", "info"),
        (2, "运行中", "1", "success"),
    ]),
    ("sys_job_log_status", "任务日志状态", &[
        (1, "失败", "0", "danger"),
        (2, "成功", "1", "success"),
        (3, "超时", "2", "warning"),
        (4, "重试中", "3", "info"),
    ]),
    ("sys_file_storage", "文件存储类型", &[
        (1, "本地存储", "0", "info"),
        (2, "S3 对象存储", "1", "warning"),
        (3, "数据库存储", "2", "success"),
    ]),
    ("sys_file_master", "文件主配置", &[
        (1, "普通", "0", "info"),
        (2, "主配置", "1", "success"),
    ]),
];

/// 菜单种子数据
/// (id, name, permission, types, sort, parent_id, route_path, icon, component, component_name, visible, keep_alive)
///
/// types: 0=目录, 1=菜单, 2=按钮/权限
/// visible: 0=显示, 1=隐藏
/// keep_alive: 0=不缓存, 1=缓存
const MENU_SEEDS: &[(u64, &str, &str, i32, i32, u64, &str, &str, &str, &str, i32, i32)] = &[
    // ── 目录 (2位) ──
    (11, "系统管理", "system:view", 0, 1, 0, "system", "Setting", "", "", 0, 0),
    (12, "系统配置", "config:view", 0, 2, 0, "config", "Tools", "", "", 0, 0),
    (13, "日志管理", "log:view", 0, 3, 0, "log", "Notebook", "", "", 0, 0),
    (14, "系统监控", "system:view", 0, 4, 0, "monitor", "Monitor", "", "", 0, 0),

    // ── 系统管理 / 菜单 (4位) ──
    (1101, "用户管理", "user:view", 1, 1, 11, "user", "User", "system/user/index", "User", 0, 1),
    (1102, "角色管理", "role:view", 1, 2, 11, "role", "UserFilled", "system/role/index", "Role", 0, 1),
    (1103, "菜单管理", "menu:view", 1, 3, 11, "menu", "Menu", "system/menu/index", "Menu", 0, 1),
    (1104, "部门管理", "dept:view", 1, 4, 11, "dept", "OfficeBuilding", "system/dept/index", "Dept", 0, 1),
    // ── 系统管理 / 按钮 (8位) ──
    (11010001, "创建用户", "user:create", 2, 1, 1101, "", "", "", "", 0, 0),
    (11010002, "编辑用户", "user:update", 2, 2, 1101, "", "", "", "", 0, 0),
    (11010003, "删除用户", "user:delete", 2, 3, 1101, "", "", "", "", 0, 0),
    (11010004, "用户状态", "user:status", 2, 4, 1101, "", "", "", "", 0, 0),
    (11010005, "重置密码", "user:password", 2, 5, 1101, "", "", "", "", 0, 0),
    (11010006, "分配角色", "user:assign_role", 2, 6, 1101, "", "", "", "", 0, 0),
    (11010007, "分配部门", "user:assign_dept", 2, 7, 1101, "", "", "", "", 0, 0),
    (11010008, "查看用户", "user:view", 2, 8, 1101, "", "", "", "", 0, 0),
    // ── 角色管理 / 按钮 ──
    (11020001, "创建角色", "role:create", 2, 1, 1102, "", "", "", "", 0, 0),
    (11020002, "编辑角色", "role:update", 2, 2, 1102, "", "", "", "", 0, 0),
    (11020003, "删除角色", "role:delete", 2, 3, 1102, "", "", "", "", 0, 0),
    (11020004, "分配菜单", "role:assign_menu", 2, 4, 1102, "", "", "", "", 0, 0),
    (11020005, "查看角色", "role:view", 2, 5, 1102, "", "", "", "", 0, 0),
    // ── 菜单管理 / 按钮 ──
    (11030001, "创建菜单", "menu:create", 2, 1, 1103, "", "", "", "", 0, 0),
    (11030002, "编辑菜单", "menu:update", 2, 2, 1103, "", "", "", "", 0, 0),
    (11030003, "删除菜单", "menu:delete", 2, 3, 1103, "", "", "", "", 0, 0),
    (11030004, "查看菜单", "menu:view", 2, 4, 1103, "", "", "", "", 0, 0),
    // ── 部门管理 / 按钮 ──
    (11040001, "创建部门", "dept:create", 2, 1, 1104, "", "", "", "", 0, 0),
    (11040002, "编辑部门", "dept:update", 2, 2, 1104, "", "", "", "", 0, 0),
    (11040003, "删除部门", "dept:delete", 2, 3, 1104, "", "", "", "", 0, 0),
    (11040004, "查看部门", "dept:view", 2, 4, 1104, "", "", "", "", 0, 0),

    // ── 系统配置 / 菜单 ──
    (1201, "参数设置", "config:view", 1, 1, 12, "index", "Document", "config/config/index", "Config", 0, 1),
    (1202, "字典类型", "dict:view", 1, 2, 12, "dict-type", "Collection", "config/dict/type", "DictType", 0, 1),
    (1203, "字典数据", "dict:view", 1, 3, 12, "dict-data", "Tickets", "config/dict/data", "DictData", 0, 1),
    // ── 系统配置 / 按钮 ──
    (12010001, "创建配置", "config:create", 2, 1, 1201, "", "", "", "", 0, 0),
    (12010002, "编辑配置", "config:update", 2, 2, 1201, "", "", "", "", 0, 0),
    (12010003, "删除配置", "config:delete", 2, 3, 1201, "", "", "", "", 0, 0),
    (12010004, "查看配置", "config:view", 2, 4, 1201, "", "", "", "", 0, 0),
    (12020001, "创建字典类型", "dict:create", 2, 1, 1202, "", "", "", "", 0, 0),
    (12020002, "编辑字典类型", "dict:update", 2, 2, 1202, "", "", "", "", 0, 0),
    (12020003, "删除字典类型", "dict:delete", 2, 3, 1202, "", "", "", "", 0, 0),
    (12020004, "查看字典类型", "dict:view", 2, 4, 1202, "", "", "", "", 0, 0),
    (12030001, "创建字典数据", "dict:create", 2, 1, 1203, "", "", "", "", 0, 0),
    (12030002, "编辑字典数据", "dict:update", 2, 2, 1203, "", "", "", "", 0, 0),
    (12030003, "删除字典数据", "dict:delete", 2, 3, 1203, "", "", "", "", 0, 0),
    (12030004, "查看字典数据", "dict:view", 2, 4, 1203, "", "", "", "", 0, 0),

    // ── 日志管理 / 菜单 ──
    (1301, "操作日志", "log:view", 1, 1, 13, "operate", "List", "log/operate", "OperateLog", 0, 1),
    (1302, "登录日志", "log:view", 1, 2, 13, "login", "Promotion", "log/login", "LoginLog", 0, 1),
    // ── 日志管理 / 按钮 ──
    (13010001, "删除操作日志", "log:delete", 2, 1, 1301, "", "", "", "", 0, 0),
    (13010002, "清空操作日志", "log:clean", 2, 2, 1301, "", "", "", "", 0, 0),
    (13010003, "查看操作日志", "log:view", 2, 3, 1301, "", "", "", "", 0, 0),
    (13020001, "删除登录日志", "log:delete", 2, 1, 1302, "", "", "", "", 0, 0),
    (13020002, "清空登录日志", "log:clean", 2, 2, 1302, "", "", "", "", 0, 0),
    (13020003, "查看登录日志", "log:view", 2, 3, 1302, "", "", "", "", 0, 0),

    // ── 文件管理 (顶层目录) ──
    (15, "文件管理", "", 0, 5, 0, "file", "FolderOpened", "", "", 0, 0),
    (1501, "文件列表", "file:view", 1, 1, 15, "list", "Document", "file/list/index", "FileList", 0, 0),
    (1502, "存储配置", "file:view", 1, 2, 15, "config", "Setting", "file/config/index", "FileConfig", 0, 0),
    // ── 文件列表 / 按钮 ──
    (15010001, "上传文件", "file:upload", 2, 1, 1501, "", "", "", "", 0, 0),
    (15010002, "删除文件", "file:delete", 2, 2, 1501, "", "", "", "", 0, 0),
    (15010003, "下载文件", "file:download", 2, 3, 1501, "", "", "", "", 0, 0),
    (15010004, "查看文件", "file:view", 2, 4, 1501, "", "", "", "", 0, 0),
    // ── 存储配置 / 按钮 ──
    (15020001, "新增配置", "file:upload", 2, 1, 1502, "", "", "", "", 0, 0),
    (15020002, "编辑配置", "file:upload", 2, 2, 1502, "", "", "", "", 0, 0),
    (15020003, "删除配置", "file:delete", 2, 3, 1502, "", "", "", "", 0, 0),
    (15020004, "设为主配置", "file:upload", 2, 4, 1502, "", "", "", "", 0, 0),

    // ── 定时任务 (顶层目录) ──
    (16, "定时任务", "", 0, 6, 0, "job", "Timer", "", "", 0, 0),
    (1601, "任务管理", "job:view", 1, 1, 16, "list", "List", "job/list/index", "JobList", 0, 0),
    (1602, "执行日志", "job:view", 1, 2, 16, "log", "Notebook", "job/log/index", "JobLog", 0, 0),
    // ── 任务管理 / 按钮 ──
    (16010001, "新增任务", "job:create", 2, 1, 1601, "", "", "", "", 0, 0),
    (16010002, "编辑任务", "job:update", 2, 2, 1601, "", "", "", "", 0, 0),
    (16010003, "删除任务", "job:delete", 2, 3, 1601, "", "", "", "", 0, 0),
    (16010004, "启停任务", "job:status", 2, 4, 1601, "", "", "", "", 0, 0),
    (16010005, "执行任务", "job:execute", 2, 5, 1601, "", "", "", "", 0, 0),
    // ── 执行日志 / 按钮 ──
    (16020001, "查看日志", "job:view", 2, 1, 1602, "", "", "", "", 0, 0),
    (16020002, "清空日志", "job:delete", 2, 2, 1602, "", "", "", "", 0, 0),

    // ── 系统监控 / 菜单 ──
    (1401, "服务器信息", "system:view", 1, 1, 14, "server", "Cpu", "monitor/server", "Server", 0, 1),
    (1402, "在线用户", "system:view", 1, 2, 14, "online", "Connection", "monitor/online", "Online", 0, 1),
    // ── 系统监控 / 按钮 ──
    (14000001, "系统监控权限", "system:view", 2, 1, 14, "", "", "", "", 0, 0),

    // ── 认证 (顶层按钮, 无父级) ──
    // (1, "用户信息", "auth:info", 2, 1, 0, "", "", "", "", 0, 0),
    // (2, "登出", "auth:logout", 2, 2, 0, "", "", "", "", 0, 0),
];

/// 执行种子数据初始化
pub async fn seed_data(db: &ToastyDb) -> AppResult<()> {
    let mut db = db.clone();

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
        .updater("system".to_string())
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
        .updater("system".to_string())
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
        .updater("system".to_string())
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
        .updater("system".to_string())
        .deleted(Deleted::No)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("创建根部门失败: {}", e))?;
    info!("已创建根部门: 总公司");

    // 5. 创建字典数据
    for (type_id, (dict_type, type_name, items)) in DICT_SEEDS.iter().enumerate() {
        SysDictType::create()
            .id((type_id + 1) as u64)
            .name(type_name.to_string())
            .dict_type(dict_type.to_string())
            .status(Status::Enabled)
            .remark("".to_string())
            .creator("system".to_string())
            .updater("system".to_string())
            .deleted(Deleted::No)
            .exec(&mut db)
            .await
            .map_err(|e| anyhow::anyhow!("创建字典类型 {} 失败: {}", dict_type, e))?;

        for (item_idx, (sort, label, value, color_type)) in items.iter().enumerate() {
            let data_id = (type_id * 100 + item_idx + 1) as u64;
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
                .updater("system".to_string())
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
            .updater("system".to_string())
            .deleted(Deleted::No)
            .exec(&mut db)
            .await
            .map_err(|e| anyhow::anyhow!("创建菜单 {} 失败: {}", name, e))?;
    }
    info!("已创建 {} 个菜单", MENU_SEEDS.len());

    // 7. 创建默认文件存储配置（本地存储）
    crate::file::model::SysFileConfig::create()
        .id(1)
        .name("本地存储".to_string())
        .storage(StorageType::Local)
        .remark("默认本地文件存储配置".to_string())
        .master(1) // 主配置
        .config(r#"{"base_path":"./uploads","base_url":"http://localhost:8888/files"}"#.to_string())
        .creator("system".to_string())
        .updater("system".to_string())
        .deleted(Deleted::No)
        .exec(&mut db)
        .await
        .map_err(|e| anyhow::anyhow!("创建默认文件存储配置失败: {}", e))?;
    info!("已创建默认文件存储配置: 本地存储");

    // 8. 超级管理员关联所有菜单
    let all_menu_ids: Vec<u64> = MENU_SEEDS.iter().map(|&(id, ..)| id).collect();
    for &menu_id in &all_menu_ids {
        crate::role::model::SysRoleMenu::create()
            .id(menu_id)
            .role_id(1)
            .menu_id(menu_id)
            .exec(&mut db)
            .await
            .map_err(|e| anyhow::anyhow!("关联管理员菜单 {} 失败: {}", menu_id, e))?;
    }
    info!("已为超级管理员关联 {} 个菜单", all_menu_ids.len());

    Ok(())
}
