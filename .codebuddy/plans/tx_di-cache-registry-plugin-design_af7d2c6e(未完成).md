---
name: tx_di-cache-registry-plugin-design
overview: 设计 tx-di 框架下的 Redis/内存缓存双引擎插件，以及支持配置热更新、双协议(gRPC+HTTP)注册、可启停的注册中心插件
todos:
  - id: fn_cache-feature-cargo
    content: 创建 tx_di_cache 的 Cargo.toml 和 features 配置（default=memory, redis 可选）
    status: completed
  - id: fn-cache-trait-impl
    content: 实现 CacheService trait 定义 + CacheConfig 配置组件 + MemoryCache 默认实现
    status: completed
    dependencies:
      - fn_cache-feature-cargo
  - id: fn-cache-redis
    content: 实现 RedisCache（feature-gated） + 单元测试
    status: completed
    dependencies:
      - fn-cache-trait-impl
  - id: fn-registry-core-struct
    content: 创建 tx_di_registry 核心数据结构：model.rs（ServiceEndpoint/ServiceInstance）+ traits.rs（ServiceRegistry/ConfigCenter）+ dynamic_config.rs（动态配置容器）+ endpoints.rs（端点注册表）
    status: completed
  - id: fn-registry-config-plugin
    content: 实现 RegistryConfig 配置组件（含 enabled 开关）+ RegistryPlugin Component（启停/注册/心跳/注销生命周期）
    status: completed
    dependencies:
      - fn-registry-core-struct
  - id: fn-registry-nacos
    content: 实现 NacosServiceRegistry + NacosConfigCenter + ConfigWatcher（配置热更新机制）
    status: completed
    dependencies:
      - fn-registry-config-plugin
  - id: fn-registry-adapt-api
    content: 在 tx_di_axum 和 admin_api/plugin.rs 中适配 EndpointProvider 端点注册
    status: pending
    dependencies:
      - fn-registry-nacos
  - id: fn-test-integration
    content: 使用 [skill:rust-ddd-test-generator] 生成完整测试套件，执行全部编译检查和测试
    status: completed
    dependencies:
      - fn-cache-redis
      - fn-registry-nacos
---

## 需求分析

为 tx-di 项目添加微服务基础设施支持，具体包括：

### 1. 缓存插件（tx_di_cache）

- 提供 `CacheService` trait 抽象层，业务代码只依赖 trait 不感知实现
- **MemoryCache** 作为默认实现（始终可用），使用 `DashMap` 做并发 KV 存储
- **RedisCache** 作为 feature-gated 增强实现（`feature = "redis"`），共享同一个 `CacheService` trait
- 不需要 MockCache，内存缓存即为开发和测试的标准实现

### 2. 注册/配置中心插件（tx_di_registry）

- 需要精心设计，核心需求如下：

**配置热更新**：注册中心（如 Nacos）上的配置变更后，应用能**运行时动态感知并更新本地配置**，不需要重启进程。这意味着需要一个新的"可更新配置"容器机制，与现有 `#[component(conf)]` 静态配置并存。

**可启停**：注册中心功能支持 `enabled` 开关，配置中关闭时，整个注册/发现/配置监听链路都不启动。启动后也能通过配置变更动态关闭。

**双协议注册**：同一服务实例同时提供 HTTP（axum）和 gRPC（tonic）端点，注册中心需要支持同时注册两种协议的地址，形成一个服务实例下包含多个端点的结构。

**生态选型**：推荐 r-nacos（Rust 原生 Nacos 实现）作为注册+配置中心的基础设施，使用 `nacos_rust_client` crate 作为客户端。

## 技术方案

### 技术栈

| 组件 | 选型 | 理由 |
| --- | --- | --- |
| 内存缓存 | `DashMap`（已有依赖） | 零额外依赖，无锁高并发 |
| Redis 客户端 | `redis = "0.28"` | Rust 生态最成熟 Redis 客户端，tokio 原生 |
| 注册中心客户端 | `nacos_rust_client` | 纯 Rust，支持 Nacos 1.x/2.x 协议 |
| 注册中心服务端 | **r-nacos** | Rust 原生 Nacos 实现，单机/集群部署 |
| 配置变更传播 | `tokio::sync::watch` | 一对多广播，最新值缓存，零漏接 |
| 动态配置容器 | `Arc<RwLock<T>>` | 运行时原子更新配置 |


