# di-framework

基于 `proc_macro` + `linkme` 的编译期依赖注入框架。

**核心特性**：Singleton / Prototype 作用域、`#[tx_cst(expr)]` 自定义值注入、自动依赖拓扑排序。

## 快速上手

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use di_core::{app, tx_comp, tx_cst};

// ── 单例组件（默认）──────────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp]
pub struct DbPool {
    // 无字段组件自动构建
}

// ── 带自定义注入值的单例 ─────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp]
pub struct AppConfig {
    #[tx_cst("my-app".to_string())]
    pub app_name: String,

    #[tx_cst(default_port())]
    pub port: u16,
}

fn default_port() -> u16 {
    8080
}

// ── 原型组件（每次注入独立实例）──────────────────────────
#[derive(Clone, Debug)]
#[tx_comp(scope = Prototype)]
pub struct RequestLogger {
    #[tx_cst("[REQUEST]".to_string())]
    pub prefix: String,

    #[tx_cst(Arc::new(Mutex::new(0u64)))]
    count: Arc<Mutex<u64>>,
}

impl RequestLogger {
    pub fn log(&self, msg: &str) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        println!("{} [#{}] {}", self.prefix, *c, msg);
    }
}

// ── 依赖其他组件的服务 ───────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp]
pub struct UserService {
    pub db: Arc<DbPool>,
    pub config: Arc<AppConfig>,
}

// ── 聚合组件：混用单例 + 原型 + 自定义注入 ───────────────
#[derive(Debug)]
#[tx_comp]
pub struct AppServer {
    pub user_svc: Arc<UserService>,         // 单例注入
    pub logger: Arc<RequestLogger>,         // 原型注入，独占实例

    #[tx_cst(HashMap::new())]
    pub headers: HashMap<String, String>,   // 自定义值，不走 DI

    #[tx_cst("0.0.0.0:8080".to_string())]
    pub bind_addr: String,
}

// ── 声明模块，自动生成 build_app_module() ────────────────
// 不指定组件列表时，自动扫描所有 #[tx_comp] 标记的组件
app! { AppModule }

fn main() {
    let mut ctx = build_app_module();
    let server = ctx.take::<AppServer>();
    println!("Server ready at {}", server.bind_addr);
}
```

## 核心概念

### Scope（作用域）

| 作用域 | 声明方式 | 行为 |
|--------|---------|------|
| **Singleton**（默认） | `#[tx_comp]` | 全局共享，首次注入时构建，缓存 `Arc<T>` |
| **Prototype** | `#[tx_comp(scope = Prototype)]` | 每次注入调用工厂，构造新实例 |

### 字段声明方式

| 写法 | 语义 |
|------|------|
| `field: Arc<T>` | 从 DI 容器注入，框架根据 T 的 scope 自动处理 |
| `#[tx_cst(expr)]` + 任意类型 | 不走 DI，直接用表达式赋值，不计入依赖图 |

### 字段注入示例

```rust
#[tx_comp]
pub struct MyComponent {
    // 自动注入：框架根据 DbPool 的 scope 决定是共享还是新建
    pub db: Arc<DbPool>,
    
    // 自定义值：直接调用表达式，不参与依赖图
    #[tx_cst("custom_value".to_string())]
    pub name: String,
    
    // 调用函数
    #[tx_cst(load_config())]
    pub config: Config,
    
    // 集合类型
    #[tx_cst(HashMap::new())]
    pub cache: HashMap<String, String>,
}
```

**关键原则**：scope 标记在**被注入者**上，消费者不需要知道依赖是单例还是原型。

### `#[tx_cst(expr)]`

用于标记字段使用自定义表达式初始化，而不是从 DI 容器注入。

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

`#[tx_cst(expr)]` 字段**不计入 `DEP_IDS`**，不参与依赖图拓扑排序。

## 架构三层

```
用户代码
  #[tx_comp(scope = Prototype)]  struct Logger { #[tx_cst(...)] prefix: String }
  #[tx_comp]  struct AppServer { logger: Arc<Logger>, db: Arc<DbPool> }
  app! { AppModule }  // 自动扫描所有组件
         │
         │ proc_macro 展开
         ▼
di-macros
  1. 解析 scope 参数 → Scope::Singleton / Scope::Prototype
  2. 解析字段：Arc<T>（DI 注入） / #[tx_cst(expr)]（自定义值）
  3. 生成 ComponentDescriptor impl（含 DEP_IDS + SCOPE + build()）
  4. 生成 linkme distributed_slice 注册条目
  5. app!{} 生成 build_app_module()，自动拓扑排序并注册
         │
         │ 链接器合并 link section
         ▼
di-core
  - BuildContext：TypeId → CompRef 映射，支持并发访问（DashMap）
  - CompRef：内部类型擦除（Cached(Arc<dyn Any>) / Factory(fn)）
  - COMPONENT_REGISTRY：全局组件元数据切片（linkme 收集）
  - topo_sort：自动拓扑排序，检测循环依赖
```

