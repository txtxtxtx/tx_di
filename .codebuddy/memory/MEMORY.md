# tx_di 项目长期记忆

## 项目结构
- Rust 工作区，DI（依赖注入）框架
- `tx-di-macros`：proc-macro crate，提供 `#[derive(Component)]` 宏
- `tx-di-core`：核心运行时 crate（Component trait、Store、App、生命周期、拓扑排序、AOP）
- `common/`：通用工具 crate（tx_common、tx_error 等）
- `plugins/`：插件 crate（如 tx_di_log）
- `examples/`：示例应用（tx_admin 等）

## tx-di-macros 模块结构（2026-07-04 重构后）
原 `comp.rs`(703行单文件) + `utils.rs` 拆分为职责清晰的多模块：
- `attr/` — 属性解析（`comp_attr.rs` 解析 `#[component(...)]`，`field_attr.rs` 解析 `#[tx_cst(...)]`）
- `classify/fields.rs` — 字段分类 `FieldKind` 枚举
- `codegen/` — 代码生成（`mod.rs` 编排 + `CodeGenContext`，`component_impl.rs`、`factory.rs`、`inner_init.rs`、`meta_entry.rs`）
- `type_utils.rs` — 类型检测（Arc/Option/Arc<dyn Trait>）
- `name_utils.rs` — 命名转换（camel_to_snake 等）

数据流：属性解析 → 字段分类 → 构建 CodeGenContext → 各 codegen 子模块生成片段 → 组装

## 已知问题
- `examples/` 中部分 crate 引用 `tx_di_core::tx_comp`（不存在的宏），属预先存在的错误，与 Component derive 宏无关

## 测试
- `cargo test -p tx-di-core` 含 64 个测试覆盖宏全部功能路径
- 插件测试在各自 crate 中

## 框架 API 迁移（2026-07-05）

### 旧 API → 新 API
- `#[tx_comp(conf, init)]` → `#[derive(Component)] #[component(conf = "key", init)]`
- `#[tx_comp(init)]` → `#[derive(Component)] #[component(init)]`
- `impl CompInit for T { fn inner_init(...) }` → `fn init(this: &mut T, _store: &Store) -> RIE<()>`（模块级自由函数）
- `impl CompInit for T { async_method!(fn async_init_impl(ctx, token) { ... }) }` → `#[component(app_async_init)]` + `async fn app_async_init(comp: Arc<T>, app: Arc<App>) -> RIE<()>`
- `impl CompInit for T { async_method!(fn async_run_impl(ctx, token) { ... }) }` → `#[component(app_async_run)]` + `async fn app_async_run(comp: Arc<T>, app: Arc<App>, token: CancellationToken) -> RIE<()>`
- `impl CompInit for T { fn init_sort() -> i32 { N } }` → `#[component(init_sort = N)]`
- `InnerContext` → `Store`（init 回调参数）
- `ctx.inject::<T>()` → `comp.field`（直接访问字段）或 `app.inject::<T>()`

### 保留不变
- `#[tx_cst(expr)]` 字段自定义构造器仍受支持
- `RIE<T>` 类型别名不变
- `async_method!` 宏已移除，`BoxFuture` 包装由 `#[derive(Component)]` 生成代码自动处理

### 回调函数签名

| 属性 | 回调签名 |
|------|---------|
| `init` | `fn init(this: &mut T, _store: &Store) -> RIE<()>` |
| `app_init` | `fn app_init(comp: Arc<T>, app: &Arc<App>) -> RIE<()>` |
| `app_async_init` | `async fn app_async_init(comp: Arc<T>, app: Arc<App>) -> RIE<()>` |
| `app_async_run` | `async fn app_async_run(comp: Arc<T>, app: Arc<App>, token: CancellationToken) -> RIE<()>` |
| `shutdown` | `fn shutdown(&self)` |

### 注意事项
- `DepsTuple` 必须在所有使用 `#[derive(Component)]` 有依赖的模块中 `use tx_di_core::DepsTuple`
- `app` 参数在异步回调中必须是 `Arc<App>`（非 `&Arc<App>`），因为 `BoxFuture` 要求 `'static`
- 异步回调直接写 `async fn`，无需 `async_method!` 或手动 `Box::pin`
