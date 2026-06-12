# tx_di

基于 `proc_macro` + `linkme` 的 Rust 依赖注入框架。编译期收集元数据，运行期自动拓扑排序并注入，零反射、零运行时扫描。

---

## 快速上手

### 依赖

```toml
[dependencies]
tx-di-core = "0.2"
```

### 基本用法

```rust
use std::sync::Arc;
use tx_di_core::{tx_comp, tx_cst, BuildContext};

// 无依赖单例
#[tx_comp]
pub struct DbPool;

// 带自定义值
#[tx_comp]
pub struct AppConfig {
    #[tx_cst("my-app".to_string())]
    pub name: String,
}

// 依赖其他组件
#[tx_comp]
pub struct UserService {
    pub db: Arc<DbPool>,
    pub config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let svc = ctx.inject::<UserService>();
}
```

---

## 属性宏 `#[tx_comp]`

| 写法 | 说明 |
|------|------|
| `#[tx_comp]` | 默认 Singleton |
| `#[tx_comp(scope = Prototype)]` | 每次注入创建新实例 |
| `#[tx_comp(scope)]` | 同 Prototype |
| `#[tx_comp(init)]` | 自行实现 `CompInit` trait |
| `#[tx_comp(conf)]` | 从 TOML 配置自动反序列化 |
| `#[tx_comp(conf = "key")]` | 指定 TOML 配置键 |
| `#[tx_comp(as_trait = dyn Trait)]` | 注册为 trait 实现，支持 trait object 注入 |

可组合使用：`#[tx_comp(scope = Prototype, as_trait = dyn MyTrait)]`

---

## 字段注入

| 写法 | 语义 |
|------|------|
| `field: Arc<T>` | 从 DI 容器注入，按 T 的 scope 处理 |
| `#[tx_cst(expr)]` | 直接用表达式赋值，不进依赖图 |
| `#[tx_cst(skip)]` | 跳过，使用 `Default::default()` |
| `Option<T>` 字段 | 自动设为 `None` |

```rust
#[tx_comp]
pub struct MyService {
    pub db: Arc<DbPool>,                            // DI 注入
    #[tx_cst("0.0.0.0:8080".to_string())]
    pub addr: String,                                // 自定义值
    #[tx_cst(RwLock::new(HashMap::new()))]
    pub cache: RwLock<HashMap<u64, String>>,         // 自定义值
    #[tx_cst(skip)]
    pub temp: Vec<u8>,                               // 跳过
}
```

---

## Trait Object 注入

通过 `as_trait` 将具体类型注册为 trait 实现，注入时自动解析。

**要求**：trait 需要 `Any + Send + Sync` 作为 supertrait。

```rust
// 定义 trait
pub trait UserRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<User>>;
}

// 注册实现
#[tx_comp(as_trait = dyn UserRepository)]
pub struct SqliteUserRepository {
    #[tx_cst(RwLock::new(HashMap::new()))]
    users: RwLock<HashMap<u64, User>>,
}

// 注入 trait object
#[tx_comp]
pub struct UserService {
    pub user_repo: Arc<dyn UserRepository>,   // 自动解析到 SqliteUserRepository
}
```

一个 trait 可以有多个实现，通过 `inject_all_traits_from_store` 获取全部：

```rust
let repos: Vec<Arc<dyn UserRepository>> = inject_all_traits_from_store(store);
```

---

## 作用域 Scope

| 作用域 | 行为 |
|--------|------|
| **Singleton**（默认） | 全局唯一，首次注入时构建并缓存 |
| **Prototype** | 每次 `inject()` 创建新实例 |

scope 标记在被注入者上，消费者只需写 `Arc<T>`。

---

## 配置组件

```toml
# configs/app.toml
[my_config]
app_name = "production"
port = 9090
```

```rust
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[tx_comp(conf)]                // 从 [my_config] 加载
pub struct MyConfig {
    #[serde(default)]
    pub app_name: String,
    pub port: u16,
}

// 自定义键名
#[tx_comp(conf = "server")]
pub struct ServerConfig { ... }
```

加载配置文件：

```rust
let ctx = BuildContext::new(Some("configs/app.toml"));
let ctx = BuildContext::new::<std::path::PathBuf>(None);  // 不用配置
```

---

## 自定义初始化 CompInit

```rust
use tx_di_core::{tx_comp, CompInit, App, RIE, CancellationToken};
use tx_di_core::async_method;

#[tx_comp(init)]
pub struct AppServer {
    pub addr: String,
}

impl CompInit for AppServer {
    // 组件构建后立即调用（BuildContext 阶段）
    fn inner_init(&mut self, ctx: &InnerContext) -> RIE<()> {
        println!("inner_init");
        Ok(())
    }

    // 同步初始化
    fn init(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
        println!("init");
        Ok(())
    }

    // 异步初始化（用 async_method! 简化）
    async_method!(
        fn async_init_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
            println!("async_init");
            Ok(())
        }
    );

    // 异步运行（长期任务）
    async_method!(
        fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
            println!("running");
            Ok(())
        }
    );

    // 初始化顺序（值越小越先执行，默认 10000）
    fn init_sort() -> i32 { 100 }
}
```

---

## BuildContext & App

```rust
// 构建阶段
let ctx = BuildContext::new(Some("configs/app.toml"));
let db: Arc<DbPool> = ctx.inject::<DbPool>();
BuildContext::debug_registry();       // 打印注册表
let app: App = ctx.build()?;          // 固化

// 或快捷方式
ctx.build_and_run().await?;           // build + init + async_init

// App 阶段（支持 Singleton 和 Prototype）
let app = Arc::new(ctx.build()?);
let db = app.inject::<DbPool>();
let logger = app.inject::<RequestLogger>();  // Prototype 每次新建

// 异步运行（后台任务管理）
let app = ctx.ins_run().await?;       // 返回 Arc<App>，后台初始化
app.waiting_exit().await;             // 等待 Ctrl+C 信号
```

---

## 约束

| 约束 | 原因 |
|------|------|
| 组件需 `T: Send + Sync + 'static` | 存入 `Arc<dyn Any + Send + Sync>` |
| 配置组件需 `Deserialize + Default` | serde 反序列化 |
| trait 注入需 `Trait: Any + Send + Sync` | `TypeId::of::<dyn Trait>()` 要求 |
| 避免循环依赖 | 拓扑排序检测，存在则 panic |
| 插件 crate 必须 `use` 导入 | `linkme` 依赖链接器 |

---

## 许可证

MIT
