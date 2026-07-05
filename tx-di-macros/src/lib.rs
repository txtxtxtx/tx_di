//! tx-di-macros — proc_macro 支持
//!
//! 提供 `#[derive(Component)]` 宏。
//! `#[tx_cst]` 和 `#[component]` 是 derive 辅助属性。
//!
//! # 模块结构
//!
//! - `attr` — 属性解析（`#[component(...)]`、`#[tx_cst(...)]`）
//! - `classify` — 字段分类（`FieldKind`）
//! - `codegen` — 代码生成（`impl Component` + linkme 注册条目）
//! - `type_utils` — 类型检测工具（`Arc<T>`、`Option<T>`、`Arc<dyn Trait>`）
//! - `name_utils` — 命名转换工具（驼峰 ↔ 蛇形）

mod attr;
mod classify;
mod codegen;
mod name_utils;
mod type_utils;

use proc_macro::TokenStream;

/// `#[derive(Component)]` — 组件 derive 宏
///
/// 为结构体自动生成 `Component` trait 实现和 `ComponentMeta` 注册条目。
///
/// # 辅助属性
///
/// ## `#[component(...)]` — 结构体属性
///
/// | 参数 | 说明 |
/// |------|------|
/// | `scope = Prototype` | 原型作用域（默认 `Singleton`） |
/// | `init` | 自定义 inner_init 回调（见下方生命周期表） |
/// | `app_init` | 自定义 init 回调 |
/// | `app_async_init` | 自定义 async_init 回调 |
/// | `app_async_run` | 自定义 async_run 回调 |
/// | `shutdown` | 自定义 shutdown 回调 |
/// | `conf` / `conf = "key"` | 配置组件 |
/// | `as_trait = dyn Trait` | Trait 实现注册 |
/// | `init_sort = N` | 初始化排序（值越小越先执行，默认 10000） |
///
/// ## `#[tx_cst(...)]` — 字段属性
///
/// | 写法 | 语义 |
/// |------|------|
/// | `#[tx_cst(expr)]` | 用表达式赋值 |
/// | `#[tx_cst(skip)]` | 跳过，使用 Default |
///
/// # 生命周期回调
///
/// 所有回调都是**可选的**——只有标记对应属性后才需要实现。不标记则使用 trait 默认实现。
///
/// 回调函数名与 `#[component(...)]` 属性名**保持一致**，便于记忆：
///
/// | `#[component(...)]` | 回调函数签名 | 覆写的 trait 方法 | 阶段 |
/// |---|---|---|---|
/// | `init` | `fn init(&mut self, store: &Store) -> RIE<()>` | `inner_init` | build 后、注册前 |
/// | `app_init` | `fn app_init(comp: Arc<Self>, app: &Arc<App>) -> RIE<()>` | `init` | 同步初始化 |
/// | `app_async_init` | `fn app_async_init(comp: Arc<Self>, app: &Arc<App>) -> BoxFuture<RIE<()>>` | `async_init` | 异步初始化 |
/// | `app_async_run` | `fn app_async_run(comp: Arc<Self>, app: &Arc<App>, token: CancellationToken) -> BoxFuture<RIE<()>>` | `async_run` | 后台运行 |
/// | `shutdown` | `fn shutdown(&self)` | `shutdown` | 优雅关闭 |
///
/// > **注意**：宏生成的覆写方法都带有 `#[inline]` 属性。如果回调函数为空或仅含简单逻辑，
/// > 编译器会直接内联消除调用开销。同时，生成的代码使用 `self::` 前缀调用回调，
/// > 即使 `init` / `shutdown` 与 trait 方法同名也不会冲突。
///
/// # 完整示例
///
/// ```ignore
/// use tx_di_core::{Component, App, Store, RIE, BoxFuture, CancellationToken};
/// use std::sync::Arc;
///
/// #[derive(Component)]
/// #[component(
///     init,                    // inner_init 回调
///     app_init,                // init 回调
///     app_async_init,          // async_init 回调
///     app_async_run,           // async_run 回调
///     shutdown                 // shutdown 回调
/// )]
/// pub struct DatabaseService {
///     pool: Arc<DbPool>,
/// }
///
/// // ── inner_init：build 后立即调用 ──
/// fn init(&mut self, store: &Store) -> RIE<()> {
///     // self 可写，可访问 store 做额外注入
///     Ok(())
/// }
///
/// // ── init：同步初始化阶段 ──
/// fn app_init(comp: Arc<Self>, app: &Arc<App>) -> RIE<()> {
///     // comp 是 Arc<Self>，可通过 comp.field 访问成员
///     tracing::info!("init: pool size = {}", comp.pool.size());
///     Ok(())
/// }
///
/// // ── async_init：异步初始化阶段 ──
/// fn app_async_init(comp: Arc<Self>, app: &Arc<App>) -> BoxFuture<RIE<()>> {
///     Box::pin(async move {
///         comp.pool.connect().await?;
///         Ok(())
///     })
/// }
///
/// // ── async_run：后台长期任务 ──
/// fn app_async_run(comp: Arc<Self>, app: &Arc<App>, token: CancellationToken) -> BoxFuture<RIE<()>> {
///     Box::pin(async move {
///         loop {
///             tokio::select! {
///                 _ = token.cancelled() => break,
///                 _ = comp.pool.health_check() => {},
///             }
///         }
///         Ok(())
///     })
/// }
///
/// // ── shutdown：优雅关闭 ──
/// fn shutdown(&self) {
///     self.pool.close();
/// }
/// ```
#[proc_macro_derive(Component, attributes(component, tx_cst))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    codegen::derive_component(input)
}