## BuildContext API

```rust,ignore
let mut ctx = build_app_module();

// 注入组件（根据组件自身的 scope 自动处理）
let db: Arc<DbPool> = ctx.inject::<DbPool>();           // 单例：返回缓存的 Arc
let logger: Arc<RequestLogger> = ctx.inject::<RequestLogger>();  // 原型：构造新实例

// 取走所有权（仅用于单例，会移除缓存）
let owned: AppServer = ctx.take::<AppServer>();

// 调试：打印所有注册的组件及其依赖
BuildContext::debug_registry();
```

## 关键设计决策

### 1. Scope 标记在被注入者上

组件自己声明是 Singleton 还是 Prototype，消费者只需要写 `Arc<T>`，框架自动处理。

### 2. 自动拓扑排序

`app!{}` 不指定组件列表时，会自动从 `COMPONENT_REGISTRY` 收集所有组件，进行拓扑排序后按依赖顺序注册。

### 3. 原型不预构建

`Scope::Prototype` 组件在初始化时**只注册工厂函数**，不立即构建实例，保证每次 `inject()` 都是全新实例。

### 4. `#[tx_cst(expr)]` 字段不进依赖图

宏解析字段时，有 `#[tx_cst]` 的字段不加入 `DEP_IDS`，不影响拓扑排序，也不要求对应类型在 ctx 中存在。

### 5. 并发安全

使用 `DashMap` 存储组件实例，支持多线程环境下的并发注入。

## 约束

| 约束 | 原因 |
|------|------|
| 组件需 `T: Send + Sync + 'static` | 存入 `Arc<dyn Any + Send + Sync>`，支持并发 |
| 组件需 `Clone`（推荐） | 便于在多个地方共享 |
| 无字段组件自动构建为 `Self {}` | 需要 struct 可默认构造 |
| `take()` 只能用于单例 | 原型组件没有缓存，无法 take |
| 避免循环依赖 | 框架会在运行时检测并 panic |

## 测试

```bash
cargo test
```

### 测试覆盖范围

项目包含 **20+ 个测试用例**，全面覆盖框架的核心功能：

#### 单例测试 (3个)
- `test_singleton_shared` - 验证单例在不同组件间共享
- `test_singleton_multiple_injects_same_instance` - 多次注入返回相同实例
- `test_singleton_arc_clone_shares_data` - Arc clone 只增加引用计数

#### 原型测试 (3个)
- `test_prototype_independent` - 验证原型实例相互独立
- `test_prototype_each_inject_creates_new_instance` - 每次注入创建新实例
- `test_prototype_with_custom_values` - 验证原型的自定义值注入

#### 自定义值注入测试 (3个)
- `test_inject_custom_values` - 验证 HashMap 和 String 注入
- `test_app_config_inject` - 验证函数调用和字面量注入
- `test_custom_value_expression_evaluated_once` - 验证表达式只求值一次

#### 依赖关系测试 (2个)
- `test_dependency_injection_chain` - 验证依赖注入链正确性
- `test_user_service_functionality` - 验证服务功能正常

#### 注册表测试 (2个)
- `test_registry` - 打印所有组件及依赖名称
- `test_scope_on_component` - 验证 scope 标记在组件自身

#### BuildContext API 测试 (3个)
- `test_build_context_len_and_empty` - 验证初始状态
- `test_build_context_after_initialization` - 验证初始化后状态
- `test_take_removes_from_context` - 验证 take 移除组件

#### 边界情况测试 (4个)
- `test_singleton_thread_safety` - 多线程环境下的单例安全性
- `test_prototype_state_isolation` - 原型实例状态隔离
- `test_component_with_no_dependencies` - 无依赖组件注入
- `test_component_with_multiple_dependencies` - 多依赖组件注入

#### 调试功能测试 (1个)
- `test_debug_registry_output` - 验证调试输出不 panic

### 运行示例

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_singleton_shared

# 运行测试并显示输出
cargo test -- --nocapture

# 运行测试并显示时间
cargo test -- --show-output
```
