# tx-di-core

类型驱动的 Rust 依赖注入（DI）框架核心 crate。

- `#[derive(Component)]` 自动注册组件（`tx-di-macros` 提供）
- `linkme` 编译期收集组件元数据，运行期拓扑排序注入
- 类型安全的 `Store::inject::<T>()` 解析依赖
- 完整的生命周期：`build → inner_init → init → async_init → async_run → shutdown`
- 内置 AOP 拦截器、配置管理、作用域（Singleton/Prototype）

当前版本：`0.3.0`

---

## 安装

```toml
[dependencies]
tx-di-core = "0.3.0"
```

工作区内开发通常使用 path 依赖：

```toml
[dependencies]
tx-di-core = { path = "../tx-di-core" }
```

> **重要**：`tx-di-core` 依赖 `linkme` 做编译期静态注册。若要把某个插件/组件 crate 真正链接进二进制，
> 必须在代码中 `use` 该 crate（仅写进 `Cargo.toml` 会被链接器优化掉）。详见下方「插件」与 `tx-di-macros` 说明。

---

## 核心概念

| 概念 | 说明 |
|------|------|
| `Component` trait | 每个被 DI 管理的类型实现此 trait，用 associated type `Deps` 声明依赖 |
| `ComponentMeta` | 瘦注册条目，由 `#[derive(Component)]` 生成，经 `linkme` 编译期收集 |
| `Store` | 类型擦除的组件存储（`DashMap<TypeId, CompRef>`），运行期解析依赖 |
| `BuildContext` | 构建阶段：加载配置 → 拓扑排序 → 构建组件 → 缓存单例 |
| `App` | 运行阶段：按序执行 `init → async_init → async_run → shutdown` |
| `Scope` | `Singleton`（默认，全局唯一）或 `Prototype`（每次注入新建） |
| AOP | `Interceptor` trait + `#[intercept]` 方法宏，零额外运行时开销 |

---

## 快速上手

```rust
use std::sync::Arc;
use tx_di_core::{Component, BuildContext};

// 无依赖单例（用 Default 自动构造）
#[derive(Component, Default)]
pub struct DbPool;

// 依赖其他组件：字段 Arc<T> 自动从容器注入
#[derive(Component)]
pub struct UserService {
    pub db: Arc<DbPool>,
}

#[tokio::main]
async fn main() {
    // 1. 构建阶段：加载配置、扫描并注册所有 #[derive(Component)]。
    //    此时组件仅完成 build + inner_init，init / async_init 尚未执行，
    //    实例「不完整」，不能当成品去调用业务方法（如 db 连接）。
    let ctx = BuildContext::new::<std::path::PathBuf>(None);

    // 2. 构建 App 并运行：先同步完成 init + async_init，再返回「已就绪」句柄。
    //    ⚠️ 组件真正可用，是在 ins_run() 返回之后，而不是在 BuildContext 阶段。
    let app = ctx.build().expect("build failed").ins_run().await.expect("run failed");

    // 3. 此后组件才完整：init / async_init 已执行完毕，可安全使用。
    let svc = app.inject::<UserService>();
    println!("user service ready");

    // 4. 等待退出信号（Ctrl+C / SIGTERM）后优雅关闭
    app.waiting_exit().await;
}
```

> 组件之间的依赖由框架在构建期自动注入，**业务代码通常不需要手动 inject**。
> 真正「使用」组件（调用其业务方法）应放在某个组件的 `async_run` / `init` 回调里，
> 或在 `ins_run()` 返回后通过 `app.inject::<T>()` 获取——此时组件已完全初始化。

构建与运行的三种常用形态：

```rust
// 形式 A：构建出 App 句柄，自行控制运行（ins_run 返回即代表 init+async_init 完成）
let app = ctx.build()?.ins_run().await?;
app.waiting_exit().await;

// 形式 B：一行启动并阻塞到退出（内部等价于 A）
ctx.build_and_run().await?;

// 形式 C：仅构建 App，不运行任何生命周期（测试 / 仅查看组件实例，不可当成品使用）
let app = ctx.build()?;
// 注意：此时 init / async_init 未执行，app.inject::<T>() 拿到的实例仍不完整
```