### 一、缓存插件详细设计

#### 目录结构

```
plugins/tx_di_cache/
├── Cargo.toml
│   [features]
│   default = ["memory"]
│   memory = []
│   redis = ["dep:redis"]
│
├── src/
│   ├── lib.rs           # 公共导出
│   ├── config.rs        # CacheConfig 配置组件
│   ├── service.rs       # CacheService trait 定义
│   ├── memory.rs        # MemoryCache 实现（默认）
│   └── redis.rs         # RedisCache 实现（feature = "redis"）
```

#### 核心设计

基础值类型统一为 `Vec<u8>`（二进制安全，可存序列化 protobuf/JSON/字符串）。CacheService trait 按 Redis 数据类型分层组织：

```rust
// service.rs - CacheService trait 抽象
#[async_trait]
pub trait CacheService: Send + Sync + 'static {

    // ── String (KV) 操作 ──
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()>;
    async fn set_string(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<()> {
        self.set(key, value.as_bytes(), ttl).await
    }
    async fn del(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn ttl(&self, key: &str) -> Result<Option<Duration>>;
    async fn expire(&self, key: &str, ttl: Duration) -> Result<()>;

    // ── Hash 操作 ──
    async fn hget(&self, key: &str, field: &str) -> Result<Option<Vec<u8>>>;
    async fn hset(&self, key: &str, field: &str, value: &[u8]) -> Result<()>;
    async fn hdel(&self, key: &str, field: &str) -> Result<()>;
    async fn hgetall(&self, key: &str) -> Result<HashMap<String, Vec<u8>>>;
    async fn hkeys(&self, key: &str) -> Result<Vec<String>>;
    async fn hlen(&self, key: &str) -> Result<usize>;

    // ── List 操作 ──
    async fn lpush(&self, key: &str, values: &[&[u8]]) -> Result<usize>;
    async fn rpush(&self, key: &str, values: &[&[u8]]) -> Result<usize>;
    async fn lpop(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn rpop(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn lrange(&self, key: &str, start: i64, stop: i64) -> Result<Vec<Vec<u8>>>;
    async fn llen(&self, key: &str) -> Result<usize>;

    // ── Set 操作 ──
    async fn sadd(&self, key: &str, members: &[&[u8]]) -> Result<usize>;
    async fn srem(&self, key: &str, members: &[&[u8]]) -> Result<usize>;
    async fn smembers(&self, key: &str) -> Result<Vec<Vec<u8>>>;
    async fn sismember(&self, key: &str, member: &[u8]) -> Result<bool>;
    async fn scard(&self, key: &str) -> Result<usize>;

    // ── Sorted Set 操作 ──
    async fn zadd(&self, key: &str, members: &[(&[u8], f64)]) -> Result<usize>;
    async fn zrem(&self, key: &str, members: &[&[u8]]) -> Result<usize>;
    async fn zrange(&self, key: &str, start: i64, stop: i64, with_scores: bool) -> Result<Vec<Vec<u8>>>;
    async fn zrangebyscore(&self, key: &str, min: f64, max: f64) -> Result<Vec<Vec<u8>>>;
    async fn zscore(&self, key: &str, member: &[u8]) -> Result<Option<f64>>;
    async fn zcard(&self, key: &str) -> Result<usize>;

    // ── 批量操作 ──
    async fn get_many(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>>;
    async fn set_many(&self, pairs: &[(&str, &[u8])], ttl: Option<Duration>) -> Result<()>;
    async fn del_many(&self, keys: &[&str]) -> Result<usize>;

    // ── 清理操作 ──
    async fn keys(&self, pattern: &str) -> Result<Vec<String>>;
    async fn clear(&self) -> Result<()>;
    async fn clear_prefix(&self, prefix: &str) -> Result<usize>;
}
```

**MemoryCache 内部数据结构**（用枚举区分不同类型）：

