# tx_di_cache — 缓存插件使用文档

基于 `tx-di-core` 的**统一缓存抽象插件**，对外暴露 `CacheService` trait（异步），屏蔽底层存储差异。

## 用途

- 提供与 Redis 原生一致的 **5 种数据类型**：String / Hash / List / Set / SortedSet，外加批量操作与按 glob 模式清理。
- 默认后端为**进程内内存缓存 `MemoryCache`**（基于 `DashMap`，惰性过期）。
- 可选启用 `redis` feature 使用 **Redis 后端 `RedisCache`**（基于 `redis` crate 连接池，延迟连接）。
- 业务代码通过 DI 注入 `Arc<dyn CacheService>`，不感知底层实现。

## 启用

`Cargo.toml`：

```toml
tx_di_cache = { path = "plugins/tx_di_cache" }                        # 仅内存
# tx_di_cache = { path = "plugins/tx_di_cache", features = ["redis"] } # 启用 Redis 后端
```

## 配置

TOML 节名为 `[cache_config]`：

```toml
[cache_config]
default_ttl_secs = 3600        # 默认 TTL(秒)
key_prefix = ""                # 键前缀，隔离不同应用
memory_max_capacity = 10000    # 内存缓存最大条目数，0 = 无限制
redis_url = "redis://127.0.0.1:6379"   # 仅 feature="redis" 时有效
redis_pool_size = 10           # 仅 feature="redis" 时有效
redis_timeout_secs = 5         # Redis 连接超时(秒)
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `default_ttl_secs` | `u64` | `3600` |
| `key_prefix` | `String` | `""` |
| `memory_max_capacity` | `usize` | `10000` |
| `redis_url` | `Option<String>` | `None` |
| `redis_pool_size` | `u32` | `10` |
| `redis_timeout_secs` | `u64` | `5` |

> 后端选择**非运行时配置开关**：`MemoryCache` 始终编译注册；`RedisCache` 仅在 `features = ["redis"]` 时编译注册。若同时启用 redis，注入 `Arc<dyn CacheService>` 会取注册顺序中第一个实现（当前为 `MemoryCache`）。要用 Redis 后端需按具体类型 `Arc<RedisCache>` 注入，或调整注册顺序。

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `CacheConfig` | `conf`, `init`, `init_sort = i32::MIN + 1` | 配置载体 |
| `MemoryCache` | `as_trait = dyn CacheService` | 内存实现（始终可用） |
| `RedisCache`（feature=redis） | `as_trait = dyn CacheService` | Redis 实现（延迟连接） |

## 使用方式

```rust
use std::time::Duration;
use std::sync::Arc;
use tx_di_core::{BuildContext, inject_trait_from_store};
use tx_di_cache::CacheService;

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    let app = BuildContext::new::<std::path::PathBuf>(Some("configs/cache_config.toml"))
        .build()?;
    let store = &app.store;

    // 注入缓存句柄（返回第一个 dyn CacheService 实现）
    let cache: Arc<dyn CacheService> = inject_trait_from_store::<dyn CacheService>(store);

    cache.set("key", b"value", Some(Duration::from_secs(60))).await?;
    let v: Option<Vec<u8>> = cache.get("key").await?;

    // Hash / List / Set / SortedSet
    cache.hset("user:1", "name", b"alice").await?;
    cache.lpush("queue", &[b"job1"]).await?;
    cache.sadd("tags", &[b"rust"]).await?;
    cache.zadd("scores", &[(b"player1", 10.0)]).await?;

    // 批量 / 清理
    cache.set_many(&[("k1", &b"v1"[..]), ("k2", &b"v2"[..])], None).await?;
    let vals = cache.get_many(&["k1", "k2"]).await?;
    let matched = cache.keys("user:*").await?;
    cache.clear_prefix("user:").await?;
    Ok(())
}
```

在业务组件中只需在字段声明 `Arc<dyn CacheService>`，DI 自动解析。

## 注意事项

1. **键前缀**：所有读写内部对 key 应用 `prefix + key`；`keys()`/`clear_prefix()` 返回/匹配时会剥离前缀；`clear()` 清空整个后端（Redis 端执行 `FLUSHDB`，有跨应用清库风险）。
2. **TTL 惰性过期（内存）**：仅在读路径检查过期；`set` 时不主动清理，也无后台定时清理。写入时 `ttl=None` 表示永不过期（`default_ttl_secs` 不会自动套用）。
3. **容量限制**：`memory_max_capacity > 0` 且已满时，`set/hset/lpush/...` 返回 `ResourceExhausted`；`0` 表示无限制。
4. **Redis 连接延迟初始化**：首次操作时才建连接；`redis_url` 为 `None` 时回退 `redis://127.0.0.1:6379`。
5. `CacheConfig.init_sort = i32::MIN + 1`，保证配置先于缓存实现加载。
6. 类型不匹配（如用 Hash 接口访问 String 值）返回 `TypeError`。