---

## `Component` trait

```rust
pub trait Component: Send + Sync + 'static {
    type Deps: DepsTuple;                       // 依赖元组，编译期类型可知
    fn build(deps: Self::Deps) -> Self;          // 从依赖构造实例（纯函数）
    const SCOPE: Scope = Scope::Singleton;       // 作用域

    // ── 生命周期钩子（全部有默认实现，按需覆写）──
    fn inner_init(&mut self, store: &Store) -> RIE<()> { Ok(()) }
    fn init(app: &Arc<App>) -> RIE<()> { Ok(()) }
    fn async_init(app: &Arc<App>) -> BoxFuture<RIE<()>> { Box::pin(async { Ok(()) }) }
    fn async_run(app: &Arc<App>, token: CancellationToken) -> BoxFuture<RIE<()>> { Box::pin(async { Ok(()) }) }
    fn shutdown(&self) {}
    fn init_sort() -> i32 { 10000 }              // 值越小越先执行
    fn trait_impls() -> &'static [fn() -> TypeId] { &[] }  // as_trait 自动生成
}
```

绝大多数情况下直接用 `#[derive(Component)]` 自动生成实现；需要极致控制时可手写 `impl Component`。

`RIE<T>` 是 `tx_error::AppResult<T>` 的类型别名。错误类型统一为 `tx_error::AppError`，DI 框架自带 `DiErr`（`RegistryError` / `AsyncInitError` / `TaskPanic` / `InjectError`）错误码。

---

## 构建期与运行期

### `BuildContext`（构建与解析）

| 方法 | 说明 |
|------|------|
| `BuildContext::new::<P>(Option<config_path>)` | 加载配置 + 自动注册所有组件 |
| `ctx.inject::<T>()` | 注入组件，返回 `Arc<T>`（未注册则 panic，附已注册列表）。**构建期仅完成 `inner_init`，`init`/`async_init` 尚未执行，实例不完整，不可当成品使用** |
| `ctx.try_inject::<T>()` | 同上，失败时返回 `None` |
| `ctx.store()` | 获取底层 `Store` 引用 |
| `ctx.len()` / `ctx.is_empty()` | 已注册组件数量 |
| `ctx.build() -> RIE<App>` | 把 store 移交给 App |
| `ctx.build_and_run().await` | 构建并运行到退出 |
| `BuildContext::debug_registry()` | 打印拓扑排序后的组件清单（调试用） |

### `App`（运行期）

| 方法 | 说明 |
|------|------|
| `app.inject::<T>()` / `app.try_inject::<T>()` | 注入组件 |
| `app.ins_run(self).await -> RIE<Arc<App>>` | 先完成 `init`+`async_init` 再返回可用句柄 |
| `app.shutdown().await` | 逆序优雅关闭所有组件 |
| `app.waiting_exit().await` | 等待退出信号 → 取消 token → 关闭后台任务 → shutdown |

生命周期执行顺序（由拓扑序 + `init_sort` 共同决定）：
`init`（同步）→ `async_init`（异步）→ `async_run`（后台并行任务，直到 `CancellationToken` 触发）→ `shutdown`（逆序）。

---

## 依赖注入规则

字段类型决定注入行为（由 `#[derive(Component)]` 推导，详见 `tx-di-macros`）：

| 字段类型 | 行为 |
|----------|------|
| `Arc<T>` | 普通组件注入，从容器取 `T` |
| `Arc<dyn Trait>` | 必选 trait 注入，`inner_init` 中填充 |
| `Option<Arc<dyn Trait>>` | 可选 trait 注入，找不到保持 `None` |
| `Option<T>` | 可选普通依赖，保持 `None` |
| `#[tx_cst(expr)]` | 用表达式赋值，不从容器注入 |
| `#[tx_cst(skip)]` | 跳过注入，使用 `Default::default()` |

