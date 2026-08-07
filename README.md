# tx_di

类型驱动的 Rust 依赖注入（DI）框架 + 插件生态。`Component` trait 声明依赖，`#[derive(Component)]` 自动注册，`linkme` 编译期收集，运行期拓扑排序注入，并提供 Web / 缓存 / 文件 / 任务 / 鉴权 / 国标（GB28181）等插件。

当前版本：`tx-di-core 0.4.0` / `tx-di-macros 0.4.0`

---

## 特性

- **类型驱动**：依赖在 `type Deps` 中声明，编译期可知，注入类型安全
- **零样板**：`#[derive(Component)]` 自动生成 trait 实现与注册条目
- **编译期收集**：基于 `linkme` 的自定义 link section，零运行时注册开销
- **运行期解析**：拓扑排序 + `DashMap` 存储，自动处理依赖顺序与循环检测
- **完整生命周期**：`build → inner_init → init → async_init → async_run → shutdown`
- **AOP**：`Interceptor` trait + `#[intercept]` 方法宏 + `around` 包装/短路，类型级 OnceLock 存储
- **插件生态**：Web / 日志 / 缓存 / 文件 / 定时任务 / 鉴权 / 国标视频等开箱即用

---

## 快速上手

### 依赖

```toml
[dependencies]
tx-di-core = "0.4.0"
```

工作区内开发通常用 path 依赖：

```toml
[dependencies]
tx-di-core = { path = "./tx-di-core" }
```

### 基本用法

```rust
use std::sync::Arc;
use tx_di_core::{Component, BuildContext, RIE};

// 无依赖单例
#[derive(Component, Default)]
pub struct DbPool;

// 依赖其他组件：字段 Arc<T> 自动从容器注入
#[derive(Component)]
pub struct UserService {
    pub db: Arc<DbPool>,
}

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    // 1. 构建阶段：加载配置 + 自动注册并构建所有组件
    //    new() 返回 RIE<BuildContext>（配置加载/拓扑排序失败会 Err）
    let ctx = BuildContext::new::<std::path::PathBuf>(None)?;

    // 2. 构建并运行：先同步完成 init + async_init，再返回「已就绪」句柄
    let app = ctx.build()?.ins_run().await?;

    // 3. ins_run 返回后组件才完整，此时注入获取的是已初始化的实例
    let svc = app.inject::<UserService>();
    app.waiting_exit().await;
    Ok(())
}
```

---

## `#[derive(Component)]` 宏

宏由 `tx-di-macros` 提供，详细文档见 [`tx-di-macros/README.md`](./tx-di-macros/README.md)。核心要点：

### 结构体属性 `#[component(...)]`

| 写法 | 说明 |
|------|------|
| `#[component(scope = Prototype)]` | 原型作用域（默认 `Singleton`） |
| `#[component(init)]` | 覆写 `inner_init`（回调 `fn init(&mut self, store)`） |
| `#[component(app_init)]` | 覆写 `init`（回调 `fn app_init(comp, app)`） |
| `#[component(app_async_init)]` | 覆写 `async_init`（回调 `fn app_async_init(comp, app)`） |
| `#[component(app_async_run)]` | 覆写 `async_run`（回调 `fn app_async_run(comp, app, token)`） |
| `#[component(shutdown)]` | 覆写 `shutdown`（回调 `fn shutdown(&self)`） |
| `#[component(init_sort = N)]` | 初始化排序（值越小越先执行，默认 `10000`） |
| `#[component(conf)]` / `#[component(conf = "key")]` | 从 TOML 配置反序列化 |
| `#[component(as_trait = dyn Trait)]` | 注册为 trait 实现，可按接口注入 |
| `#[component(intercept(T1, T2))]` | 声明 AOP 拦截器，配合方法上的 `#[intercept]` |

> 回调函数名与属性名保持一致（如 `app_init` 标志 → `fn app_init`）。宏生成的覆写方法带 `#[inline]`，空回调会被编译器消除。

### 字段属性 `#[tx_cst(...)]`

