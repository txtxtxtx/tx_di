# tx-di-macros

proc-macro crate，为 `tx-di-core` DI 框架提供 `#[derive(Component)]` 宏。

## 模块结构

```
tx-di-macros/src/
├── lib.rs              # proc_macro 入口 + 文档
├── attr/
│   ├── mod.rs          # re-export
│   ├── comp_attr.rs    # #[component(...)] 属性解析 + CompAttr / ScopeAttr 定义
│   └── field_attr.rs   # #[tx_cst(...)] 字段属性解析
├── classify/
│   ├── mod.rs
│   └── fields.rs       # FieldKind 枚举 + classify_fields 函数
├── codegen/
│   ├── mod.rs          # CodeGenContext + derive_component_impl 编排
│   ├── component_impl.rs   # 生成 impl Component for T
│   ├── factory.rs          # 生成 factory 闭包
│   ├── inner_init.rs       # 生成 inner_init 方法
│   └── meta_entry.rs       # 生成 linkme ComponentMeta 注册条目
├── type_utils.rs       # 类型检测工具（Arc/Option/Arc<dyn Trait>）
└── name_utils.rs       # 命名转换工具（驼峰 ↔ 蛇形）
```

### 数据流

```
属性解析 ──→ 字段分类 ──→ CodeGenContext ──→ 代码生成 ──→ 组装输出
(comp_attr)  (fields)                     (4个子模块)    (quote!)
```

### 各模块职责

| 模块 | 职责 | 对外 API |
|------|------|----------|
| `attr::comp_attr` | 解析 `#[component(...)]` 为 `CompAttr` | `parse_component_attr_from_attributes`, `CompAttr`, `ScopeAttr` |
| `attr::field_attr` | 解析 `#[tx_cst(expr)]` 和 `#[tx_cst(skip)]` | `extract_inject_expr`, `has_skip_attr` |
| `classify::fields` | 按类型形态对字段分类 | `classify_fields`, `FieldKind` |
| `codegen` | 编排代码生成 + 持有 `CodeGenContext` | `derive_component`（proc_macro 入口） |
| `codegen::component_impl` | 生成 `impl Component` | `gen_component_impl` |
| `codegen::factory` | 生成 factory 闭包 | `gen_factory_fn` |
| `codegen::inner_init` | 生成 `inner_init` 方法 | `gen_inner_init` |
| `codegen::meta_entry` | 生成 `linkme` 注册条目 | `gen_meta_entry` |
| `type_utils` | 类型检测工具函数 | `strip_arc_type`, `is_option_type`, `is_arc_dyn_trait`, `is_plain_arc_dyn_trait`, `extract_trait_from_arc`, `extract_trait_from_option_arc` |
| `name_utils` | 命名转换工具函数 | `camel_to_snake`, `camel_to_screaming_snake` |

---

## 使用方法

### 1. 基本使用

```rust
use tx_di_core::Component;

#[derive(Component)]
pub struct UserService {
    repo: Arc<UserRepo>,
    config: Arc<AppConfig>,
}
```

字段 `repo: Arc<UserRepo>` 会自动解析为 `UserRepo` 类型的组件依赖，从容器中注入。

### 2. 作用域

```rust
// 原型作用域：每次注入都创建新实例
#[derive(Component)]
#[component(scope = Prototype)]
pub struct RequestContext {
    request_id: String,
}
```

默认作用域为 `Singleton`（单例）。支持 `Singleton` 和 `Prototype` 两种。

### 3. 生命周期回调

每个生命周期都通过一个 `#[component(...)]` 标志和一个自定义函数实现：

| `#[component(...)]` | 回调函数签名 | 覆写 trait 方法 | 阶段 |
|---|---|---|---|
| `init` | `fn init(&mut self, store: &Store) -> RIE<()>` | `inner_init` | build 后 |
| `app_init` | `fn app_init(comp: Arc<Self>, app: &Arc<App>) -> RIE<()>` | `init` | 同步初始化 |
| `app_async_init` | `fn app_async_init(comp: Arc<Self>, app: &Arc<App>) -> BoxFuture<RIE<()>>` | `async_init` | 异步初始化 |
| `app_async_run` | `fn app_async_run(comp: Arc<Self>, app: &Arc<App>, token: CancellationToken) -> BoxFuture<RIE<()>>` | `async_run` | 后台运行 |
| `shutdown` | `fn shutdown(&self)` | `shutdown` | 优雅关闭 |