---

## 配置组件

通过 `#[component(conf = "key")]` 让组件从 TOML 配置反序列化。框架在 `BuildContext::new` 阶段加载配置（默认读取可执行文件同目录 `config/config.toml`，或显式传入路径），并通过 `AppAllConfig` 存放于 Store。

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

全局配置也可直接注入后读取：

```rust
let cfg = ctx.inject::<tx_di_core::AppAllConfig>();
if let Some(host) = cfg.get::<String>("web_config.host") { /* ... */ }
```

---

## AOP 拦截器

拦截器本身也是 `#[derive(Component)]`，可依赖其他服务，通过 `#[component(intercept(T1, T2))]` 声明，
并对目标方法加 `#[intercept]` 属性宏。详见 `tx-di-macros` 与 `aop` 模块文档。

```rust
use tx_di_core::aop::{Interceptor, CallContext, CallResult};

#[derive(Component, Default)]
pub struct LoggingInterceptor;

impl Interceptor for LoggingInterceptor {
    fn before(&self, ctx: &CallContext) -> RIE<()> {
        tracing::info!("→ {}", ctx.method_name);
        Ok(())
    }
    fn after(&self, ctx: &CallContext, result: &mut CallResult) {
        tracing::info!("← {}", ctx.method_name);
    }
}
```

框架内置 `LoggingInterceptor`、`MetricsInterceptor`。拦截器链按「组件实例指针」存储，支持同进程多 App 互不干扰。

---

## 模块结构

```
tx-di-core/src/
├── lib.rs          # crate 根，统一 re-export
├── component.rs    # Component trait + DepsTuple
├── lifecycle.rs    # BuildContext / App / 生命周期编排
├── store.rs        # Store / CompRef / trait object 注入
├── registry.rs     # ComponentMeta + linkme 收集
├── topology.rs     # 拓扑排序（Kahn，支持 init_sort 优先级）
├── scope.rs        # Scope 枚举
├── config.rs       # AppAllConfig 配置管理
├── aop.rs          # Interceptor / InterceptorChain / 内置拦截器
├── error.rs        # DiErr 错误码（复用 tx_error::AppError）
```

### 主要 re-export

```rust
// 宏与 trait
pub use tx_di_macros::{Component, intercept};

// 核心类型
pub use component::{BoxFuture, Component, DepsTuple};
pub use lifecycle::{App, BuildContext, InnerContext, get_sys_config, set_sys_config, CONFIG_PATH};
pub use registry::{ComponentMeta, COMPONENT_REGISTRY};
pub use scope::Scope;
pub use store::{Store, CompRef, TraitImplEntry, TraitImplMap,
                inject_from_store, inject_trait_from_store, inject_all_traits_from_store};
pub use topology::topo_sort;
pub use aop::{CallContext, CallResult, Interceptor, InterceptorChain};

// 错误与通用
pub use tx_error::{AppError, AppResult, AppErrCode, CodeMsg};
pub use tx_common::{ApiR, ApiRes, FormattedDateTime, RCode};
pub type RIE<T> = AppResult<T>;
pub use tokio_util::sync::CancellationToken;
```

---

## 约束

| 约束 | 原因 |
|------|------|
| 组件需 `T: Send + Sync + 'static` | 存入 `Arc<dyn Any + Send + Sync>` |
| 配置组件需 `Deserialize + Default` | serde 反序列化 |
| trait 注入需 `Trait: Any + Send + Sync` | 需 `TypeId::of::<dyn Trait>()` |
| 避免循环依赖 | 拓扑排序检测，启动时返回 `RegistryError` |
| 最多 16 个依赖 | `DepsTuple` 元组实现上限（宏元数限制） |

---

## 许可证

MIT