| 写法 | 语义 |
|------|------|
| `field: Arc<T>` | 从 DI 容器注入 |
| `field: Option<Arc<T>>` | **可选组件注入**：已注册则注入，未注册则 `None` |
| `field: Option<Arc<dyn Trait>>` | **可选 trait 注入**：有实现则 `Some`，无则 `None` |
| `field: Arc<dyn Trait>` | **必选 trait 注入**（必须有实现） |
| `field: Vec<Arc<dyn Trait>>` | **列表 trait 注入**：所有实现，无实现则空 Vec |
| `#[tx_cst(expr)]` | 用表达式赋值，**优先级最高**，覆盖类型推断 |
| `#[tx_cst(skip)]` | 跳过注入，使用 `Default::default()` |

```rust
#[derive(Component)]
pub struct MyService {
    pub db: Arc<DbPool>,                                  // DI 注入
    pub cache: Option<Arc<RedisClient>>,                   // 可选注入
    pub repo: Arc<dyn UserRepository>,                     // 必选 trait
    pub middlewares: Vec<Arc<dyn SipMiddleware>>,          // 列表 trait
    #[tx_cst("0.0.0.0:8080".to_string())]
    pub addr: String,                                      // 自定义值
    #[tx_cst(skip)]
    pub temp: Vec<u8>,                                     // 跳过
}
```

---

## `Component` trait

```rust
pub trait Component: Send + Sync + 'static {
    type Deps: DepsTuple;                              // 依赖元组，编译期类型可知
    fn build(deps: Self::Deps, store: &Store) -> Self; // 从依赖 + Store 构造（trait 注入用 Store）
    const SCOPE: Scope = Scope::Singleton;

    // 生命周期钩子（全部有默认实现，按需覆写）
    fn inner_init(&mut self, store: &Store) -> RIE<()> { Ok(()) }
    fn init(app: &Arc<App>) -> RIE<()> { Ok(()) }
    fn async_init(app: &Arc<App>) -> BoxFuture<RIE<()>> { Box::pin(async { Ok(()) }) }
    fn async_run(app: &Arc<App>, token: CancellationToken) -> BoxFuture<RIE<()>> { Box::pin(async { Ok(()) }) }
    fn shutdown(&self) {}
    fn init_sort() -> i32 { 10000 }
}
```

`RIE<T>` 是 `AppResult<T>` 的别名（`Result<T, AppError>`）。`DiErr` 错误码：`RegistryError` / `InjectError` / `ConfigError` / `TraitInjectError` / `AsyncInitError` / `TaskPanic`。

### 依赖上限

最多 **16 个 `Arc<T>` 依赖字段**，超过时编译期报错并给出迁移建议（合并为配置组件或拆分子组件）。

---

## 作用域

| 作用域 | 行为 |
|--------|------|
| **Singleton**（默认） | 全局唯一，首次注入时构建并缓存 `Arc<T>` |
| **Prototype** | 每次注入调用工厂，构造新实例。关闭时自动追踪存活实例 |

> Prototype 组件的 `#[component(shutdown)]` 回调也会被正确调用。

---

## 配置组件

```toml
# configs/config.toml
[app_config]
app_name = "production"
port = 9090
```

```rust
use serde::Deserialize;

#[derive(Component, Deserialize, Default)]
#[component(conf = "app_config")]
pub struct AppConfig {
    #[serde(default)]
    pub app_name: String,
    pub port: u16,
}
```

配置加载失败时返回 `Err` 而非 panic；`AppAllConfig` 提供 `get()`（宽松）和 `get_strict()`（严格）两种读取模式。

---

## Trait Object 注入

```rust
pub trait UserRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> Option<User>;
}

#[derive(Component)]
#[component(as_trait = dyn UserRepository)]
pub struct SqliteUserRepo { /* ... */ }

#[derive(Component)]
pub struct UserService {
    pub repo: Arc<dyn UserRepository>,            // 必选 trait 注入
    pub cache: Option<Arc<dyn CacheProvider>>,     // 可选 trait 注入
}
```

`Arc<dyn Trait>` 必选注入在 build 阶段直接从 Store 获取（安全，无 unsafe）。`Option<Arc<dyn Trait>>` 可选注入无实现时返回 `None`，不阻止启动。

---

## AOP 拦截器

拦截器本身也是 `#[derive(Component)]`，可依赖其他服务。拦截器链按**类型级 `OnceLock`** 存储（无全局表、无锁、无泄漏），在 `init` 阶段由 `#[component(intercept(...))]` 自动初始化。

