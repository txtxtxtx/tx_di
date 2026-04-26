# tx_di

基于 `proc_macro` + `linkme` 的 Rust 依赖注入框架。**编译期收集元数据，运行期自动拓扑排序并注入**，零反射、零运行时扫描开销。

[![Crates.io](https://img.shields.io/crates/v/tx-di-core.svg)](https://crates.io/crates/tx-di-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

- **GitHub**: https://github.com/txtxtxtx/tx_di.git  
- **Gitee**: https://gitee.com/tian_xiong/tx_di.git

---

## 目录

- [特性一览](#特性一览)
- [快速上手](#快速上手)
- [核心概念](#核心概念)
  - [作用域 Scope](#作用域-scope)
  - [字段声明方式](#字段声明方式)
  - [配置组件 conf](#配置组件-conf)
  - [自定义初始化 CompInit](#自定义初始化-compinit)
- [BuildContext API](#buildcontext-api)
- [App（固化上下文）](#app固化上下文)
- [配置文件加载](#配置文件加载)
- [内置插件](#内置插件)
  - [tx_di_log — 日志插件](#tx_di_log--日志插件)
  - [tx_di_axum — Web 插件](#tx_di_axum--web-插件)
- [架构原理](#架构原理)
- [约束与注意事项](#约束与注意事项)
- [测试](#测试)

---

## 特性一览

| 特性 | 说明 |
|------|------|
| **零反射** | 依赖关系在编译期由宏生成，链接器通过 `linkme` 收集 |
| **Singleton / Prototype** | 两种作用域，scope 标记在**被注入者**上，消费者无感知 |
| **自动拓扑排序** | Kahn 算法，运行时自动解析构建顺序，循环依赖立即报错 |
| **自定义值注入** | `#[tx_cst(expr)]` 支持任意 Rust 表达式，不进入依赖图 |
| **TOML 配置加载** | `#[tx_comp(conf)]` 自动从配置文件反序列化组件 |
| **生命周期回调** | `CompInit` trait 支持同步 `init` / 异步 `async_init` |
| **并发安全** | 使用 `DashMap` 存储实例，`Arc<T>` 共享，线程安全 |
| **插件化** | `tx_di_log`（日志）、`tx_di_axum`（Web）开箱即用 |

---

## 快速上手

### 1. 添加依赖

```toml
[dependencies]
tx-di-core = "0.1.6"
linkme = "0.3"
tokio = { version = "1", features = ["full"] }
```

### 2. 定义并注入组件

```rust
use std::sync::Arc;
use tx_di_core::{tx_comp, tx_cst, BuildContext};

// ── 无依赖单例 ────────────────────────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp]                          // 默认 Singleton
pub struct DbPool;

// ── 带自定义值的单例 ──────────────────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp]
pub struct AppConfig {
    #[tx_cst("my-app".to_string())]
    pub app_name: String,

    #[tx_cst(8080u16)]
    pub port: u16,
}

// ── 原型组件（每次注入创建新实例）────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp(scope = Prototype)]
pub struct RequestLogger {
    #[tx_cst("[REQ]".to_string())]
    pub prefix: String,
}

// ── 依赖其他组件的服务 ────────────────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp]
pub struct UserService {
    pub db: Arc<DbPool>,            // 自动注入 DbPool 单例
    pub config: Arc<AppConfig>,     // 自动注入 AppConfig 单例
}

// ── main ──────────────────────────────────────────────────────────
#[tokio::main]
async fn main() {
    // 创建上下文，自动扫描所有 #[tx_comp] 组件并按拓扑顺序构建
    let mut ctx = BuildContext::new::<std::path::PathBuf>(None);

    let svc = ctx.inject::<UserService>();
    println!("app_name: {}", svc.config.app_name);
    println!("port:     {}", svc.config.port);
}
```

---

## 核心概念

### 作用域 Scope

| 作用域 | 宏写法 | 行为 |
|--------|--------|------|
| **Singleton**（默认） | `#[tx_comp]` | 全局唯一，首次注入时构建并缓存 `Arc<T>` |
| **Prototype** | `#[tx_comp(scope = Prototype)]` 或 `#[tx_comp(scope)]` | 每次 `inject()` 调用工厂创建全新实例 |

> **关键原则**：scope 标记在**被注入者**上，消费者只需写 `Arc<T>`，框架自动处理。

```rust
// ✅ Prototype 标记在 RequestLogger 自己身上
#[tx_comp(scope = Prototype)]
pub struct RequestLogger { ... }

// 消费者无需关心 scope
#[tx_comp]
pub struct AppServer {
    pub logger: Arc<RequestLogger>,  // 每次注入 AppServer 时创建新的 logger 实例
}
```

---

### 字段声明方式

| 写法 | 语义 |
|------|------|
| `field: Arc<T>` | 从 DI 容器注入，框架根据 `T` 的 scope 自动处理 |
| `field: T`（其他类型）| 非 `Arc` 包裹时，宏仍会尝试 `ctx.inject::<T>()` |
| `#[tx_cst(expr)]` | **不走 DI**，直接用表达式赋值，不计入依赖图 |
| `Option<T>` 字段 | 自动设为 `None`，不参与依赖注入 |
| `#[tx_cst(skip)]` | 跳过注入，使用 `Default::default()` |

```rust
#[tx_comp]
pub struct MyService {
    // ① DI 注入
    pub db: Arc<DbPool>,

    // ② 自定义值：任意表达式，不进依赖图
    #[tx_cst("0.0.0.0:8080".to_string())]
    pub bind_addr: String,

    // ③ 调用函数
    #[tx_cst(default_headers())]
    pub headers: HashMap<String, String>,

    // ④ 集合
    #[tx_cst(Vec::new())]
    pub items: Vec<String>,

    // ⑤ 读取环境变量
    #[tx_cst(std::env::var("SECRET").unwrap_or_default())]
    pub secret: String,
}
```

> `#[tx_cst(expr)]` 字段**不会**被加入 `DEP_IDS`，不影响拓扑排序，对应类型也无需在 ctx 中注册。

---

### 配置组件 conf

用 `#[tx_comp(conf)]` 标记的组件会自动从 TOML 配置文件中加载配置，无需手动解析。

```toml
# configs/app.toml
[app_config]
app_name = "production-app"
port = 9090
```

```rust
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
#[tx_comp(conf)]        // 自动从 [app_config] 段加载（结构体名转蛇形：AppConfig → app_config）
pub struct AppConfig {
    #[serde(default = "default_name")]
    pub app_name: String,

    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_name() -> String { "my-app".to_string() }
fn default_port() -> u16 { 8080 }
```

**自定义配置键**：

```rust
#[tx_comp(conf = "server")]   // 从 TOML 的 [server] 段读取，而非默认的 [my_component]
pub struct MyComponent { ... }
```

**加载配置文件**：

```rust
// 指定配置文件路径
let ctx = BuildContext::new(Some("configs/app.toml"));

// 不使用配置文件（所有配置组件使用 serde 默认值）
let ctx = BuildContext::new::<std::path::PathBuf>(None);
```

---

### 自定义初始化 CompInit

在所有依赖构建完成后，可以通过 `CompInit` trait 执行同步或异步初始化逻辑。

```rust
use tx_di_core::{tx_comp, CompInit, App, BoxFuture, RIE};
use std::sync::Arc;

#[derive(Debug)]
#[tx_comp(init)]        // init flag：告知宏"我自己写 CompInit 实现"
pub struct AppServer {
    pub user_svc: Arc<UserService>,
    pub bind_addr: String,          // 假设通过 #[tx_cst] 注入
}

impl CompInit for AppServer {
    /// 同步初始化：App 构建后按 init_sort 顺序调用
    fn init(ctx: Arc<App>) -> RIE<()> {
        let server = ctx.inject::<AppServer>();
        println!("AppServer 同步初始化，bind_addr: {}", server.bind_addr);
        Ok(())
    }

    /// 异步初始化：所有 init() 完成后并行执行
    /// 注意：返回的 Future 必须是 'static，不能持有 ctx 引用
    fn async_init(ctx: Arc<App>) -> BoxFuture<'static, RIE<()>> {
        let addr = ctx.inject::<AppServer>().bind_addr.clone(); // 先提取
        Box::pin(async move {
            println!("AppServer 异步初始化，bind_addr: {}", addr);
            // 可执行：数据库连接、加载远程配置、预热缓存等
            Ok(())
        })
    }

    /// 控制初始化顺序：值越小越先初始化，默认 10000
    fn init_sort() -> i32 {
        1000
    }
}
```

**组件内部初始化（`inner_init`）**：在组件构建时（`build()` 内部）立即调用，早于 `init()`：

```rust
impl CompInit for MyConfig {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        // 在 build() 完成字段赋值后立即执行
        self.computed_value = compute(&self.raw_value);
        Ok(())
    }
}
```

---

## BuildContext API

`BuildContext` 是构建阶段的主要入口，负责组件注册、注入和生命周期管理。

```rust
use std::path::PathBuf;
use tx_di_core::BuildContext;

// 创建上下文（自动扫描 + 拓扑排序 + 注册所有 #[tx_comp] 组件）
let mut ctx = BuildContext::new(Some("configs/app.toml"));
// 或不使用配置文件：
let mut ctx = BuildContext::new::<PathBuf>(None);

// ── 注入 ──────────────────────────────────────────────────────────

// inject: 根据 scope 自动处理（Singleton 返回缓存 Arc，Prototype 创建新实例）
let db: Arc<DbPool> = ctx.inject::<DbPool>();
let logger: Arc<RequestLogger> = ctx.inject::<RequestLogger>();  // 每次新实例

// get: 仅用于 Singleton，不需要 &mut self（适合在 Arc<BuildContext> 场景）
let db: Arc<DbPool> = ctx.get::<DbPool>();

// get_singleton: 无需 ComponentDescriptor 约束，直接从 store 读取
let db: Arc<DbPool> = ctx.get_singleton::<DbPool>();

// try_get_singleton: 安全版本，未找到返回 None 而非 panic
let db: Option<Arc<DbPool>> = ctx.try_get_singleton::<DbPool>();

// take: 移交所有权（仅 Singleton），从 ctx 中移除该组件
let server: AppServer = ctx.take::<AppServer>().expect("未找到 AppServer");

// ── 调试 ──────────────────────────────────────────────────────────

BuildContext::debug_registry();     // 打印所有注册组件及依赖关系（tracing debug）
println!("组件数: {}", ctx.len());
println!("为空: {}", ctx.is_empty());

// ── 构建固化上下文 ──────────────────────────────────────────────

// build(): 执行初始化回调，返回不可变的 App 实例
let app: App = ctx.build().await?;

// build_and_run(): build + App::run() 的快捷方式
ctx.build_and_run().await?;
```

---

## App（固化上下文）

`App` 是 `BuildContext::build()` 之后的"固化"状态，只支持 Singleton 注入（Prototype 在 App 阶段不可用），适合在 `Arc<App>` 共享场景（如 axum handler）中使用。

```rust
use std::sync::Arc;
use tx_di_core::{BuildContext, App};

let mut ctx = BuildContext::new::<std::path::PathBuf>(None);
let app = Arc::new(ctx.build().await?);

// 在 axum handler 或多线程场景中共享 Arc<App>
let db = app.inject::<DbPool>();                      // panic if not found
let db = app.try_inject::<DbPool>();                  // Option<Arc<T>>，安全版本

println!("组件数: {}", app.len());
```

---

## 配置文件加载

### 完整配置示例

```toml
# configs/app.toml

[app_config]
app_name = "my-app"
port = 8080

[log_config]
level = "info"
dir = "./logs"
console_output = true
time_format = "local"
retention_days = 90
prefix = "my-app"

[web_config]
host = "0.0.0.0"
port = 8080
enable_cors = true
max_body_size = 10485760    # 10MB
timeout_secs = 30

[web_config.spa_apps]
"/app" = "./static/dist"
```

### 访问全局配置对象 AppAllConfig

框架自动创建 `AppAllConfig` 单例，可以直接访问原始 TOML 数据：

```rust
let global_cfg = ctx.inject::<tx_di_core::AppAllConfig>();

// 读取指定键（点号分隔多层路径）
let app_name: Option<String> = global_cfg.get("app_config.app_name");
let port: Option<u16>        = global_cfg.get("app_config.port");

// 带默认值读取
let retention = global_cfg.get_or_default("log_config.retention_days", 90u64);

// 获取 toml::Value
let raw_value = global_cfg.get_value("web_config.host");
```

---

## 内置插件

插件通过 `linkme` 在编译期自动注册组件，使用时**必须在代码中 `use` 导入**，否则链接器会优化掉 crate，导致组件无法注册。

### tx_di_log — 日志插件

基于 `tracing` + `tracing-subscriber`，支持按天滚动文件日志、控制台彩色输出、模块级别过滤。

**添加依赖**：

```toml
[dependencies]
tx_di_log = "0.1.0"
```

**使用**：

```rust
use tx_di_core::BuildContext;
use tx_di_log;  // ← 必须导入，触发 LogConfig 组件注册

#[tokio::main]
async fn main() {
    // 创建上下文后日志立即生效（LogConfig 在构建阶段初始化）
    let ctx = BuildContext::new(Some("configs/app.toml"));

    tracing::info!("应用启动");
    tracing::debug!("调试信息");
}
```

**配置**：

```toml
[log_config]
level = "info"                    # off / error / warn / info / debug / trace
dir = "./logs"                    # 日志文件目录
console_output = true             # 是否输出到控制台
time_format = "local"             # utc / local
retention_days = 90               # 日志文件保留天数
prefix = "my-app"                 # 文件名前缀：my-app-2026-04-26.log

# 可选：模块级别覆盖
[log_config.modules]
"my_app::db" = "debug"
"noisy_lib"  = "warn"
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `level` | String | `"info"` | 全局日志级别 |
| `dir` | String | `"./logs"` | 日志文件目录 |
| `console_output` | bool | `false` | 是否输出到控制台 |
| `time_format` | String | `"utc"` | 时间格式 |
| `retention_days` | u64 | `90` | 日志保留天数 |
| `prefix` | String | `"tx_di"` | 日志文件名前缀 |

---

### tx_di_axum — Web 插件

基于 `axum` + `tokio`，提供异步 HTTP 服务器，支持 CORS、静态文件、SPA 应用、中间件链路配置。

**添加依赖**：

```toml
[dependencies]
tx_di_axum = "0.1.0"
```

**使用**：

```rust
use tx_di_core::BuildContext;
use tx_di_axum;   // ← 必须导入，触发 WebConfig 组件注册
use tx_di_log;    // 推荐配合日志插件

#[tokio::main]
async fn main() {
    let mut ctx = BuildContext::new(Some("configs/app.toml"));

    // build_and_run() 会调用 WebConfig 的 async_init，在后台启动 Web 服务器
    ctx.build_and_run().await.expect("启动失败");

    // Web 服务器在后台运行，保持主线程不退出
    tokio::signal::ctrl_c().await.ok();
}
```

**配置**：

```toml
[web_config]
host = "0.0.0.0"
port = 8080
enable_cors = true
max_body_size = 10485760    # 10MB
timeout_secs = 30

# 中间件列表：[优先级, 名称]（数字越大越靠外层，最先接收请求）
layers = [
    [10,    "api_log"],       # 请求/响应日志
    [100,   "compression"],   # 响应压缩
    [10000, "cors"],          # CORS（最外层）
]

# SPA 应用：URL 前缀 → 静态文件目录
[web_config.spa_apps]
"/app" = "./static/dist"
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `host` | String | `"127.0.0.1"` | 监听地址，支持 IPv4 / IPv6 |
| `port` | u16 | `8080` | 监听端口 |
| `enable_cors` | bool | `false` | 是否启用 CORS |
| `max_body_size` | usize | `10485760` | 最大请求体（字节） |
| `timeout_secs` | u64 | `30` | 请求超时时间（秒） |
| `spa_apps` | Map | `{}` | SPA 应用路由映射 |

**内置端点**：

| 路径 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查，返回 `OK` |

**IPv6 支持**：

```toml
[web_config]
host = "::1"    # IPv6 localhost
# host = "::"  # 监听所有 IPv6 接口
port = 8080
```

框架自动处理 IPv6 地址的方括号格式（`[::1]:8080`）。

---

## 架构原理

```
用户代码（定义组件）
  #[tx_comp]                 struct DbPool {}
  #[tx_comp(scope=Prototype)] struct Logger { #[tx_cst(...)] prefix: String }
  #[tx_comp]                 struct AppServer { db: Arc<DbPool>, log: Arc<Logger> }
         │
         │  proc_macro 展开（编译期）
         ▼
tx-di-macros
  1. 解析 scope 参数   →  Singleton / Prototype
  2. 解析字段
       Arc<T> 或 T    →  FieldKind::Inject  → 加入 DEP_IDS
       Option<T>      →  FieldKind::Optional → 注入 None
       #[tx_cst(expr)] →  FieldKind::Custom  → 不进依赖图
       #[tx_cst(skip)] →  FieldKind::Skip    → Default::default()
  3. 生成 ComponentDescriptor impl（DEP_IDS + SCOPE + build()）
  4. 生成 CompInit 默认空实现（用户标记 init 则跳过）
  5. 注册 linkme distributed_slice 条目（ComponentMeta）
         │
         │  链接器合并 link section（运行前）
         ▼
tx-di-core（运行时）
  COMPONENT_REGISTRY        全局静态组件元数据切片（linkme 收集）
  BuildContext::new()
    ├─ 创建 AppAllConfig 单例（加载 TOML）
    └─ auto_register_all()
         ├─ 从 COMPONENT_REGISTRY 收集所有 ComponentMeta
         ├─ topo_sort()（Kahn 算法，O(V+E)）检测循环依赖
         └─ 按顺序调用 factory_fn，Singleton 立即构建并缓存
  BuildContext::build()
    ├─ 按 init_sort 顺序调用 init()（同步）
    ├─ 并行调用所有 async_init()（异步）
    └─ 转移 store → App
```

---

## 约束与注意事项

| 约束 | 原因 |
|------|------|
| 组件需 `T: Send + Sync + 'static` | 存入 `Arc<dyn Any + Send + Sync>`，支持多线程 |
| 配置组件需 `Deserialize + Default` | 配置键不存在时使用 serde 默认值 |
| `take()` 仅用于 Singleton | Prototype 无缓存，无法取出 |
| 插件 crate 必须 `use` 导入 | `linkme` 依赖链接器，未引用的 crate 会被优化掉 |
| 避免循环依赖 | 拓扑排序时检测，存在则 panic 并列出循环节点 |
| `async_init` 返回 `'static` Future | 不能在 async 块中直接借用 `ctx`，需先提取数据 |
| `inner_init` 在 `build()` 内调用 | 此时 `ctx` 仍处于构建阶段，不要 `inject` 尚未构建的组件 |

---

## 测试

```bash
# 运行所有测试
cargo test

# 只运行 di-example 的测试
cargo test -p di-example

# 显示输出
cargo test -- --nocapture
```

### 测试覆盖范围（30+ 个用例）

| 分类 | 用例数 | 涵盖内容 |
|------|--------|---------|
| 单例行为 | 3 | 共享验证、多次注入、Arc 引用计数 |
| 原型行为 | 3 | 独立实例、每次新建、自定义值 |
| 自定义值注入 | 3 | HashMap / String / 函数调用 |
| 依赖链 | 2 | 多级依赖、服务功能验证 |
| 注册表 | 2 | 组件数量、scope 验证 |
| BuildContext API | 3 | len / is_empty / take |
| 边界情况 | 4 | 线程安全、状态隔离、无依赖、多依赖 |
| 调试功能 | 1 | `debug_registry` 不 panic |
| 配置文件加载 | 9 | 从文件加载、默认值、嵌套键、类型转换、单例验证 |

---

## 许可证

MIT — 详见 [LICENSE](LICENSE)
