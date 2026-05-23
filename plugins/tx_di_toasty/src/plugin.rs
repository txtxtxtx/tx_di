//! Toasty 数据库插件核心组件

use crate::config::ToastyConfig;
use std::sync::{Arc, OnceLock};
use tx_di_core::{tx_comp, CompInit, InnerContext, RIE};
use tx_di_core::App;
use tokio_util::sync::CancellationToken;

/// Toasty 数据库实例的类型别名
///
/// `toasty::Db` 是 Toasty ORM 的核心类型，封装了数据库连接池和模型注册表。
/// 通过 `ToastyDb` 在 DI 容器中传递，其他组件可以注入并使用。
pub type ToastyDb = toasty::Db;

/// Toasty 数据库插件
///
/// 封装 Toasty ORM 的初始化逻辑，包括：
/// - 数据库连接建立（通过 URL 自动选择驱动）
/// - 模型注册（通过 `toasty::models!` 宏收集所有 `#[derive(Model)]` 结构体）
/// - 连接池配置
/// - Schema 自动推送（可选）
///
/// # DI 注入方式
///
/// ```rust,ignore
/// // 在其他组件中注入
/// #[tx_comp(init)]
/// pub struct MyService {
///     pub toasty: Arc<ToastyPlugin>,  // 自动注入
/// }
/// ```
///
/// # 使用方式
///
/// ```rust,ignore
/// // 获取 Db 实例
/// let db = toasty_plugin.db();
///
/// // CRUD 操作
/// let user = User::create().name("Alice").exec(db).await?;
/// let users = User::all().exec(db).await?;
/// ```
#[derive(Debug)]
#[tx_comp(init)]
pub struct ToastyPlugin {
    /// 配置引用
    pub config: Arc<ToastyConfig>,

    /// Toasty 数据库实例
    ///
    /// 通过 `OnceLock` 延迟初始化，因为 `toasty::Db` 的构建是异步的，
    /// 需要在 `async_init` 阶段完成。
    #[tx_cst(OnceLock::new())]
    pub db: OnceLock<ToastyDb>,
}

impl ToastyPlugin {
    /// 获取已初始化的 Db 引用
    ///
    /// 必须在 `async_init` 完成后调用，否则 panic。
    pub fn db(&self) -> &ToastyDb {
        self.db
            .get()
            .expect("ToastyPlugin: db not initialized yet, async_init not completed")
    }

    /// 尝试获取 Db 引用（安全版本）
    pub fn try_db(&self) -> Option<&ToastyDb> {
        self.db.get()
    }
}

impl CompInit for ToastyPlugin {
    fn inner_init(&mut self, _ctx: &InnerContext) -> RIE<()> {
        tracing::info!("ToastyPlugin 初始化（同步阶段）");
        Ok(())
    }

    fn async_init(ctx: Arc<App>, _token: CancellationToken) -> tx_di_core::BoxFuture {
        Box::pin(async move {
            let plugin = ctx.inject::<ToastyPlugin>();
            let config = plugin.config.clone();

            // 防止重复初始化
            if plugin.db.get().is_some() {
                tracing::warn!("ToastyPlugin: db already initialized, skipping");
                return Ok(());
            }

            tracing::info!(url = %config.database_url, "正在连接数据库...");

            // 构建 Builder 并应用配置
            let mut builder = toasty::Db::builder();

            // 模型注册 — 调用方需在自己的 crate 中通过 build_db_with_models() 注册
            // 此处不注册模型，由业务层处理

            // 连接池配置
            if let Some(max) = config.max_pool_size {
                builder.max_pool_size(max);
            }
            if let Some(ref prefix) = config.table_name_prefix {
                builder.table_name_prefix(prefix);
            }
            if let Some(secs) = config.pool_wait_timeout_secs {
                builder.pool_wait_timeout(Some(std::time::Duration::from_secs(secs)));
            }
            if let Some(secs) = config.pool_create_timeout_secs {
                builder.pool_create_timeout(Some(std::time::Duration::from_secs(secs)));
            }
            if let Some(secs) = config.pool_health_check_interval_secs {
                if secs == 0 {
                    builder.pool_health_check_interval(None);
                } else {
                    builder.pool_health_check_interval(Some(std::time::Duration::from_secs(secs)));
                }
            }
            if let Some(secs) = config.pool_max_connection_lifetime_secs {
                builder.pool_max_connection_lifetime(Some(std::time::Duration::from_secs(secs)));
            }
            if let Some(secs) = config.pool_max_connection_idle_time_secs {
                builder.pool_max_connection_idle_time(Some(std::time::Duration::from_secs(secs)));
            }
            if config.pool_pre_ping {
                builder.pool_pre_ping(true);
            }

            // 通过 URL 连接（自动选择驱动）
            let db = builder
                .connect(&config.database_url)
                .await
                .map_err(|e| anyhow::anyhow!("数据库连接失败 '{}': {}", config.database_url, e))?;

            // 自动推送 Schema（开发环境）
            if config.auto_schema {
                tracing::info!("正在推送数据库 Schema...");
                db.push_schema().await.map_err(|e| {
                    anyhow::anyhow!("Schema 推送失败: {}", e)
                })?;
                tracing::info!("Schema 推送完成");
            }

            // 写入 OnceLock
            if plugin.db.set(db).is_err() {
                tracing::warn!("ToastyPlugin: db concurrently initialized");
            }

            tracing::info!("ToastyPlugin 数据库初始化完成");
            Ok(())
        })
    }

