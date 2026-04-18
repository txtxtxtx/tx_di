# di-framework v2

基于 `proc_macro` + `linkme` + `const 类型编码` 的编译期依赖注入框架。

**v2 新增**：Singleton / Prototype 作用域、`#[inject(expr)]` 自定义值注入、`CompRef` 类型擦除存储层。

## 快速上手

```rust
use std::collections::HashMap;
use di_core::{Singleton, Prototype};
use di_macros::{component, app};

// ── 单例组件（默认）──────────────────────────────────────
#[derive(Clone, Debug)]
#[component]
pub struct DbPool { /* ... */ }

// ── 带自定义注入值的单例 ─────────────────────────────────
#[derive(Clone, Debug)]
#[component]
pub struct AppConfig {
    #[inject("my-app".to_string())]
    pub name: String,

    #[inject(default_port())]   // 调用任意表达式/函数
    pub port: u16,
}
fn default_port() -> u16 { 8080 }

// ── 原型组件（每次注入独立实例）──────────────────────────
#[derive(Clone, Debug)]
#[component(scope = Prototype)]
pub struct RequestLogger {
    #[inject("[REQ]".to_string())]
    pub prefix: String,

    #[inject(0u64)]
    pub count: u64,
}

// ── 聚合组件：混用单例 + 原型 + 自定义注入 ───────────────
#[derive(Debug)]
#[component]
pub struct AppServer {
    pub db:      Singleton<DbPool>,        // 共享同一个 DbPool
    pub config:  Singleton<AppConfig>,     // 共享配置
    pub logger:  Prototype<RequestLogger>, // 独占自己的 Logger

    #[inject(HashMap::new())]
    pub headers: HashMap<String, String>,  // 不走 ctx，直接赋值
}

// ── 声明模块，生成 build_app_module() ──────────────────
app! {
    AppModule [
        DbPool,
        AppConfig,
        RequestLogger,  // 原型：注册工厂，不立即构建
        AppServer,
    ]
}

fn main() {
    let mut ctx = build_app_module();
    let server = ctx.take::<AppServer>();
    println!("{:?}", server);
}
```

## 核心概念

### Scope（作用域）

| 作用域 | 声明方式 | 存储 | 注入行为 |
|--------|---------|------|---------|
| **Singleton**（默认） | `#[component]` | `Arc<T>` 存一份 | `Arc::clone`，计数 +1 |
| **Prototype** | `#[component(scope = Prototype)]` | 工厂函数 | 每次注入构造新实例 |

### 字段声明方式

| 写法 | 语义 |
|------|------|
| `field: Singleton<T>` | 单例注入，持有 `Arc<T>`，实现 `Deref<Target=T>` |
| `field: Prototype<T>` | 原型注入，持有 `Box<T>`，实现 `Deref<Target=T>` + `DerefMut` |
| `#[inject(expr)]` + 任意类型 | 不走 ctx，直接用表达式赋值，不计入依赖图 |

### `Singleton<T>` vs `Prototype<T>`

```rust
// Singleton：clone 只增加引用计数，不复制数据
let s1: Singleton<DbPool> = ctx.get_singleton::<DbPool>();
let s2 = s1.clone();  // Arc::clone，s1 和 s2 指向同一个 DbPool

// Prototype：clone 构造全新实例（需 T: Clone）
let p1: Prototype<LogService> = ctx.get_prototype::<LogService>();
let p2 = p1.clone();  // 新的 LogService 实例，与 p1 完全独立

// 两者都可以直接 .method() 调用（Deref 透明）
s1.query("select 1");   // 等价于 (*s1).query(...)
p1.log("hello");        // 等价于 (*p1).log(...)
```

### `#[inject(expr)]`

```rust
#[component]
pub struct Config {
    // 任意 Rust 表达式
    #[inject(std::env::var("APP_NAME").unwrap_or("default".to_string()))]
    pub name: String,

    // 函数调用
    #[inject(load_tls_config())]
    pub tls: TlsConfig,

    // 字面量
    #[inject(42u32)]
    pub timeout_secs: u32,

    // HashMap、Vec 等集合
    #[inject(HashMap::new())]
    pub cache: HashMap<String, String>,

    // 正常 DI 注入（无 #[inject]）
    pub db: Singleton<DbPool>,
}
```