```rust
use tx_di_core::{Component, App, Store, RIE, BoxFuture, CancellationToken};
use std::sync::Arc;

#[derive(Component)]
#[component(init, app_init, app_async_run, shutdown)]
pub struct DatabaseService {
    pool: Arc<DbPool>,
}

fn init(&mut self, store: &Store) -> RIE<()> {
    Ok(())
}

fn app_init(comp: Arc<Self>, app: &Arc<App>) -> RIE<()> {
    println!("connected: {}", comp.pool.is_connected());
    Ok(())
}

fn app_async_run(comp: Arc<Self>, app: &Arc<App>, token: CancellationToken) -> BoxFuture<RIE<()>> {
    Box::pin(async move {
        loop { tokio::select! { _ = token.cancelled() => break, } }
        Ok(())
    })
}

fn shutdown(&self) {
    self.pool.close();
}
```

### 4. 配置组件

### 4. Trait 实现注册

```rust
#[derive(Component)]
#[component(as_trait = dyn UserRepository)]
pub struct UserRepoImpl {
    db: Arc<DbPool>,
}
```

通过 trait object 按接口注入：

```rust
#[derive(Component)]
pub struct UserService {
    // Arc<dyn Trait> — 必选 trait 注入
    repo: Arc<dyn UserRepository>,

    // Option<Arc<dyn Trait>> — 可选 trait 注入
    cache: Option<Arc<dyn CacheProvider>>,
}
```

### 5. 生命周期钩子

```rust
#[derive(Component)]
#[component(init)]
pub struct DatabaseService {
    pool: Arc<DbPool>,
}

impl DatabaseService {
    fn __di_component_init(&mut self, store: &Store) -> RIE<()> {
        // 自定义初始化逻辑，在 build 完成后、正式使用前调用
        Ok(())
    }
}
```

### 6. 初始化排序

```rust
use tx_di_core::Component;

#[derive(Component)]
#[component(init_sort = -2147483648)] // 最小整数 → 最先初始化
pub struct DatabaseMigrator {
    db: Arc<DbPool>,
}
```

默认排序值为 `10000`。值越小越先执行。通常核心基础设施使用负值。

### 7. 字段级自定义

```rust
#[derive(Component)]
pub struct Logger {
    // #[tx_cst(expr)] — 用表达式赋值，不从 DI 容器注入
    #[tx_cst("info".to_string())]
    level: String,

    // #[tx_cst(skip)] — 跳过注入，使用 Default::default()
    #[tx_cst(skip)]
    buffer: Vec<String>,

    // Option<T> — 注入时不赋值，保持 None
    fallback: Option<Arc<FallbackLogger>>,
}
```

### 8. 依赖注入规则总结

| 字段类型 | 注入行为 |
|----------|----------|
| `Arc<T>` | 普通组件注入，从容器获取 `T` |
| `Arc<dyn Trait>` | 必选 trait 注入，`inner_init` 中填充 |
| `Option<Arc<dyn Trait>>` | 可选 trait 注入，找不到时保持 `None` |
| `Option<T>` | 可选普通依赖，保持 `None` |
| `#[tx_cst(expr)]` | 表达式赋值 |
| `#[tx_cst(skip)]` | 跳过，使用 `Default::default()` |

---

## 开发指南

### 添加新的 `#[component(...)]` 参数

1. 在 `attr/comp_attr.rs` 的 `CompAttr` 结构体添加字段
2. 在 `CompAttrArgs` 添加对应字段
3. 在 `parse()` 方法中添加解析分支
4. 在 `codegen/` 相应子模块中使用新字段生成代码

### 添加新的字段注入类型

1. 在 `classify/fields.rs` 的 `FieldKind` 添加变体
2. 在 `classify_fields()` 中添加匹配规则
3. 在 `codegen/component_impl.rs` 的 `build_fields` 映射中添加处理
4. 如有需要，在 `codegen/inner_init.rs` 或 `codegen/meta_entry.rs` 中添加相应处理

### 测试

```bash
# 运行全部测试
cargo test -p tx-di-core

# 仅编译宏（快速检查）
cargo build -p tx-di-macros
```

核心测试位于 `tx-di-core/tests/test_component.rs`，覆盖宏的各条代码生成路径。