```rust
use std::collections::{HashMap, HashSet, BTreeSet, VecDeque};

enum CacheValue {
    StringValue {
        data: Vec<u8>,
        expires_at: Option<Instant>,
    },
    HashValue {
        fields: HashMap<String, Vec<u8>>,
        expires_at: Option<Instant>,
    },
    ListValue {
        deque: VecDeque<Vec<u8>>,
        expires_at: Option<Instant>,
    },
    SetValue {
        members: HashSet<Vec<u8>>,
        expires_at: Option<Instant>,
    },
    SortedSetValue {
        members: BTreeSet<ZMember>,
        expires_at: Option<Instant>,
    },
}

struct ZMember {
    member: Vec<u8>,
    score: f64,
}
```

**RedisCache 实现**（feature-gated）：

```rust
#[cfg(feature = "redis")]
#[derive(Component)]
#[component(conf = "cache", as_trait = dyn CacheService)]
pub struct RedisCache {
    config: Arc<CacheConfig>,
    client: Arc<redis::aio::ConnectionManager>,
}
// 所有方法直接委托给 redis::cmd，天然支持各数据类型
```

**关键设计决策**：

1. 使用 `Vec<u8>` 作为值类型——二进制安全，可存序列化 protobuf/JSON/字符串/图片等
2. Redis 原生的 5 种数据结构（String/Hash/List/Set/SortedSet）全部映射到 trait 方法
3. MemoryCache 内部用枚举 + `DashMap<String, CacheValue>` 实现同样语义
4. 通过 `#[component(as_trait = dyn CacheService)]` 注册到 DI，用户无感知切换

#### 使用示例

```rust
// 用户在 Component 中依赖缓存
#[derive(Component)]
pub struct MyService;

fn init(this: &mut MyService, store: &Store) -> RIE<()> {
    let cache: Arc<dyn CacheService> = inject_trait_from_store::<dyn CacheService>(store);
    // 使用 cache.get/set/del，不关心底层实现
    Ok(())
}
```

### 二、注册/配置中心插件详细设计

这是项目的核心设计。整体架构如下：

```
plugins/tx_di_registry/
├── Cargo.toml
│   [features]
│   default = ["nacos"]
│   nacos = ["dep:nacos_rust_client"]
│   mock = []
│
├── src/
│   ├── lib.rs                # 重新导出
│   ├── config.rs             # RegistryConfig（enabled 开关）
│   ├── model.rs              # ServiceInstance, ServiceEndpoint
│   ├── traits.rs             # ServiceRegistry, ConfigCenter, EndpointProvider
│   ├── plugin.rs             # RegistryPlugin Component
│   ├── endpoints.rs          # 静态端点注册表
│   ├── config_watcher.rs     # 配置变更传播
│   ├── dynamic_config.rs     # DynamicConfig<T> 热更新容器
│   └── nacos/                # Nacos 实现（feature="nacos"）
│       ├── mod.rs
│       ├── registry_impl.rs
│       └── config_impl.rs
```

#### 2.1 数据模型（model.rs）

```rust
/// 服务协议类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol { Http, Grpc }

/// 服务端点（一个协议一个地址）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub protocol: Protocol,
    pub ip: String,
    pub port: u16,
    pub metadata: HashMap<String, String>,
}

/// 服务实例（包含多个协议的端点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub service_name: String,
    pub instance_id: String,
    pub endpoints: Vec<ServiceEndpoint>,
    pub healthy: bool,
    pub metadata: HashMap<String, String>,
}
```

#### 2.2 核心 Trait 定义（traits.rs）