`#[inject(expr)]` 字段**不计入 `DEP_IDS`**，不参与依赖图拓扑排序。

## 架构三层

```
用户代码
  #[component(scope = Prototype)]  struct Logger { #[inject(...)] prefix: String }
  #[component]  struct AppServer { logger: Prototype<Logger>, db: Singleton<DbPool> }
  app! { AppModule [ DbPool, Logger, AppServer ] }
         │
         │ proc_macro 展开
         ▼
di-macros
  1. 解析 scope 参数 → Scope::Singleton / Scope::Prototype
  2. 解析字段：Singleton<T> / Prototype<T> / #[inject(expr)] / Raw
  3. 生成 ComponentDescriptor impl（含 DEP_IDS + SCOPE + build()）
  4. 生成 linkme distributed_slice 注册条目
  5. app!{} 生成 build_app_module()，按 SCOPE 选择 insert_singleton / register_prototype
         │
         │ 链接器合并 link section
         ▼
di-core
  - Singleton<T> / Prototype<T>：用户侧字段类型，均实现 Deref
  - CompRef：内部类型擦除（Singleton(Arc<dyn Any>) / Prototype(fn)）
  - BuildContext：TypeId → CompRef 映射
  - COMPONENT_REGISTRY：全局组件元数据切片（linkme 收集）
```

## BuildContext API

```rust
let mut ctx = build_app_module();

// 单例
let arc:  Arc<DbPool>      = ctx.get_arc::<DbPool>();
let sing: Singleton<DbPool> = ctx.get_singleton::<DbPool>();
let ref_: &DbPool           = ctx.get::<DbPool>();       // 兼容旧接口
let owned: DbPool           = ctx.take::<DbPool>();      // 取走所有权

// 原型（每次调用构造新实例）
let proto: Prototype<Logger> = ctx.get_prototype::<Logger>();
```

## 关键设计决策

### 1. `insert_singleton` 双 key 存储
`TypeId<T>` 和 `TypeId<Arc<T>>` 各存一份，方便字段声明为 `Singleton<T>`（取 Arc<T>）或 legacy `Arc<T>` 类型的组件都能从 ctx 找到。

### 2. `take` 的 Arc 计数处理
`take::<T>()` 会**同时移除** `TypeId<T>` 和 `TypeId<Arc<T>>` 两条记录，降低引用计数后才能成功拆包。

### 3. 原型不预构建
`Scope::Prototype` 组件在 `app!{}` 生成的初始化函数中**只注册工厂函数**，不立即构建实例，保证每次注入都是全新实例。

### 4. `#[inject(expr)]` 字段不进依赖图
宏解析字段时，有 `#[inject]` 的字段不加入 `DEP_IDS`，不影响拓扑排序，也不要求对应类型在 ctx 中存在。

## 约束

| 约束 | 原因 |
|------|------|
| Singleton 组件需 `T: Send + Sync` | 存入 `Arc<dyn Any + Send + Sync>` |
| Prototype 组件 `clone()` 需 `T: Clone` | `Prototype<T>::clone()` 构造新实例 |
| 无字段组件 build() 生成 `Self {}` | 需要 struct 有默认可构造的形态 |
| `take()` 失败时 panic | Arc 仍被 Singleton<T> 字段持有时无法拆包，应在所有下游组件都已构建后再 take |
| `app!{}` 列表需用户按拓扑序排列 | proc_macro 阶段无类型信息，debug build 会验证 |

## 测试

```
test tests::test_app_config_inject    ... ok   // #[inject] 自定义值
test tests::test_inject_custom_values ... ok   // HashMap + String 注入
test tests::test_prototype_independent ... ok  // 两个 Prototype 实例完全独立
test tests::test_registry             ... ok   // linkme 注册表验证
test tests::test_singleton_shared     ... ok   // Arc 指针相等验证单例共享
```
