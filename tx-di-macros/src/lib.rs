mod comp;
mod utils;

use proc_macro::TokenStream;

/// 组件宏,标注一个结构体为组件
///
/// ```rust,ignore
/// #[tx_comp] // 默认 单例,无自定义初始化方法，不是配置组件
/// pub struct DbPool { ... }
///
/// #[tx_comp(scope,init)] init 表示有自定义的初始化方法 只有 scope 表示原型，不是配置组件
/// pub struct XxxServer {
///     db: Arc<DbPool>, // 自动注入
///     #[tx_cst(build_count())] // 自定义值
///     count: u32,
/// }
///
/// fn build_count() -> u32 {
///     0
/// }
/// #[tx_comp(conf)] //表示是配置组件，自动从配置文件加载配置，配置文件路径为 configs / app.toml
/// pub struct AppConfig {
///     port: u16,
///     addr: String,
/// }
#[proc_macro_attribute]
pub fn tx_comp(attr: TokenStream, item: TokenStream) -> TokenStream{
    comp::tx_comp(attr, item)
}

/// 自定义值宏
///
/// 调用自定义方法生成自定义值
/// ```rust,ignore
/// #[tx_comp(scope = Prototype)]
/// pub struct XxxServer {
///     db: <DbPool>, // 自动注入
///     #[tx_cst(build_count())] // 自定义值
///     count: u32,
/// }
///
/// fn build_count() -> u32 {
///     0
/// }
#[proc_macro_attribute]
pub fn tx_cst(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // 空操作：直接返回原始项，不做任何修改
    item
}
