
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    Attribute, Expr, GenericArgument, Ident, ItemStruct, Path, PathArguments, Result as SynResult,
    Token, Type,
};

const TX_CST: &str = "tx_cst";
// ─────────────────────────────────────────────────────────────────────────────
// 辅助函数
// ─────────────────────────────────────────────────────────────────────────────

/// 从字段属性中提取 `#[tx_cst(expr)]` 的表达式
fn extract_inject_expr(attrs: &[Attribute]) -> SynResult<Option<Expr>> {
    for attr in attrs {
        if attr.path().is_ident(TX_CST) {
            let expr: Expr = attr.parse_args()?;
            return Ok(Some(expr));
        }
    }
    Ok(None)
}

/// 从 #[component(scope = Singleton/Prototype)] 属性解析 scope
#[derive(Debug, Clone, PartialEq)]
enum ScopeAttr {
    Singleton,
    Prototype,
}

/// 解析 `#[component]` 属性的作用域参数
///
/// 从过程宏的属性令牌流中解析出组件的作用域类型。支持以下语法：
/// - `#[component]` - 无参数时默认为 Singleton
/// - `#[component(scope = Singleton)]` - 显式指定单例作用域
/// - `#[component(scope = Prototype)]` - 指定原型作用域（每次获取创建新实例）
///
/// # 参数
///
/// * `attr_tokens` - 宏属性中的令牌流，即 `#[component(...)]` 括号内的内容
///
/// # 返回值
///
/// * `Ok(ScopeAttr::Singleton)` - 解析成功，返回单例作用域
/// * `Ok(ScopeAttr::Prototype)` - 解析成功，返回原型作用域
/// * `Err(syn::Error)` - 解析失败，包含详细的错误信息和源码位置
///
/// # 错误情况
///
/// * 提供了非 `scope` 的参数名
/// * `scope` 的值不是 `Singleton` 或 `Prototype`
/// * 语法格式不正确（缺少等号等）
fn parse_component_attr(attr_tokens: TokenStream) -> SynResult<ScopeAttr> {
    if attr_tokens.is_empty() {
        return Ok(ScopeAttr::Singleton);
    }

    struct ScopeKv {
        value: Expr,
    }
    impl Parse for ScopeKv {
        fn parse(input: ParseStream) -> SynResult<Self> {
            let key: Ident = input.parse()?;
            if key != "scope" {
                return Err(syn::Error::new_spanned(
                    key,
                    "#[tx_comp] 只支持 scope 参数，例如：#[tx_comp(scope = Prototype)]",
                ));
            }
            let _eq: Token![=] = input.parse()?;
            let value: Expr = input.parse()?;
            Ok(ScopeKv { value })
        }
    }

    let kv: ScopeKv = syn::parse(attr_tokens)?;
    // 解析 scope 值：支持裸 Ident（Prototype）和完整路径（di_core::Scope::Prototype）
    let ident_str = match &kv.value {
        Expr::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        _ => kv.value.to_token_stream().to_string(),
    };
    match ident_str.as_str() {
        "Singleton" => Ok(ScopeAttr::Singleton),
        "Prototype" => Ok(ScopeAttr::Prototype),
        other => Err(syn::Error::new_spanned(
            &kv.value,
            format!("未知的 scope `{}`，只支持 Singleton 或 Prototype", other),
        )),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. #[component] 宏
// ─────────────────────────────────────────────────────────────────────────────

/// 组件宏,标注一个结构体为组件
/// ```rust,ignore
/// #[tx_comp] // 默认 Singleton,可选 Prototype
/// pub struct DbPool { ... }
///
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
pub fn tx_comp(attr: TokenStream, item: TokenStream) -> TokenStream {
    // 解析作用域参数
    let scope_attr = match parse_component_attr(attr) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };
    let input = parse_macro_input!(item as ItemStruct);
    match component_impl(scope_attr, input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
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
fn component_impl(scope_attr: ScopeAttr, input: ItemStruct) -> SynResult<TokenStream2> {
    let struct_name = &input.ident;
    let vis = &input.vis;

    // ── 解析字段 ──────────────────────────────────────────────────────────

    /// 字段的注入方式
    enum FieldKind {
        /// 从 ctx 注入：统一使用 ctx.inject::<T>()，返回 Arc<T>
        Inject { ty: Type },
        /// #[inject(expr)]：直接用表达式，不计入依赖图
        Custom { expr: Expr },
    }

    let mut fields_info: Vec<(syn::Ident, FieldKind)> = Vec::new();

    // 过滤掉 #[inject(...)] 属性（避免 rustc 报错）
    let mut clean_fields = input.fields.clone();
    for field in &mut clean_fields {
        field.attrs.retain(|a| !a.path().is_ident(TX_CST));
    }

    for field in &input.fields {
        let ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(&field.ty, "#[tx_comp] 只支持具名字段"))?;

        let inject_expr = extract_inject_expr(&field.attrs)?;
        let kind = if let Some(expr) = inject_expr {
            FieldKind::Custom { expr }
        } else {
            // 裸字段（Arc<T> 或 T），统一走 ctx.inject::<T>()
            FieldKind::Inject {
                ty: field.ty.clone(),
            }
        };

        fields_info.push((ident.clone(), kind));
    }

    // ── 生成 build() 字段赋值 ─────────────────────────────────────────────

    let build_fields: Vec<TokenStream2> = fields_info
        .iter()
        .map(|(fname, kind)| {
            match kind {
                FieldKind::Inject { ty } => {
                    // inject_ty 是传给 ctx.inject::<T>() 的类型。
                    // 如果字段类型是 Arc<T>，inject::<T>() 正好返回 Arc<T>，
                    // 不需要额外解包。只需要传入 "去掉 Arc<> 的那层"。
                    let inject_ty = strip_arc(ty);
                    quote! {
                        #fname: ctx.inject::<#inject_ty>()
                    }
                }
                FieldKind::Custom { expr } => {
                    quote! { #fname: #expr }
                }
            }
        })
        .collect();

    // ── 生成 DEP_IDS ─────────────────────────────────────────────────────

    let dep_type_ids: Vec<TokenStream2> = fields_info
        .iter()
        .filter_map(|(_, kind)| {
            match kind {
                FieldKind::Inject { ty } => {
                    let inject_ty = strip_arc(ty);
                    Some(quote! { || ::std::any::TypeId::of::<#inject_ty>() })
                }
                FieldKind::Custom { .. } => None, // #[inject] 不计入依赖图
            }
        })
        .collect();

    // ── scope ─────────────────────────────────────────────────────────────

    let scope_const = match scope_attr {
        ScopeAttr::Singleton => quote! { ::di_core::Scope::Singleton },
        ScopeAttr::Prototype => quote! { ::di_core::Scope::Prototype },
    };

    let meta_ident = format_ident!(
        "__DI_META_{}",
        camel_to_screaming_snake(&struct_name.to_string())
    );

    // 重新构造结构体（去掉 #[inject] 属性）
    let clean_input = ItemStruct {
        fields: clean_fields,
        ..input.clone()
    };

    let output = quote! {
        // ── 原始结构体定义（已去掉 #[inject] 属性） ───────────────────────
        #clean_input

        // ── ComponentDescriptor impl ──────────────────────────────────────
        impl ::di_core::ComponentDescriptor for #struct_name {
            const DEP_IDS: &'static [fn() -> ::std::any::TypeId] = &[
                #( #dep_type_ids ),*
            ];

            const SCOPE: ::di_core::Scope = #scope_const;

            fn build(ctx: &mut ::di_core::BuildContext) -> Self {
                Self {
                    #( #build_fields ),*
                }
            }
        }

        // ── linkme 注册条目 ───────────────────────────────────────────────
        // factory 返回 Box<T>，di-core 的 call_factory 内部包 Arc<T>
        #[::di_core::linkme::distributed_slice(::di_core::COMPONENT_REGISTRY)]
        #[linkme(crate = ::di_core::linkme)]
        #[allow(non_upper_case_globals)]
        #vis static #meta_ident: ::di_core::ComponentMeta = ::di_core::ComponentMeta {
            type_id: || ::std::any::TypeId::of::<#struct_name>(),
            deps: &[ #( #dep_type_ids ),* ],
            name: ::std::stringify!(#struct_name),
            scope: #scope_const,
            factory_fn: Some(|ctx: &mut ::di_core::BuildContext| {
                ::std::boxed::Box::new(
                    <#struct_name as ::di_core::ComponentDescriptor>::build(ctx)
                )
            }),
        };
    };

    Ok(output)
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. app!{} 宏
// ─────────────────────────────────────────────────────────────────────────────

struct AppInput {
    module_name: Ident,
    components: Vec<Path>,
}

impl Parse for AppInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let module_name: Ident = input.parse()?;

        let components = if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            if content.is_empty() {
                vec![]
            } else {
                let components: Punctuated<Path, Comma> =
                    content.parse_terminated(Path::parse, Token![,])?;
                components.into_iter().collect()
            }
        } else {
            vec![]
        };

        Ok(AppInput {
            module_name,
            components,
        })
    }
}

/// 声明一个 DI 模块，生成 `build_<module_name>()` 初始化函数。
///
/// case:
/// ```rust.ignore
/// app!{
///   MyModule
///   [xxx,xxx] // 可以没有，没有自动扫描
/// }
///
#[proc_macro]
pub fn app(input: TokenStream) -> TokenStream {
    let AppInput {
        module_name,
        components,
    } = parse_macro_input!(input as AppInput);

    match app_impl(module_name, components) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn app_impl(module_name: Ident, components: Vec<Path>) -> SynResult<TokenStream2> {
    let fn_name = {
        let snake = camel_to_snake(&module_name.to_string());
        format_ident!("build_{}", snake, span = Span::call_site())
    };

    let component_count = components.len();

    // 为每个组件生成 register_factory 调用
    // 注册组件：factory_fn 用 FactoryFnBox 包装
    let build_stmts: Vec<TokenStream2> = components
        .iter()
        .map(|ty| {
            quote! {
                ctx.register_factory::<#ty>(
                    <#ty as ::di_core::ComponentDescriptor>::SCOPE,
                    |ctx: &mut ::di_core::BuildContext| {
                        ::std::boxed::Box::new(
                            <#ty as ::di_core::ComponentDescriptor>::build(ctx)
                        )
                    },
                );
            }
        })
        .collect();

    let output = quote! {
        // 由 `app!{}` 宏自动生成的初始化函数。
        #[allow(non_snake_case, dead_code)]
        pub fn #fn_name() -> ::di_core::BuildContext {
            let mut ctx = ::di_core::BuildContext::new();
                        if #component_count == 0 {
                // 获取所有注册的组件
                let metas: ::std::vec::Vec<&::di_core::ComponentMeta> = ::di_core::COMPONENT_REGISTRY.iter().collect();
                let sorted_ids = ::di_core::topo_sort(&metas);

                for tid in &sorted_ids {
                    if let Some(meta) = metas.iter().find(|m| (m.type_id)() == *tid) {
                        if let Some(factory_fn) = meta.factory_fn {
                            ctx.register_factory_boxed((meta.type_id)(), meta.scope, factory_fn);
                        }
                    }
                }
            } else {
                #( #build_stmts )*
            }
            ctx
        }
    };

    Ok(output)
}

