//! Toasty 数据库插件核心组件
//!
//! 设计说明：
//! =====
//! Toasty 要求模型在 `Db::builder()` 阶段通过 `builder.models(models)` 注册，
//! 这比 DI 容器的 `async_init` 阶段更早。
//!
//! 解决方案：静态 `GLOBAL_MODELS` + `register_models()`
//! --------------------------------------------------
//! 1. 业务 crate 在 `BuildContext::new()` 之后、`build()` 之前，
//!    调用 `ToastyPlugin::register_models(models!(&...))` 注册模型。
//! 2. 可以多次调用 `register_models()`，后调用的模型会合并进去
//!    （重复 ModelId 自动覆盖，不会报错）。
//! 3. `async_init` 执行时，从 `GLOBAL_MODELS` 读取合并后的 ModelSet，
//!    注册到 builder 并连接数据库。
//!
//! 使用方式：
//! ```rust,ignore
//! use tx_di_toasty::ToastyPlugin;
//!
//! // 在 main.rs 中，build() 之前
//! ToastyPlugin::register_models(toasty::models!(User, Device));
//! // 另一个插件/模块也可以继续注册
//! ToastyPlugin::register_models(toasty::models!(AuditLog));
//!
//! let app = BuildContext::new(Some("config.toml")).build()?.ins_run().await?;
//! let db = app.inject::<ToastyPlugin>().db();
//! ```

use crate::config::ToastyConfig;
use std::sync::{Arc, OnceLock, RwLock};
use toasty::ModelSet;
use tokio::runtime::Handle;
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
/// - 模型注册（通过 `ToastyPlugin::register_models()` 在 build 前注册）
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
/// # 模型注册（必须在 build() 之前完成）
///
/// ```rust,ignore
/// // main.rs
/// ToastyPlugin::register_models(toasty::models!(User, Device));
/// // 可多次调用，模型会合并
/// ToastyPlugin::register_models(toasty::models!(AuditLog));
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
    /// 模型池
    #[tx_cst(Arc::new(RwLock::new(ModelSet::new())))]
    pub models: Arc<RwLock<ModelSet>>,
}

impl ToastyPlugin {

    /// 注册数据库模型到插件的模型集合中
    ///
    /// 此方法用于在 DI 容器构建之前将业务模型注册到 Toasty ORM。
    /// 支持多次调用，新注册的模型会与已有模型合并，相同 ModelId 的模型会被自动覆盖。
    ///
    /// # 参数
    ///
    /// - `models`: 要注册的模型集合，包含一个或多个实现了 `toasty::Model` trait 的模型
    ///
    /// # 使用示例
    ///
    /// ```rust,ignore
    /// // 单次注册多个模型
    /// toasty_plugin::register_models(toasty::models!(User, Device));
    ///
    /// // 可以多次调用，模型会累积合并
    /// toasty_plugin::register_models(toasty::models!(AuditLog));
    /// ```
    ///
    /// # 注意事项
    ///
    /// - 必须在 `APP::run()` 之前调用
    /// - 如果同一个 ModelId 被多次注册，后注册的模型定义会覆盖之前的
    /// - 此方法是线程安全的，内部使用写锁保护
    pub fn register_models(&self, models: ModelSet) {
        let mut inner_models = self.models.write().unwrap();

        for model in models {
            inner_models.add(model);
        }
    }

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

    fn init(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
        let plugin = ctx.inject::<ToastyPlugin>();
        let config = plugin.config.clone();

        // 防止重复初始化
        if plugin.db.get().is_some() {
            tracing::warn!("ToastyPlugin: db already initialized, skipping");
            return Ok(());
        }

        // ── 同步读取全局模型（不持有锁跨越 await）────────────────
        let models = {
            let models = plugin.models.read()
                .map_err(|e| anyhow::anyhow!("无法获取注册的模型:{e}"))?;
            models.clone()
        }; // models guard 在这里 drop
        tracing::info!(url = %config.database_url, "正在连接数据库...");
        // 构建 Builder 并应用配置
        let mut builder = toasty::Db::builder();
        tracing::info!(model_count = models.len(), "注册模型到 Toasty Db::builder");
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
        let rt_handle = Handle::current();
        // 通过 URL 连接（自动选择驱动）
        let db = rt_handle.block_on(async {
            builder
                .connect(&config.database_url)
                .await
                .map_err(|e| anyhow::anyhow!("数据库连接失败 '{}': {}", config.database_url, e))
        })?;
        // 自动推送 Schema（开发环境）
        if config.auto_schema {
            tracing::info!("正在推送数据库 Schema...");
            rt_handle.block_on(async {
                db.push_schema().await.map_err(|e| {
                    anyhow::anyhow!("Schema 推送失败: {}", e)
                })
            })?;
            tracing::info!("Schema 推送完成");
        }
        // 写入 OnceLock
        if plugin.db.set(db).is_err() {
            tracing::warn!("ToastyPlugin: db concurrently initialized");
        }
        tracing::info!("ToastyPlugin 数据库初始化完成");
        Ok(())
    }

    fn init_sort() -> i32 {
        i32::MAX - 1
    }
}

// ── 兼容辅助：手动构建数据库（不依赖 DI 异步初始化）─────────────────────
//
// 适用于：
// - 测试代码
// - 需要在 DI build() 之前拿到 Db 实例的场景
// - 工具程序（迁移生成器等）
//
// 注意：如果在调用此函数之前已经调用过 `register_models()`，
//       此处会**重新注册**全局模型并构建新 Db 实例，
//       DI 容器中的 ToastyPlugin 不会自动感知此次构建。
impl ToastyPlugin {
    /// 使用指定的模型集合和配置构建数据库
    ///
    /// 这是一个**兼容辅助函数**，适用于测试或工具程序。
    ///
    /// 在正常的 DI 流程中，不需要手动调用此函数——
    /// 只需在 `build()` 之前调用 `register_models()`，
    /// `async_init` 会自动完成数据库构建。
    ///
    /// # 使用示例
    ///
    /// ```rust,ignore
    /// // 测试代码
    /// let config = ToastyConfig { database_url: "sqlite://:memory:".into(), .. };
    /// let db = ToastyPlugin::build_db_with_models(
    ///     toasty::models!(User, Device),
    ///     &config,
    /// ).await?;
    /// ```
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
