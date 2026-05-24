//! Toasty 数据库配置

use serde::Deserialize;
use tx_di_core::{tx_comp, CompInit, InnerContext, RIE};

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
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
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
        }
    }
}

impl CompInit for ToastyConfig {
    fn inner_init(&mut self, _ctx: &InnerContext) -> RIE<()> {
        tracing::debug!(
            url = %self.database_url,
            auto_schema = self.auto_schema,
            max_pool = ?self.max_pool_size,
            "Toasty 数据库配置已加载"
        );
        Ok(())
    }

    fn init_sort() -> i32 {
        i32::MIN + 2
    }
}

fn default_database_url() -> String {
    "sqlite://gb28181.db".to_string()
}

fn default_auto_schema() -> bool {
    true
}