// ─────────────────────────────────────────────────────────────────────────────
// 辅助
// ─────────────────────────────────────────────────────────────────────────────

/// 如果 `ty` 是 `Arc<T>`，返回 T；否则返回 ty 本身。
fn strip_arc(ty: &Type) -> TokenStream2 {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => return quote! { #ty },
    };
    let segs = &path.segments;
    if segs.len() == 1 && segs[0].ident == "Arc" {
        if let PathArguments::AngleBracketed(ab) = &segs[0].arguments {
            if ab.args.len() == 1 {
                if let GenericArgument::Type(inner) = &ab.args[0] {
                    return quote! { #inner };
                }
            }
        }
    }
    quote! { #ty }
}

/// 将驼峰命名法字符串转换为蛇形命名法。
///
/// 在大写字母前插入下划线，并将所有字符转换为小写。
/// 第一个字符前不插入下划线。
///
/// # 参数
///
/// * `s` - 输入的驼峰命名法字符串
///
/// # 返回值
///
/// 转换后的蛇形命名法字符串
///
/// case `DbPool` -> `db_pool`
fn camel_to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        // 在非首字符的大写字母前插入下划线
        if ch.is_uppercase() && i != 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

/// 将驼峰命名法字符串转换为大写蛇形命名法（SCREAMING_SNAKE_CASE）。
///
/// 先转换为蛇形命名法，再将所有字符转为大写。
/// 常用于生成常量名或静态变量名。
///
/// # 参数
///
/// * `s` - 输入的驼峰命名法字符串
///
/// # 返回值
///
/// 转换后的大写蛇形命名法字符串
fn camel_to_screaming_snake(s: &str) -> String {
    camel_to_snake(s).to_uppercase()
}