```rust
/// 服务注册/发现 trait
#[async_trait]
pub trait ServiceRegistry: Send + Sync + 'static {
    /// 注册服务实例（包含全部端点）
    async fn register(&self, instance: &ServiceInstance) -> Result<()>;
    /// 更新实例（在线修改端点/元数据）
    async fn update(&self, instance: &ServiceInstance) -> Result<()>;
    /// 注销服务实例
    async fn deregister(&self, instance_id: &str) -> Result<()>;
    /// 发现服务（按名称获取实例列表）
    async fn discover(&self, service_name: &str) -> Result<Vec<ServiceInstance>>;
    /// 监听服务变更
    async fn subscribe(&self, service_name: &str, callback: Box<dyn Fn(Vec<ServiceInstance>) + Send + Sync>);
}

/// 配置中心 trait
#[async_trait]
pub trait ConfigCenter: Send + Sync + 'static {
    /// 获取配置（返回 DynamicConfig 以支持热更新）
    async fn get_config<T: DeserializeOwned + Send>(&self, data_id: &str, group: &str) -> Result<Option<DynamicConfig<T>>>;
    /// 发布/更新配置
    async fn publish_config(&self, data_id: &str, group: &str, content: &str) -> Result<()>;
    /// 删除配置
    async fn remove_config(&self, data_id: &str, group: &str) -> Result<()>;
    /// 订阅配置变更（返回 watch Receiver）
    fn subscribe(&self) -> watch::Receiver<ConfigChangeEvent>;
}

/// 端点提供者 trait — HTTP/gRPC 插件实现此 trait
pub trait EndpointProvider: Send + Sync {
    /// 返回当前服务提供的所有端点
    fn get_endpoints(&self) -> Vec<ServiceEndpoint>;
}
```

#### 2.3 配置热更新机制（dynamic_config.rs）

这是最核心的设计。现有 `#[component(conf)]` 是构建时一次性加载不可变。新的 `DynamicConfig` 允许运行时更新：

```rust
/// 动态配置容器 — 原子更新 + 版本号追踪
pub struct DynamicConfig<T> {
    inner: Arc<RwLock<T>>,
    key: String,
    version: Arc<AtomicU64>,
    /// 变更事件发送端
    tx: watch::Sender<ConfigChangeEvent>,
}

impl<T: Clone + Send + Sync + 'static> DynamicConfig<T> {
    /// 获取当前配置快照
    pub fn get(&self) -> T {
        self.inner.read().unwrap().clone()
    }

    /// 更新配置（原子操作，触发通知）
    pub fn update(&self, new_val: T) {
        let mut w = self.inner.write().unwrap();
        *w = new_val;
        let ver = self.version.fetch_add(1, Ordering::Release) + 1;
        let _ = self.tx.send(ConfigChangeEvent {
            key: self.key.clone(),
            version: ver,
        });
    }

    /// 获取版本号（用于判断是否变更）
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// 订阅变更事件
    pub fn subscribe(&self) -> watch::Receiver<ConfigChangeEvent> {
        self.tx.subscribe()
    }
}
```

**配置热更新数据流**：

```
RegistryPlugin::async_run()
  │ 启动 ConfigWatcher
  ▼
┌─────────────────────────────┐
│ NacosConfigCenter           │
│ (长轮询监听 Nacos 配置变化)  │
│ for each (data_id, group):  │
│   loop: long_polling()      │
│     if changed:             │
│       new_val = fetch()     │
│       DynamicConfig.update  │
└───────────┬─────────────────┘
            │ 配置变更事件
            ▼
┌─────────────────────────────┐
│ ConfigWatcher               │
│ 收到 ConfigChangeEvent:     │
│ 1. 反序列化新值为 T         │
│ 2. 校验有效性              │
│ 3. DynamicConfig.update()  │
│ 4. 记录变更日志            │
└───────────┬─────────────────┘
            │ watch::Sender 广播
            ▼
┌──────────────────────┐  ┌──────────────────────┐
│ 组件A                │  │ 组件B                │
│ watch Receiver      │  │ watch Receiver      │
│ on_config_change(): │  │ on_config_change(): │
│  重新加载业务配置    │  │  更新连接池/超时等    │
└──────────────────────┘  └──────────────────────┘
```

**重要设计决策**：并非所有配置都需要热更新。策略如下：

| 配置类别 | 更新方式 | 示例 |
| --- | --- | --- |
| 基础配置（端口、DB连接串） | 构建时 `#[component(conf)]` | `WebConfig.host` |
| 业务配置（功能开关、阈值） | 运行时 `DynamicConfig<T>` | rate_limit, feature_flag |
| 敏感配置（密码、密钥） | 外部 Secret Store | 不落地代码 |