```rust
use tx_di_core::aop::{Interceptor, CallContext, CallResult, BoxCall};

#[derive(Component, Default)]
pub struct AuthInterceptor;

impl Interceptor for AuthInterceptor {
    fn before(&self, ctx: &CallContext) -> tx_di_core::RIE<()> {
        tracing::info!("→ {}", ctx.method_name);
        Ok(())
    }
    fn after(&self, ctx: &CallContext, result: &CallResult) {
        match result {
            CallResult::Ok => tracing::info!("← OK"),
            CallResult::Err(e) => tracing::warn!("← ERR: {}", e),
        }
    }
    fn around(&self, ctx: &CallContext, call: BoxCall) -> CallResult {
        // 可完全替换/包装/重试业务逻辑
        call.execute()
    }
}

#[derive(Component)]
#[component(intercept(AuthInterceptor))]
pub struct UserService;

impl UserService {
    #[intercept]
    pub fn get_user(&self, id: u64) -> tx_di_core::RIE<User> { /* ... */ }
}
```

**执行流程**：`before → body → after`（`around` 为高级 API，用于短路/重试/包装）。

**参数传递**：`#[intercept]` 方法的参数通过 `CallContext::args` 传入拦截器。简单类型（`i64`/`u64`/`f64`/`bool`/`String`）直接映射；复杂类型通过 serde JSON 序列化存入 `ArgValue::Serialized`，拦截器可反序列化还原。

```rust
impl Interceptor for AuditInterceptor {
    fn before(&self, ctx: &CallContext) -> tx_di_core::RIE<()> {
        if let Some(ArgValue::Serialized { json, .. }) = ctx.get_arg("req") {
            let req: UserReq = serde_json::from_str(json).unwrap();
            // ...
        }
        Ok(())
    }
}
```

框架内置 `LoggingInterceptor`（INFO 级别 before/after 日志）和 `MetricsInterceptor`（计数器）。

---

## BuildContext & App

```rust
// 构建阶段：加载配置 + 自动注册所有组件（仅 inner_init 完成）
let ctx = BuildContext::new(Some("configs/app.toml"))?;

// 构建并运行：先同步完成 init + async_init，再返回「可用」句柄
let app = ctx.build()?.ins_run().await?;

// ins_run 返回后组件才完整，此时注入获取的是已初始化的实例
let db = app.inject::<DbPool>();

// 阻塞等待退出信号（Ctrl+C / SIGTERM）后优雅关闭
app.waiting_exit().await;
```

> **关键认知**：`BuildContext` 阶段 `inject` 拿到的实例只完成了 `build + inner_init`，`init` / `async_init` **尚未执行**。组件真正「可用」是在 `app.ins_run()` 返回之后。

| 阶段 | 方法 | 说明 |
|------|------|------|
| 构建 | `BuildContext::new(Option<path>) -> RIE<Self>` | 加载配置 + 注册并构建组件（仅 `inner_init`） |
| 解析 | `ctx.inject::<T>()` / `ctx.try_inject::<T>()` | 构建期注入（`Arc<T>`，实例不完整） |
| 构建 | `ctx.build() -> RIE<App>` | 移交 store 给 App（仍不执行 init/async_init） |
| 运行 | `ctx.build_and_run().await` | 构建并阻塞运行到退出 |
| 运行 | `app.ins_run(self) -> RIE<Arc<App>>` | 先完成 `init`+`async_init` 再返回**已就绪**句柄 |
| 运行 | `app.inject::<T>()` / `app.try_inject::<T>()` | 运行期注入（`Arc<T>`，实例已完整初始化） |
| 运行 | `app.waiting_exit().await` | 等待退出信号并优雅关闭（超时默认 5s，可通过 `shutdown_timeout_secs` 配置） |
| 关闭 | `app.shutdown().await` | 幂等关闭（多次调用安全），逆序关闭所有组件 + Prototype 实例 |

---

## 插件

插件同样是 `#[derive(Component)]` 组件，通过 `linkme` 注册。**使用前必须在代码中 `use` 该插件 crate**（仅写在 `Cargo.toml` 会被链接器优化掉，导致组件不注册）：

