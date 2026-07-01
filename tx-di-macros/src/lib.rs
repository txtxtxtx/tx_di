//! tx-di-macros — proc_macro 支持
//!
//! 提供 `#[derive(Component)]` 宏。
//! `#[tx_cst]` 和 `#[component]` 是 derive 辅助属性。

mod comp;
mod utils;

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
/// | `scope = Prototype` | 原型作用域 |
/// | `init` | 有自定义生命周期实现 |
/// | `conf` / `conf = "key"` | 配置组件 |
/// | `as_trait = dyn Trait` | Trait 实现注册 |
///
/// ## `#[tx_cst(...)]` — 字段属性
///
/// | 写法 | 语义 |
/// |------|------|
/// | `#[tx_cst(expr)]` | 用表达式赋值 |
/// | `#[tx_cst(skip)]` | 跳过，使用 Default |
///
/// # 示例
///
/// ```ignore
/// #[derive(Component)]
/// pub struct UserService {
///     repo: Arc<UserRepo>,
///     config: Arc<AppConfig>,
/// }
///
/// #[derive(Component)]
/// #[component(scope = Prototype)]
/// pub struct RequestContext { ... }
///
/// #[derive(Component)]
/// pub struct Logger {
///     #[tx_cst("info".to_string())]
///     level: String,
/// }
/// ```
#[proc_macro_derive(Component, attributes(component, tx_cst))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    comp::derive_component(input)
}
