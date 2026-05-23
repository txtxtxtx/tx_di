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
use tx_di_core::{tx_comp, CompInit, InnerContext, RIE};
use tx_di_core::App;
use tokio_util::sync::CancellationToken;

// ── 全局模型注册表 ────────────────────────────────────────────────────────
//
// 为什么用 RwLock 而不是 OnceLock？
// OnceLock 只能 set 一次，但业务可能需要多个独立模块分别注册模型
// （例如：主应用注册 User/Device，审计插件注册 AuditLog）。
// RwLock 允许多次合并写入，最后 async_init 读取一次。
static GLOBAL_MODELS: RwLock<Option<toasty::ModelSet>> = RwLock::new(None);

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
}

impl ToastyPlugin {
    /// 注册模型到全局模型池
    ///
    /// 可以在 `BuildContext::new()` 之后、`build()` 之前的任意时间调用，
    /// 多次调用会**合并**模型（重复 `ModelId` 自动覆盖，不会报错）。
    ///
    /// # 使用示例
    ///
    /// ```rust,ignore
    /// // main.rs 或各插件的注册函数中
    /// ToastyPlugin::register_models(toasty::models!(User, Device, Channel));
    ///
    /// // 另一个插件也可以继续追加
    /// ToastyPlugin::register_models(toasty::models!(AuditLog, GbDeviceGroup));
    /// ```
    pub fn register_models(models: toasty::ModelSet) {
        let mut global = GLOBAL_MODELS.write().unwrap();
        if let Some(ref mut existing) = *global {
            // 合并：逐个 add，重复 ModelId 自动覆盖（合并语义）
            for model in models {
                existing.add(model);
            }
        } else {
            *global = Some(models);
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

    /// 清除全局模型注册表（主要用于测试）
    #[cfg(test)]
    pub fn clear_registered_models() {
        *GLOBAL_MODELS.write().unwrap() = None;
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

            // ── 同步读取全局模型（不持有锁跨越 await）────────────────
            let maybe_models = {
                let global = GLOBAL_MODELS.read().unwrap();
                global.clone() // Option<ModelSet> 需要 ModelSet: Clone
            }; // global guard 在这里 drop

            tracing::info!(url = %config.database_url, "正在连接数据库...");

            // 构建 Builder 并应用配置
            let mut builder = toasty::Db::builder();

            // ── 注册模型 ─────────────────────────────────────
            if let Some(ref models) = maybe_models {
                tracing::info!(model_count = models.len(), "注册模型到 Toasty Db::builder");
                builder.models(models.clone());
            } else {
                tracing::warn!("ToastyPlugin: 未注册任何模型，Db 将在无模型状态下初始化");
            }

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
