use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    Attribute, Expr, Ident, ItemStruct, Result as SynResult,
    Token, Type,
};

use crate::utils::{camel_to_screaming_snake, camel_to_snake, is_option_type, strip_arc};

/// 组件宏,标注一个结构体为组件
/// ```rust,ignore
/// #[tx_comp] // 默认 Singleton,可选 Prototype
/// pub struct DbPool { ... }
///
/// #[tx_comp(scope,init)] init 表示有自定义的初始化方法 只有 scope 表示原型
/// pub struct XxxServer {
///     db: <DbPool>, // 自动注入
///     #[tx_cst(build_count())] // 自定义值
///     count: u32,
/// }
///
/// fn build_count() -> u32 {
///     0
/// }
pub fn tx_comp(attr: TokenStream, item: TokenStream) -> TokenStream {
    // 解析作用域参数
    let comp_attr = match parse_component_attr(attr) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };
    let input = parse_macro_input!(item as ItemStruct);
    match component_impl(comp_attr, input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn component_impl(comp_attr: CompAttr, input: ItemStruct) -> SynResult<TokenStream2> {
    let struct_name = &input.ident;
    let vis = &input.vis;

    /// 字段的注入方式
    enum FieldKind {
        /// 从 ctx 注入：统一使用 ctx.inject::<T>()，返回 Arc<T>
        Inject { ty: Type },
        /// #[inject(expr)]：直接用表达式，不计入依赖图
        Custom { expr: Expr },
        /// ctx 获取不到注入 None
        Optional { ty: Type },
        /// 不自动注入，用户手动创建该字段或是使用默认值
        Skip,
    }

    let mut fields_info: Vec<(syn::Ident, FieldKind)> = Vec::new();

    // 过滤掉 #[tx_cst(...)] 属性（避免 rustc 报错）
    let mut clean_fields = input.fields.clone();
    for field in &mut clean_fields {
        field.attrs.retain(|a| !a.path().is_ident(TX_CST));
    }

    for field in &input.fields {
        let ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(&field.ty, "#[tx_cst] 只支持具名字段"))?;
        // 优先级 skip > optional > custom > inject
        let inject_expr = extract_inject_expr(&field.attrs)?;
        let kind = if has_skip_attr(&field.attrs) {
            FieldKind::Skip
        }
        else if is_option_type(&field.ty) {
            // Option<T> 类型字段，使用可选注入
            // let inner_ty = extract_option_inner(&field.ty);
            FieldKind::Optional {
                ty: field.ty.clone(),
                // ty: inner_ty.unwrap_or_else(|| field.ty.clone()),
            }
        }
        else if let Some(expr) = inject_expr {
            FieldKind::Custom { expr }
        }
        else {
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
                FieldKind::Skip => {
                    quote! {
                        #fname: Default::default()
                    }
                }
                FieldKind::Optional { .. } => {
                    quote! {
                        #fname: None
                    }
                }
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

    let mut dep_type_ids: Vec<TokenStream2> = fields_info
        .iter()
        .filter_map(|(_, kind)| {
            match kind {
                FieldKind::Inject { ty } => {
                    let inject_ty = strip_arc(ty);
                    Some(quote! { || ::std::any::TypeId::of::<#inject_ty>() })
                }
                FieldKind::Custom { .. } => None, // 自定义 不计入依赖图
                FieldKind::Optional {..} => None, // Option 回注入None 不计入依赖图
                FieldKind::Skip => None, // 不自动注入，使用默认值 不计入依赖图
            }
        })
        .collect();

    // ── scope ─────────────────────────────────────────────────────────────

    let scope_const = match &comp_attr.scope {
        ScopeAttr::Singleton => quote! { ::tx_di_core::Scope::Singleton },
        ScopeAttr::Prototype => quote! { ::tx_di_core::Scope::Prototype },
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


    let comp_init_impl = if comp_attr.has_init {
        // 用户自己写，宏不生成
        quote! {}
    }
    else {
        // 宏生成默认空实现
        quote! {
            impl ::tx_di_core::CompInit for #struct_name {}
        }
    };

    let conf_build_code = if let Some(conf_option) = &comp_attr.conf {
        // 这是一个配置组件，需要从 AppAllConfig 中读取配置
        let config_key = if let Some(custom_key) = conf_option {
            // 使用自定义配置键
            quote! { #custom_key }
        } else {
            // 使用结构体名转蛇形
            let snake_name = camel_to_snake(&struct_name.to_string());
            quote! { #snake_name }
        };
        // 是配置组件就移除依赖,todo 目前配置不能依赖组件
        dep_type_ids = vec![];
        // 生成从配置反序列化的代码
        Some(quote! {
            fn build(ctx: &mut ::tx_di_core::BuildContext) -> Self {
                let app_config = ctx.inject::<::tx_di_core::AppAllConfig>();
                // 尝试从配置中获取，如果不存在则反序列化空表以触发 serde 默认值
                let mut config = if let Some(value) = app_config.get_value(#config_key) {
                    // 配置存在，正常反序列化
                    <Self as ::serde::Deserialize>::deserialize(value.clone())
                        .unwrap_or_else(|e| {
                            eprintln!("[di] 警告：配置 '{}' 解析失败: {}，使用 serde 默认值", #config_key, e);
                            // 反序列化空表以触发 serde 默认函数
                            let empty_table = ::tx_di_core::Value::Table(::tx_di_core::map::Map::new());
                            <Self as ::serde::Deserialize>::deserialize(empty_table)
                                .expect("[di] 错误：没有实现默认方法，没有默认值")
                        })
                } else {
                    // 配置键不存在，反序列化空表以触发 serde 默认函数
                    let empty_table = ::tx_di_core::Value::Table(::tx_di_core::map::Map::new());
                    <Self as ::serde::Deserialize>::deserialize(empty_table)
                        .expect("[di] 错误：没有实现默认方法，没有默认值")
                };
                <Self as ::tx_di_core::CompInit>::inner_init(&mut config,ctx);
                config
            }
        })
    } else {
        // 非配置组件，使用正常的字段注入逻辑
        None
    };

    let build_impl = if let Some(conf_code) = conf_build_code {
        conf_code
    }
    else {
        // 原有的 build 实现
        quote! {
            fn build(ctx: &mut ::tx_di_core::BuildContext) -> Self {
                let mut comp = Self {
                    #( #build_fields ),*
                };
                <Self as ::tx_di_core::CompInit>::inner_init(&mut comp, ctx);
                comp
            }
        }
    };

    let output = quote! {
        // ── 原始结构体定义（已去掉 #[tx_cst] 属性） ───────────────────────
        #clean_input

        // ── CompInit impl（默认空实现，用户可手动覆盖） ───────────────────
        // impl ::tx_di_core::CompInit for #struct_name {}
        # comp_init_impl

        // ── ComponentDescriptor impl ──────────────────────────────────────
        impl ::tx_di_core::ComponentDescriptor for #struct_name {
            const DEP_IDS: &'static [fn() -> ::std::any::TypeId] = &[
                #( #dep_type_ids ),*
            ];

            const SCOPE: ::tx_di_core::Scope = #scope_const;

            #build_impl
        }

        // ── linkme 注册条目 ───────────────────────────────────────────────
        // factory 返回 Box<T>，tx-di-core 的 call_factory 内部包 Arc<T>
        #[::tx_di_core::linkme::distributed_slice(::tx_di_core::COMPONENT_REGISTRY)]
        #[linkme(crate = ::tx_di_core::linkme)]
        #[allow(non_upper_case_globals)]
        #vis static #meta_ident: ::tx_di_core::ComponentMeta = ::tx_di_core::ComponentMeta {
            type_id: || ::std::any::TypeId::of::<#struct_name>(),
            deps: &[ #( #dep_type_ids ),* ],
            name: ::std::stringify!(#struct_name),
            scope: #scope_const,
            // build
            factory_fn: Some(|ctx: &mut ::tx_di_core::BuildContext| {
                ::std::boxed::Box::new(
                    <#struct_name as ::tx_di_core::ComponentDescriptor>::build(ctx)
                )
            }),
            // 初始化方法排序
            init_sort_fn: <#struct_name as ::tx_di_core::CompInit>::init_sort,
            // 初始化方法
            init_fn: Some(<#struct_name as ::tx_di_core::CompInit>::init),
            // 异步初始化方法
            async_init_fn: Some(<#struct_name as ::tx_di_core::CompInit>::async_init),
        };
    };

    Ok(output)
}

/// 解析 `#[tx_comp(...)]` 的参数
fn parse_component_attr(attr_tokens: TokenStream) -> SynResult<CompAttr> {
    if attr_tokens.is_empty() {
        return Ok(CompAttr {
            scope: ScopeAttr::Singleton,
            has_init: false,
            conf: None,
        });
    }

    struct AttrArgs {
        scope: ScopeAttr,
        has_init: bool,
        conf: Option<Option<String>>,
    }

    impl Parse for AttrArgs {
        fn parse(input: ParseStream) -> SynResult<Self> {
            let mut scope = ScopeAttr::Singleton;
            let mut has_init = false;
            let mut conf = None;
            // 解析逗号分隔的参数列表
            loop {
                if input.is_empty() {
                    break;
                }

                let key: Ident = input.parse()?;

                if key == "scope" {
                    if input.peek(Token![=]) {
                        let _eq: Token![=] = input.parse()?;
                        let value: Expr = input.parse()?;
                        let ident_str = match &value {
                            Expr::Path(p) => p
                                .path
                                .segments
                                .last()
                                .map(|s| s.ident.to_string())
                                .unwrap_or_default(),
                            _ => value.to_token_stream().to_string(),
                        };
                        scope = match ident_str.as_str() {
                            "Singleton" => ScopeAttr::Singleton,
                            "Prototype" => ScopeAttr::Prototype,
                            other => {
                                return Err(syn::Error::new_spanned(
                                    &value,
                                    format!(
                                        "未知的 scope `{}`，只支持 Singleton 或 Prototype",
                                        other
                                    ),
                                ))
                            }
                        };
                    } else {
                        scope = ScopeAttr::Prototype;
                    }
                }
                else if key == "init" {
                    // 裸 flag，不带 = value
                    has_init = true;
                }
                else if key == "conf" {
                    // 支持两种形式：
                    // 1. conf (裸 flag) → 使用结构体名转蛇形
                    // 2. conf = "custom_key" → 使用自定义键
                    if input.peek(Token![=]) {
                        let _eq: Token![=] = input.parse()?;
                        let value: Expr = input.parse()?;
                        let key_str = match &value {
                            Expr::Lit(lit) => {
                                if let syn::Lit::Str(s) = &lit.lit {
                                    s.value()
                                } else {
                                    return Err(syn::Error::new_spanned(
                                        &value,
                                        "conf 的值必须是字符串字面量",
                                    ));
                                }
                            }
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    &value,
                                    "conf 的值必须是字符串字面量",
                                ));
                            }
                        };
                        conf = Some(Some(key_str));
                    } else {
                        // 裸 flag，使用结构体名
                        conf = Some(None);
                    }
                } else {
                    return Err(syn::Error::new_spanned(
                        key,
                        "#[tx_comp] 只支持 scope 和 init 参数，\
                         例如：#[tx_comp(scope = Prototype, init)]",
                    ));
                }

                // 消耗可选的逗号分隔符
                if input.peek(Token![,]) {
                    let _: Token![,] = input.parse()?;
                }
                else {
                    break;
                }
            }

            Ok(AttrArgs { scope, has_init, conf })
        }
    }

    let args: AttrArgs = syn::parse(attr_tokens)?;
    Ok(CompAttr {
        scope: args.scope,
        has_init: args.has_init,
        conf: args.conf,
    })
}

/// `#[tx_comp(...)]` 的完整参数解析结果
///
/// case:
/// `#[tx_comp]` // 单例，不是配置，没有初始化方法
///
/// `#[tx_comp(scope)]` // 原型，不是配置，没有初始化方法
///
/// `#[tx_comp(init)]` // 单例，不是配置，初始化方法
///
/// `#[tx_comp(conf)]` // 单例，是配置，没有初始化方法
///
/// `#[tx_comp(conf = "aa_bb")]` //单例，是配置，没有初始化方法
#[derive(Debug)]
struct CompAttr {
    /// 作用域，默认 Singleton
    scope: ScopeAttr,
    /// 是否有 `init` flag：
    /// - 默认 false → 宏自动生成空 `impl CompInit`
    /// - true  → 用户自己写 `impl CompInit`，宏不生成
    has_init: bool,
    /// 配置组件标识：
    /// - 默认 None → 不是配置组件
    /// - Some(None) → 是配置组件，使用结构体名转蛇形作为配置键 case `AaBb` -> `aa_bb`
    /// - Some(Some(key)) → 是配置组件，使用自定义配置键
    conf: Option<Option<String>>,
}

const TX_CST: &str = "tx_cst";


/// 从 `#[component(scope = Singleton/Prototype)]` 属性解析 scope
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeAttr {
    Singleton,
    Prototype,
}

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

/// 检查字段是否有 #[tx_cst(skip)] 属性
fn has_skip_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident(TX_CST) {
            if let Ok(ident) = attr.parse_args::<Ident>() {
                return ident == "skip";
            }
        }
        false
    })
}