```rust
use tx_di_core::BuildContext;
use tx_di_axum;   // 触发 Web 组件注册
use tx_di_log;    // 触发日志组件注册

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    let ctx = BuildContext::new(Some("configs/app.toml"))?;
    ctx.build_and_run().await?;
    Ok(())
}
```

### 插件一览

| 插件 | 说明 | 关键依赖 / 特性 |
|------|------|----------------|
| **tx_di_log** | 日志插件：基于 `tracing` + `tracing-appender` 的按天滚动文件输出，可配级别/目录/格式。`LogPlugins` 以 `init_sort = i32::MIN` 最先初始化。 | `tracing-subscriber` |
| **tx_di_axum** | Web 服务器：axum + tokio，TOML 配置驱动。内置 `/health`、CORS、压缩、API 日志、`DiComp<T>` 提取器、`R<T>` 统一响应、静态文件/SPA 托管；`api-doc` feature（默认开）自动生成 OpenAPI。`WebPlugin` 在 `async_init` 启动服务器。 | `axum` / `tower-http` |
| **tx_di_cache** | 缓存：基于 `DashMap` 的内存缓存，可选 Redis 后端（`feature = "redis"`），统一 `Cache` 接口。 | `redis`（可选） |
| **tx_di_file** | 文件存储：基于 `OpenDAL`，支持本地（`local`，默认）与 S3（`s3`）后端，统一文件操作接口。 | `opendal` |
| **tx_di_job** | 定时任务：秒级 Cron 调度，支持内部函数 / Shell / Python 执行器，重试与超时，任务管理 API。依赖 `tx_di_toasty` 持久化。 | `cron`/`toasty` |
| **tx_di_toasty** | ORM：封装 `toasty`，支持 `sqlite`（默认）/`postgresql`/`mysql`/`dynamodb`（feature），提供 `ToastyPlugin` 注册模型。 | `toasty` |
| **tx_di_nacos** | 配置中心 + 服务注册 + 应用启动循环（非组件 crate）：启动拉取远程配置与本地合并、监听配置变更优雅重启、注册 HTTP/gRPC 端点，`app_loop!` 宏一键接入。 | `nacos-sdk` |
| **tx_di_sa_token** | 鉴权：封装 `sa-token-plugin-axum`，支持 `memory`（默认）/`redis`/`database`（feature）存储，与 `tx_di_axum` 配合做登录鉴权。 | `sa-token-plugin-axum` |
| **tx_di_sip** | SIP 协议栈：基于 `rsipstack`，提供 SIP 传输与事务层，供 GB28181 插件复用。 | `rsipstack` |
| **tx_di_gb28181** | GB28181 服务端（UAS/SIP 注册中心、设备目录、回放控制），基于 `rsipstack` + `tx_gb28181` + `tx_di_sip`。 | `tx_di_sip` |
| **tx_di_gb_dev** | GB28181 设备端（UAC）：注册 / 保活 / 响应，可被子设备与级联下层复用。 | `tx_di_sip` |

> 各插件均含独立 `README.md`（如 [`plugins/tx_di_axum/README.md`](./plugins/tx_di_axum/README.md)），含完整配置项与示例。

---

## 约束

| 约束 | 原因 |
|------|------|
| 组件需 `T: Send + Sync + 'static` | 存入 `Arc<dyn Any + Send + Sync>` |
| 配置组件需 `Deserialize + Default` | serde 反序列化 |
| trait 注入需 `Trait: Any + Send + Sync` | 需 `TypeId::of::<dyn Trait>()` |
| 避免循环依赖 | 拓扑排序检测，启动时返回 `RegistryError` |
| 最多 16 个 `Arc<T>` 依赖 | `DepsTuple` 元组实现上限，编译期检查 |

---

## 目录结构

```
tx_di/
├── tx-di-core/        # DI 框架核心（Component / BuildContext / App / Store / AOP）
├── tx-di-macros/      # #[derive(Component)] 与 #[intercept] 过程宏
├── plugins/           # 插件生态（见上方「插件」）
├── common/            # 公共库（tx_error / tx_common 等）
├── configs/           # 示例 TOML 配置
└── examples/          # 示例工程
```

---

## 许可证

MIT