#### 2.4 Enable/Disable 启停机制（config.rs + plugin.rs）

```rust
// config.rs
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf = "registry", init, init_sort = i32::MIN)]
pub struct RegistryConfig {
    /// 主开关：是否启用注册中心功能
    #[serde(default)]
    pub enabled: bool,
    /// Nacos 服务地址
    #[serde(default = "default_nacos_addr")]
    pub nacos_addr: String,
    /// 命名空间
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// 分组
    #[serde(default = "default_group")]
    pub group: String,
    /// 本地服务名
    #[serde(default = "default_service_name")]
    pub service_name: String,
    /// 是否自动注册本地端点
    #[serde(default = "default_true")]
    pub auto_register: bool,
    /// 心跳间隔（秒）
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_secs: u64,
}

// plugin.rs
#[derive(Component)]
#[component(
    conf = "registry",
    app_async_init, app_async_run,
    init_sort = i32::MAX - 50,
)]
pub struct RegistryPlugin {
    pub config: Arc<RegistryConfig>,
    // 只在 enabled=true 时初始化
    #[tx_cst(OnceLock::new())]
    registry: OnceLock<Arc<dyn ServiceRegistry>>,
    #[tx_cst(OnceLock::new())]
    config_center: OnceLock<Arc<dyn ConfigCenter>>,
}

// async_init: 如果 enabled=false，跳过初始化
async fn app_async_init(comp: Arc<RegistryPlugin>, app: Arc<App>) -> RIE<()> {
    if !comp.config.enabled {
        info!("注册中心已禁用（registry.enabled=false）");
        return Ok(());
    }
    // 1. 初始化 Nacos 客户端
    let (registry, config_center) = init_nacos(&comp.config).await?;
    comp.registry.set(registry).ok();
    comp.config_center.set(config_center).ok();

    // 2. 收集本地端点并注册
    if comp.config.auto_register {
        let endpoints = collect_endpoints();
        let instance = ServiceInstance {
            service_name: comp.config.service_name.clone(),
            instance_id: format!("{}-{}", comp.config.service_name, get_local_ip()),
            endpoints,
            healthy: true,
            metadata: HashMap::new(),
        };
        comp.registry().register(&instance).await?;
    }
    Ok(())
}

// async_run: 启动配置监听 + 心跳
async fn app_async_run(comp: Arc<RegistryPlugin>, app: Arc<App>, token: CancellationToken) -> RIE<()> {
    if !comp.config.enabled {
        return Ok(());
    }
    // 启动 ConfigWatcher
    let watcher = ConfigWatcher::new(comp.config_center());
    tokio::spawn(watcher.run(token.clone()));

    // 启动心跳
    let registry = comp.registry();
    let instance_id = "...".to_string();
    tokio::spawn(heartbeat_task(registry, instance_id, token.clone()));
    Ok(())
}

// shutdown: 注销服务
fn shutdown(this: &RegistryPlugin) {
    if let Some(registry) = this.registry.get() {
        // 异步关闭需要特殊处理，或使用 block_on
        info!("正在注销服务实例...");
    }
}
```

#### 2.5 双协议端点注册机制（endpoints.rs）

```rust
/// 静态端点注册表（类似 ROUTER_REGISTRY 模式）
static ENDPOINT_PROVIDERS: LazyLock<Mutex<Vec<Arc<dyn Fn() -> Vec<ServiceEndpoint> + Send + Sync>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// 注册端点提供者闭包
pub fn register_endpoints(provider: Arc<dyn Fn() -> Vec<ServiceEndpoint> + Send + Sync>) {
    if let Ok(mut registry) = ENDPOINT_PROVIDERS.lock() {
        registry.push(provider);
    }
}

/// 收集所有已注册的端点
pub fn collect_endpoints() -> Vec<ServiceEndpoint> {
    let mut all = Vec::new();
    if let Ok(registry) = ENDPOINT_PROVIDERS.lock() {
        for provider in registry.iter() {
            all.extend(provider());
        }
    }
    all
}
```

