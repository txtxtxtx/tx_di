# tx_di

类型驱动的 Rust 依赖注入框架。`Component` trait 声明依赖，`#[derive(Component)]` 自动注册，linkme 编译期收集，运行期拓扑排序注入。

---

## 快速上手

### 依赖

```toml
[dependencies]
tx-di-core = "0.3"
```

### 基本用法

```rust
use std::sync::Arc;
use tx_di_core::{Component, BuildContext};

// 无依赖单例
#[derive(Component, Default)]
pub struct DbPool;

// 依赖其他组件
#[derive(Component)]
pub struct UserService {
    pub db: Arc<DbPool>,
}

fn main() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let svc = ctx.inject::<UserService>();
}
```

---

## `#[derive(Component)]` 宏

### 结构体属性 `#[component(...)]`

| 写法 | 说明 |
|------|------|
| `#[component(scope = Prototype)]` | 每次注入创建新实例 |
| `#[component(init)]` | 有自定义生命周期实现 |
| `#[component(conf)]` | 从 TOML 配置反序列化 |
| `#[component(conf = "key")]` | 指定 TOML 配置键 |
| `#[component(as_trait = dyn Trait)]` | 注册为 trait 实现 |

### 字段属性 `#[tx_cst(...)]`

| 写法 | 语义 |
|------|------|
| `field: Arc<T>` | 从 DI 容器注入 |
| `#[tx_cst(expr)]` | 用表达式赋值 |
| `#[tx_cst(skip)]` | 跳过，使用 `Default::default()` |
| `Option<T>` 字段 | 自动设为 `None` |

```rust
#[derive(Component)]
pub struct MyService {
    pub db: Arc<DbPool>,                            // DI 注入
    #[tx_cst("0.0.0.0:8080".to_string())]
    pub addr: String,                                // 自定义值
    #[tx_cst(skip)]
    pub temp: Vec<u8>,                               // 跳过
}
```

---

## Component trait

```rust
pub trait Component: Send + Sync + 'static {
    type Deps: DepsTuple;
    fn build(deps: Self::Deps) -> Self;
    const SCOPE: Scope = Scope::Singleton;

    // 生命周期钩子（全部可选）
    fn inner_init(&mut self, store: &Store) -> Result<(), IE> { Ok(()) }
    fn init(app: &Arc<App>) -> Result<(), IE> { Ok(()) }
    fn async_init(app: &Arc<App>) -> BoxFuture<Result<(), IE>> { ... }
    fn async_run(app: &Arc<App>, token: CancellationToken) -> BoxFuture<Result<(), IE>> { ... }
    fn shutdown(&self) {}
    fn init_sort() -> i32 { 10000 }
}
```

---

## 作用域

| 作用域 | 行为 |
|--------|------|
| **Singleton**（默认） | 全局唯一，首次注入时构建并缓存 |
| **Prototype** | 每次注入创建新实例 |

---

## 配置组件

```toml
# configs/config.toml
[app_config]
app_name = "production"
port = 9090
```

```rust
#[derive(Component, serde::Deserialize, Default)]
#[component(conf = "app_config")]
pub struct AppConfig {
    #[serde(default)]
    pub app_name: String,
    pub port: u16,
}
```

---

## Trait Object 注入

```rust
pub trait UserRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> Option<User>;
}

#[derive(Component)]
#[component(as_trait = dyn UserRepository)]
pub struct SqliteUserRepo { ... }

#[derive(Component)]
pub struct UserService {
    pub repo: Arc<dyn UserRepository>,   // 自动解析
}
```

---

## BuildContext & App

```rust
// 构建阶段
let ctx = BuildContext::new(Some("configs/config.toml"));
let db = ctx.inject::<DbPool>();
let app = ctx.build()?;

// App 阶段
let app = Arc::new(app);
let db = app.inject::<DbPool>();

// 异步运行
let app = ctx.ins_run().await?;
app.waiting_exit().await;
```

---

## AOP 拦截器

```rust
use tx_di_core::aop::{Interceptor, CallContext, CallResult};

pub struct LoggingInterceptor;

impl Interceptor for LoggingInterceptor {
    fn before(&self, ctx: &CallContext) {
        tracing::info!("→ {}", ctx.method_name);
    }
    fn after(&self, ctx: &CallContext, result: &CallResult) {
        tracing::info!("← {}", ctx.method_name);
    }
}
```

---

## 约束

| 约束 | 原因 |
|------|------|
| 组件需 `T: Send + Sync + 'static` | 存入 `Arc<dyn Any + Send + Sync>` |
| 配置组件需 `Deserialize + Default` | serde 反序列化 |
| trait 注入需 `Trait: Any + Send + Sync` | `TypeId::of::<dyn Trait>()` |
| 避免循环依赖 | 拓扑排序检测，启动时 panic |
| 最多 16 个依赖 | DepsTuple 元组实现限制 |

---

## 许可证

MIT