    fn init_sort() -> i32 {
        // 在 ToastyConfig(100) 之后、业务组件之前
        100
    }
}

// ── 模型注册辅助 ─────────────────────────────────────────────────────────────

/// 带模型注册的数据库构建器
///
/// 由于 Toasty 的模型注册需要在 `Db::builder()` 阶段完成，
/// 而 DI 框架的 `async_init` 不支持泛型参数，
/// 本模块提供了在运行时动态注册模型的辅助方法。
///
/// # 使用方式
///
/// ```rust,ignore
/// // 在业务 crate 的 main.rs 中
/// use tx_di_toasty::ToastyPlugin;
///
/// // 使用 toasty::models! 宏注册当前 crate 的所有模型
/// let db = ToastyPlugin::build_db_with_models(
///     toasty::models!(User, Device, Channel),
///     &config,
/// ).await?;
/// ```
impl ToastyPlugin {
    /// 使用指定的模型集合和配置构建数据库
    ///
    /// 适用于需要在 DI 容器外手动初始化数据库、或需要注册模型的场景。
    /// 这是最常用的初始化方式，因为模型必须在 `Db::builder()` 阶段注册。
    pub async fn build_db_with_models(
        models: toasty::ModelSet,
        config: &ToastyConfig,
    ) -> RIE<toasty::Db> {
        let mut builder = toasty::Db::builder();
        builder.models(models);

        // 连接池配置
        if let Some(max) = config.max_pool_size {
            builder.max_pool_size(max);
        }
        if let Some(ref prefix) = config.table_name_prefix {
            builder.table_name_prefix(prefix);
        }
        if let Some(secs) = config.pool_wait_timeout_secs {
            builder.pool_wait_timeout(Some(std::time::Duration::from_secs(secs)));
        }
        if let Some(secs) = config.pool_create_timeout_secs {
            builder.pool_create_timeout(Some(std::time::Duration::from_secs(secs)));
        }
        if let Some(secs) = config.pool_health_check_interval_secs {
            if secs == 0 {
                builder.pool_health_check_interval(None);
            } else {
                builder.pool_health_check_interval(Some(std::time::Duration::from_secs(secs)));
            }
        }
        if let Some(secs) = config.pool_max_connection_lifetime_secs {
            builder.pool_max_connection_lifetime(Some(std::time::Duration::from_secs(secs)));
        }
        if let Some(secs) = config.pool_max_connection_idle_time_secs {
            builder.pool_max_connection_idle_time(Some(std::time::Duration::from_secs(secs)));
        }
        if config.pool_pre_ping {
            builder.pool_pre_ping(true);
        }

        let db = builder
            .connect(&config.database_url)
            .await
            .map_err(|e| anyhow::anyhow!("数据库连接失败 '{}': {}", config.database_url, e))?;

        if config.auto_schema {
            db.push_schema().await.map_err(|e| {
                anyhow::anyhow!("Schema 推送失败: {}", e)
            })?;
        }

        Ok(db)
    }

    /// 仅构建 Schema（不连接数据库）
    ///
    /// 适用于迁移生成器等工具场景。
    pub fn build_schema(models: toasty::ModelSet) -> RIE<toasty::schema::app::Schema> {
        let mut builder = toasty::Db::builder();
        builder.models(models);
        Ok(builder.build_app_schema()
            .map_err(|e| anyhow::anyhow!("Schema 构建失败: {}", e))?)
    }
}