**HTTP 端点注册**（在 tx_di_axum 的 WebPlugin 中适配）：

```rust
// 在 WebPlugin::app_async_init 中，构建完路由后：
let addr = format!("{}:{}", config.host, config.port);
let http_endpoint = ServiceEndpoint {
    protocol: Protocol::Http,
    ip: config.host.clone(),
    port: config.port,
    metadata: HashMap::new(),
};
register_endpoints(Arc::new(move || vec![http_endpoint.clone()]));
```

**gRPC 端点注册**（在 AdminPlugin 等业务插件中适配）：

```rust
// 在 AdminPlugin::app_async_init 中：
let grpc_endpoint = ServiceEndpoint {
    protocol: Protocol::Grpc,
    ip: "0.0.0.0".into(),
    port: GRPC_PORT,
    metadata: HashMap::new(),
};
register_endpoints(Arc::new(move || vec![grpc_endpoint.clone()]));
```

#### 2.6 现有项目的适配方式

本项目 gRPC 服务器的启动方式与注册中心插件完全兼容：

- 当前 `admin_api/src/plugin.rs` 在 `app_async_init` 中用 `tokio::spawn` 启动 gRPC
- 只需要在该函数中添加 `register_endpoints(...)` 调用即可
- `RegistryPlugin` 的 `init_sort = i32::MAX - 50` 晚于 `AdminPlugin` 的 `i32::MAX - 100`，所以端点已在注册时准备就绪

### 两种插件新增文件清单

```
新增: plugins/tx_di_cache/
  ├── Cargo.toml
  ├── src/lib.rs
  ├── src/config.rs
  ├── src/service.rs
  ├── src/memory.rs
  └── src/redis.rs

新增: plugins/tx_di_registry/
  ├── Cargo.toml
  ├── src/lib.rs
  ├── src/config.rs
  ├── src/model.rs
  ├── src/traits.rs
  ├── src/plugin.rs
  ├── src/endpoints.rs
  ├── src/config_watcher.rs
  ├── src/dynamic_config.rs
  └── src/nacos/
      ├── mod.rs
      ├── registry_impl.rs
      └── config_impl.rs

修改: plugins/tx_di_axum/src/comp.rs
  - 在 WebPlugin app_async_init 中注册 HTTP 端点

修改: Cargo.toml (workspace)
  - 添加 tx_di_cache, tx_di_registry 成员
```

### 向后兼容性

1. 缓存插件完全是新增功能，不影响现有代码
2. 注册中心插件的 `enabled` 默认值为 `false`，用户必须显式启用并配置才生效
3. 现有 gRPC 启动方式（`tokio::spawn`）在不启用注册中心时完全不变
4. `DynamicConfig<T>` 是可选机制，不影响现有的 `#[component(conf)]` 静态配置模式
5. 端点注册使用全局静态注册表，与已有的 `ROUTER_REGISTRY` 模式一致

### 性能考量

1. **内存缓存**：`DashMap` 无锁并发，单次 get 约 50-100ns
2. **配置变更传播**：使用 `watch` channel 实现零开销空闲等待（不轮询），只在配置实际变更时触发
3. **心跳开销**：Nacos 心跳间隔可配置（默认 5s），心跳请求极轻量
4. **配置监听**：Nacos 使用长轮询（30s），无连续请求开销

## Agent Extensions

### SubAgent: code-explorer

- **用途**：用于在实现阶段跨多个目录搜索和读取文件，例如搜索所有使用 `app.inject` 或 `Store` 的调用点以适配缓存注入；搜索所有 gRPC 服务注册点以适配端点注册
- **预期产出**：提高大规模代码浏览效率，确保实现不遗漏调用点

### Skill: rust-ddd-test-generator

- **用途**：用于为两个新插件生成完整的 DDD 测试用例（单元测试 + 集成测试），包括 CacheService trait 的 mock 测试、MemoryCache/RedisCache 的功能测试、RegistryPlugin 的启停测试、ConfigWatcher 的热更新测试
- **预期产出**：生成完整的测试套件，确保插件质量生产就绪