//! sa-token 配置组件

use sa_token_plugin_axum::SaStorage;
use serde::Deserialize;
use std::sync::Arc;
use tx_di_core::{Component, RIE, Store};
// MemoryStorage 始终可用：sa-token-storage-memory 为直接依赖，不依赖任何 feature。
// 开启 redis feature 时内存存储仍作为兜底（如未配置 redis_url）。
use sa_token_storage_memory::MemoryStorage;

#[cfg(feature = "redis")]
use sa_token_plugin_axum::RedisStorage;

/// sa-token 配置结构体
///
/// 从 TOML 配置文件 `[sa_token]` 节自动加载。
///
/// ```toml
/// [sa_token]
/// token_name = "Authorization"
/// timeout = 86400
/// is_concurrent = true
/// is_share = true
/// token_style = "uuid"
/// is_read_header = true
/// ```
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf = "sa_token", init, init_sort = i32::MIN + 1)]
pub struct SaTokenConf {
    /// Token 名称（读取 Token 的 Header/Parameter/Cookie 键名）
    #[serde(default = "default_token_name")]
    pub token_name: String,

    /// Token 有效期（秒），-1 表示永不过期
    #[serde(default = "default_timeout")]
    pub timeout: i64,

    /// Token 最低活跃频率（秒），-1 表示不限制
    ///
    /// 配合 auto_renew 使用时，表示自动续签的时长
    #[serde(default = "default_active_timeout")]
    pub active_timeout: i64,

    /// 是否开启自动续签
    #[serde(default)]
    pub auto_renew: bool,

    /// 是否允许同一账号并发登录
    #[serde(default = "default_is_concurrent")]
    pub is_concurrent: bool,

    /// 在多人登录同一账号时，是否共享一个 Token
    #[serde(default = "default_is_share")]
    pub is_share: bool,

    /// Token 风格：uuid / simple-uuid / random-32 / random-64 / random-128 / jwt / hash / tik
    #[serde(default = "default_token_style")]
    pub token_style: String,

    /// 是否从请求体读取 Token
    #[serde(default)]
    pub is_read_body: bool,

    /// 是否从 Header 读取 Token
    #[serde(default = "default_true")]
    pub is_read_header: bool,

    /// 是否从 Cookie 读取 Token
    #[serde(default = "default_true")]
    pub is_read_cookie: bool,

    /// 是否输出操作日志
    #[serde(default)]
    pub is_log: bool,

    /// JWT 密钥（如果使用 JWT 风格 Token）
    #[serde(default)]
    pub jwt_secret_key: Option<String>,

    /// JWT 算法（默认 HS256）
    #[serde(default)]
    pub jwt_algorithm: Option<String>,

    /// 是否启用防重放攻击（nonce 机制）
    #[serde(default)]
    pub enable_nonce: bool,

    /// Nonce 有效期（秒），-1 表示使用 token timeout
    #[serde(default = "default_nonce_timeout")]
    pub nonce_timeout: i64,

    /// 是否启用 Refresh Token
    #[serde(default)]
    pub enable_refresh_token: bool,

    /// Refresh Token 有效期（秒），默认 7 天
    #[serde(default = "default_refresh_token_timeout")]
    pub refresh_token_timeout: i64,

    /// Redis 连接字符串（`features = ["redis"]` 时生效）
    ///
    /// 优先级：`redis_url` 配置 > 环境变量 `REDIS_URL` > 默认 `redis://127.0.0.1:6379`。
    /// 生产多实例部署必须配置此项（会话需跨实例共享）。
    #[serde(default)]
    pub redis_url: Option<String>,
}

impl Default for SaTokenConf {
    fn default() -> Self {
        Self {
            token_name: default_token_name(),
            timeout: default_timeout(),
            active_timeout: default_active_timeout(),
            auto_renew: false,
            is_concurrent: default_is_concurrent(),
            is_share: default_is_share(),
            token_style: default_token_style(),
            is_read_body: false,
            is_read_header: true,
            is_read_cookie: true,
            is_log: false,
            jwt_secret_key: None,
            jwt_algorithm: None,
            enable_nonce: false,
            nonce_timeout: default_nonce_timeout(),
            enable_refresh_token: false,
            refresh_token_timeout: default_refresh_token_timeout(),
            redis_url: None,
        }
    }
}

