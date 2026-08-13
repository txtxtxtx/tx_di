//! Toasty 数据库配置

use serde::Deserialize;
use tx_di_core::{Component, RIE, Store};

/// Toasty 数据库配置结构体
///
/// 从 TOML 配置文件 `[toasty_config]` 节自动加载。
///
/// ```toml
/// [toasty_config]
/// database_url = "sqlite://gb28181.db"
/// auto_schema = true
/// max_pool_size = 10
/// table_name_prefix = ""
/// ```
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf = "toasty", init, init_sort = i32::MIN + 2)]
pub struct ToastyConfig {
    /// 数据库连接字符串
    ///
    /// 支持格式（由 URL scheme 自动选择驱动，需启用对应 feature）：
    /// - SQLite: `"sqlite://path/to/db.db"` 或 `"sqlite://memory"`
    /// - PostgreSQL: `"postgresql://user:pass@host:port/database"`
    /// - MySQL: `"mysql://user:pass@host:port/database"`
    /// - DynamoDB: `"dynamodb://endpoint/region/table_prefix"`
    #[serde(default = "default_database_url")]
    pub database_url: String,

    /// 是否自动推送 Schema（创建/更新数据表）
    ///
    /// - `true`: 启动时自动调用 `db.push_schema()`（开发环境推荐）
    /// - `false`: 手动管理数据库迁移（生产环境推荐）
    #[serde(default = "default_auto_schema")]
    pub auto_schema: bool,

    /// 连接池最大连接数
    ///
    /// 默认为 `num_cpus * 2`。驱动可能会限制此值（如内存 SQLite 强制单连接）。
    #[serde(default)]
    pub max_pool_size: Option<usize>,

    /// 表名前缀
    ///
    /// 所有表名会自动添加此前缀，如 `"app_"` → `"app_users"`。
    #[serde(default)]
    pub table_name_prefix: Option<String>,

    /// 从连接池获取空闲连接的最大等待时间（秒）
    ///
    /// 超时返回错误。`None` 表示无限等待（默认）。
    #[serde(default)]
    pub pool_wait_timeout_secs: Option<u64>,

    /// 建立新数据库连接的最大允许时间（秒）
    #[serde(default)]
    pub pool_create_timeout_secs: Option<u64>,

    /// 连接池后台健康检查间隔（秒）
    ///
    /// 定期 ping 空闲连接以检测静默断开。默认 60 秒。
    /// 设为 0 禁用后台扫描。
    #[serde(default)]
    pub pool_health_check_interval_secs: Option<u64>,

    /// 连接最大存活时间（秒）
    ///
    /// 适用于负载均衡器/服务器空闲超时场景，推荐远程数据库设为 1800（30分钟）。
    #[serde(default)]
    pub pool_max_connection_lifetime_secs: Option<u64>,

    /// 连接最大空闲时间（秒）
    ///
    /// 驱逐空闲时间超过此值的连接。
    #[serde(default)]
    pub pool_max_connection_idle_time_secs: Option<u64>,

    /// 是否启用 pre-ping（每次取出连接前先 ping）
    ///
    /// 适用于不能容忍任何失败查询的部署。代价是每次检出增加一次往返。
    #[serde(default)]
    pub pool_pre_ping: bool,

    /// 默认管理员密码
    ///
    /// 当数据库为空（无任何用户）时，自动创建用户 `admin` + 此密码。
    /// 登录后应立即修改。默认值 `"admin123"`。
    #[serde(default = "default_admin_password")]
    pub default_admin_password: String,

    /// 启动时执行受控 Schema 迁移（生产环境推荐）
    ///
    /// `auto_schema=false` 时，若此项为 `true`，启动时调用 `ToastyPlugin::migrate()`：
    /// 1. 创建版本审计表 `_schema_migrations`
    /// 2. 执行 `db.push_schema()`（toasty 对模型与库结构做 diff，仅执行增量 DDL）
    /// 3. 记录迁移时间
    ///
    /// 与 `auto_schema=true` 的区别：迁移受控、可审计，且不会自动改写配置文件。
    #[serde(default)]
    pub migrate_on_start: bool,
}

impl Default for ToastyConfig {
    fn default() -> Self {
        Self {
            database_url: default_database_url(),
            auto_schema: default_auto_schema(),
            max_pool_size: None,
            table_name_prefix: None,
            pool_wait_timeout_secs: None,
            pool_create_timeout_secs: None,
            pool_health_check_interval_secs: None,
            pool_max_connection_lifetime_secs: None,
            pool_max_connection_idle_time_secs: None,
            pool_pre_ping: false,
            default_admin_password: default_admin_password(),
            migrate_on_start: false,
        }
    }
}

/// `#[component(init)]` 回调：解析相对路径并打印日志
fn init(this: &mut ToastyConfig, _store: &Store) -> RIE<()> {
    // 生产部署通过环境变量 DATABASE_URL 覆盖连接串（容器/多实例场景），
    // 优先级高于 TOML 配置，避免镜像内硬编码数据库口令。
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if !url.trim().is_empty() {
            this.database_url = url;
        }
    }
    // 生产部署通过 APP_HOME 锚定 SQLite 相对路径，避免依赖进程工作目录
    this.database_url = tx_di_core::resolve_sqlite_url(&this.database_url);
    tracing::debug!(
        url = %this.database_url,
        auto_schema = this.auto_schema,
        max_pool = ?this.max_pool_size,
        "Toasty ORM 数据库配置已加载"
    );
    Ok(())
}

fn default_database_url() -> String {
    "sqlite://gb28181.db".to_string()
}

fn default_auto_schema() -> bool {
    true
}

fn default_admin_password() -> String {
    "admin123".to_string()
}
