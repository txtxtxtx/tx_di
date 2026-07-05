---
name: plugin-framework-update
overview: 将 tx_di_toasty、tx_di_sa_token、tx_di_job、tx_di_file、tx_di_axum 五个插件从旧的 `#[tx_comp(...)]` + `CompInit` 模式迁移到新的 `#[derive(Component)]` + `#[component(...)]` + 自由函数回调模式
todos:
  - id: migrate-tx-di-file
    content: 迁移 tx_di_file：FileConfig（conf+init）、FilePlugin（init+init_sort+async_init）
    status: completed
  - id: migrate-tx-di-sa-token
    content: 迁移 tx_di_sa_token：SaTokenConf（conf+init）、SaTokenPlugin（init+init_sort）
    status: completed
  - id: migrate-tx-di-axum
    content: 迁移 tx_di_axum：WebConfig（conf+init）、WebPlugin（init+async_init+async_run）
    status: completed
  - id: migrate-tx-di-toasty
    content: 迁移 tx_di_toasty：ToastyConfig（conf+init）、ToastyPlugin（init+async_init）
    status: completed
  - id: migrate-tx-di-job
    content: 迁移 tx_di_job：JobConfig（conf+init）、JobPlugin（init+async_init+async_run）
    status: completed
    dependencies:
      - migrate-tx-di-toasty
  - id: full-workspace-verify
    content: cargo check --workspace 完整编译验证
    status: completed
    dependencies:
      - migrate-tx-di-file
      - migrate-tx-di-sa-token
      - migrate-tx-di-axum
      - migrate-tx-di-toasty
      - migrate-tx-di-job
---

将 5 个插件从旧 API 迁移到新框架 API。旧 API 使用 `#[tx_comp(...)]` 属性宏 + `impl CompInit for T` trait 实现；新 API 使用 `#[derive(Component)]` + `#[component(...)]` 属性 + 自由函数回调。参考已更新的 `tx_di_log` 插件。

## 迁移清单

### 需修改的文件（共 5 个插件，10-12 个源文件）

| 插件 | 需修改源文件 |
| --- | --- |
| tx_di_file | `config.rs` (FileConfig), `plugin.rs` (FilePlugin) |
| tx_di_sa_token | `config.rs` (SaTokenConf), `plugin.rs` (SaTokenPlugin) |
| tx_di_axum | `comp.rs` (WebPlugin), `config.rs` (WebConfig) |
| tx_di_toasty | `config.rs` (ToastyConfig), `plugin.rs` (ToastyPlugin) |
| tx_di_job | `config.rs` (JobConfig), `comp.rs` (JobPlugin) |


### 迁移模式对照

| 旧代码 | 新代码 |
| --- | --- |
| `#[tx_comp(conf, init)]` | `#[derive(Component)] #[component(conf, init)]` |
| `#[tx_comp(init)]` | `#[derive(Component)] #[component(init)]` |
| `impl CompInit for T { fn inner_init(&mut self, _ctx: &InnerContext) -> RIE<()> { ... }` | `fn init(this: &mut T, _store: &Store) -> RIE<()> { ... }`（自由函数） |
| `impl CompInit for T { fn init_sort() -> i32 { N } }` | `#[component(init_sort = N)]` |
| `tx_di_core::async_method!(fn async_init_impl(ctx: Arc<App>, _token) { ... })` | `#[component(app_async_init)]` + `fn app_async_init(comp: Arc<T>, app: &Arc<App>) -> BoxFuture<RIE<()>> { Box::pin(async move { ... }) }` |
| `tx_di_core::async_method!(fn async_run_impl(ctx: Arc<App>, token) { ... })` | `#[component(app_async_run)]` + `fn app_async_run(comp: Arc<T>, app: &Arc<App>, token: CancellationToken) -> BoxFuture<RIE<()>> { Box::pin(async move { ... }) }` |
| `ctx.inject::<T>()` (from impl block) | `comp.field` (直接访问字段) 或 `app.store.try_inject::<T>()` |


## 技术方案

### 迁移策略

每个插件依次进行以下标准化迁移：

1. **属性宏替换**：`#[tx_comp(conf, init)]` → `#[derive(Component)] #[component(conf, init)]`
2. **`CompInit` trait 实现移除**：删除整个 `impl CompInit for T { ... }` 块
3. **`inner_init` → `init` 自由函数**：将 `fn inner_init(&mut self, _ctx: &InnerContext)` 改为同模块中的 `fn init(this: &mut T, _store: &Store)` 自由函数
4. **`init_sort` → 属性**：将 `fn init_sort() -> i32 { N }` 移到 `#[component(init_sort = N)]` 属性
5. **`async_method!` 宏替换**：将 `tx_di_core::async_method!(fn async_init_impl(ctx, token) { ... })` 拆分为：

- 属性 `#[component(app_async_init)]`
- 自由函数 `fn app_async_init(comp: Arc<T>, app: &Arc<App>) -> BoxFuture<RIE<()>>`
- 函数体中的 `ctx.inject::<T>()` 替换为 `comp`（已注入）、`ctx.inject::<Dep>()` 替换为 `app.store.try_inject::<Dep>()`

6. **`async_run_impl` → `app_async_run`**：类似模式，附加 `token` 参数
7. **删除废弃导入**：移除 `CompInit`, `InnerContext`, `async_method`, `tx_comp` 导入

### 保留不变的

- `#[tx_cst(expr)]` 字段自定义构造器 — 新框架仍然支持
- 配置文件的 TOML key（使用默认 snake_case，无需指定 `conf = "key"`）
- `RIE` 类型别名
- `App::inject` 和 `Store` API

### 执行顺序

1. `tx_di_file` — 最简单，只有 `init` + `init_sort`，无异步初始化
2. `tx_di_sa_token` — 只有 `init` + `init_sort`，无异步化
3. `tx_di_axum` — 最复杂，有 `async_init_impl` + `async_run_impl`，需迁移为 `app_async_init` + `app_async_run`
4. `tx_di_toasty` — 有 `init` + `async_init_impl`，需迁移为 `init` + `app_async_init`；`ctx.inject::<T>()` → `comp.field` / `app.store.try_inject::<T>()`
5. `tx_di_job` — 最复杂，有 `init` + `async_init_impl` + `async_run_impl`；依赖 `tx_di_toasty` 更新完成

### 验证方式

每个插件更新后执行 `cargo check -p <plugin>` 确认编译通过。所有插件更新后执行 `cargo check --workspace` 确认无回归。

## Agent 扩展使用

### code-explorer (SubAgent)

- **用途**：在每个插件修改前，使用 SubAgent 确认目标文件的精确内容，避免替换错误
- **预期产出**：准确的旧代码内容和上下文，确保搜索替换精确
- **使用阶段**：每个插件的修改步骤开始前

### rust-ddd-test-generator (Skill)

- **用途**：在 tx_di_axum 和 tx_di_job 迁移完成后，生成端到端集成测试用例验证生命周期钩子
- **预期产出**：针对 `app_async_init` 和 `app_async_run` 的测试模板
- **使用阶段**：tx_di_axum 和 tx_di_job 迁移完成后