/// `#[component(init)]` 回调：配置加载后打印日志
fn init(this: &mut SaTokenConf, _store: &Store) -> RIE<()> {
    #[cfg(feature = "redis")]
    let storage_backend = match &this.redis_url {
        Some(url) => format!("redis (url: {url})"),
        None => "memory (redis_url 未配置)".to_string(),
    };
    #[cfg(not(feature = "redis"))]
    let storage_backend = "memory".to_string();
    tracing::info!(
        token_name = %this.token_name,
        timeout = this.timeout,
        is_concurrent = this.is_concurrent,
        token_style = %this.token_style,
        storage = %storage_backend,
        "SaToken 配置已加载"
    );
    Ok(())
}

/// 将自定义配置转换为 SaTokenStateBuilder 的链式调用
impl SaTokenConf {
    /// 应用配置到 SaTokenStateBuilder
    ///
    /// 异步：Redis 存储后端需要异步建连，必须在 `async_init` 阶段调用。
    pub async fn apply_to_builder(
        &self,
        builder: sa_token_plugin_axum::SaTokenStateBuilder,
    ) -> RIE<sa_token_plugin_axum::SaTokenStateBuilder> {
        // 根据特性选择存储后端
        let storage = self.create_storage().await?;

        let mut b = builder
            .storage(storage)
            .token_name(&self.token_name)
            .timeout(self.timeout)
            .active_timeout(self.active_timeout)
            .auto_renew(self.auto_renew)
            .is_concurrent(self.is_concurrent)
            .is_share(self.is_share)
            .token_style(parse_token_style(&self.token_style));

        if let Some(ref key) = self.jwt_secret_key {
            b = b.jwt_secret_key(key);
        }

        Ok(b)
    }

    /// 创建存储后端
    ///
    /// 语义（内存始终兜底）：
    /// - 启用 `redis` feature 且配置了 `redis_url` → RedisStorage（多实例共享会话）
    /// - 其他情况 → MemoryStorage（默认/未配置 redis_url 时）
    async fn create_storage(&self) -> RIE<Arc<dyn SaStorage + Send + Sync>> {
        #[cfg(feature = "redis")]
        {
            let redis_url = self
                .redis_url
                .clone()
                .or_else(|| std::env::var("REDIS_URL").ok());
            if let Some(url) = redis_url {
                tracing::info!("使用 Redis 存储后端 (RedisStorage: {url})");
                let storage = RedisStorage::new(&url, "sa-token:").await.map_err(|e| {
                    tx_di_core::AppError::with_context(
                        tx_di_core::DiErr::InjectError,
                        format!("Redis 存储连接失败 ({}): {}", url, e),
                    )
                })?;
                return Ok(Arc::new(storage));
            }
        }

        tracing::info!("使用内存存储后端 (MemoryStorage)");
        Ok(Arc::new(MemoryStorage::new()))
    }
}

fn parse_token_style(s: &str) -> sa_token_plugin_axum::sa_token_core::config::TokenStyle {
    match s {
        "uuid" => sa_token_plugin_axum::sa_token_core::config::TokenStyle::Uuid,
        "simple-uuid" => sa_token_plugin_axum::sa_token_core::config::TokenStyle::SimpleUuid,
        "random-32" => sa_token_plugin_axum::sa_token_core::config::TokenStyle::Random32,
        "random-64" => sa_token_plugin_axum::sa_token_core::config::TokenStyle::Random64,
        "random-128" => sa_token_plugin_axum::sa_token_core::config::TokenStyle::Random128,
        "jwt" => sa_token_plugin_axum::sa_token_core::config::TokenStyle::Jwt,
        "hash" => sa_token_plugin_axum::sa_token_core::config::TokenStyle::Hash,
        "tik" => sa_token_plugin_axum::sa_token_core::config::TokenStyle::Tik,
        "timestamp" => sa_token_plugin_axum::sa_token_core::config::TokenStyle::Timestamp,
        _ => sa_token_plugin_axum::sa_token_core::config::TokenStyle::Uuid,
    }
}

fn default_token_name() -> String {
    "Authorization".to_string()
}

fn default_timeout() -> i64 {
    86400 // 24 小时
}

fn default_active_timeout() -> i64 {
    -1 // 不限制
}

fn default_is_concurrent() -> bool {
    true
}

fn default_is_share() -> bool {
    true
}

fn default_token_style() -> String {
    "uuid".to_string()
}

fn default_true() -> bool {
    true
}

fn default_nonce_timeout() -> i64 {
    -1
}

fn default_refresh_token_timeout() -> i64 {
    604800 // 7 天
}
