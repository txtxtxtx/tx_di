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
pub fn tx_comp(attr: TokenStream, item: TokenStream) -> TokenStream {
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

    enum FieldKind {
        Inject { ty: Type },
        Custom { expr: Expr },
        #[allow(dead_code)]
        Optional { ty: Type },
        Skip,
    }

    let mut fields_info: Vec<(syn::Ident, FieldKind)> = Vec::new();

    let mut clean_fields = input.fields.clone();
    for field in &mut clean_fields {
        field.attrs.retain(|a| !a.path().is_ident(TX_CST));
    }

    for field in &input.fields {
        let ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(&field.ty, "#[tx_cst] 只支持具名字段"))?;
        let inject_expr = extract_inject_expr(&field.attrs)?;
        let kind = if has_skip_attr(&field.attrs) {
            FieldKind::Skip
        }
        else if is_option_type(&field.ty) {
            FieldKind::Optional { ty: field.ty.clone() }
        }
        else if let Some(expr) = inject_expr {
            FieldKind::Custom { expr }
        }
        else {
            FieldKind::Inject { ty: field.ty.clone() }
        };
        fields_info.push((ident.clone(), kind));
    }

    // ── 生成 build() / build_fields 字段赋值 ────────────────────────────────
    // build 序列化 store-based → 用 inject_from_store 填充依赖

    let build_fields: Vec<TokenStream2> = fields_info
        .iter()
        .map(|(fname, kind)| {
            match kind {
                FieldKind::Skip => quote! { #fname: Default::default() },
                FieldKind::Optional { .. } => quote! { #fname: None },
                FieldKind::Inject { ty } => {
                    let inject_ty = strip_arc(ty);
                    quote! { #fname: ::tx_di_core::inject_from_store::<#inject_ty>(store) }
                }
                FieldKind::Custom { expr } => quote! { #fname: #expr },
            }
        })
        .collect();

    // ── 生成 DEP_IDS ─────────────────────────────────────────────────────

    let mut dep_type_ids: Vec<TokenStream2> = fields_info
        .iter()
        .filter_map(|(_, kind)| match kind {
            FieldKind::Inject { ty } => {
                let inject_ty = strip_arc(ty);
                Some(quote! { || ::std::any::TypeId::of::<#inject_ty>() })
            }
            FieldKind::Custom { .. } | FieldKind::Optional { .. } | FieldKind::Skip => None,
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

    // 重新构造结构体（去掉 #[tx_cst] 属性）
    let clean_input = ItemStruct {
        fields: clean_fields,
        ..input.clone()
    };

    let comp_init_impl = if comp_attr.has_init {
        quote! {}
    } else {
        quote! {
            impl ::tx_di_core::CompInit for #struct_name {}
        }
    };

    // ── 生成 store-based build 方法 ──────────────────────────────────────
    // 统一签名：fn build(store: &DashMap<TypeId, CompRef>) -> Self

    let build_impl = if let Some(conf_option) = &comp_attr.conf {
        // ── 配置组件 ─────────────────────────────────────────────────
        let config_key = if let Some(custom_key) = conf_option {
            quote! { #custom_key }
        } else {
            let snake_name = camel_to_snake(&struct_name.to_string());
            quote! { #snake_name }
        };
        dep_type_ids = vec![];
        quote! {
            fn build(
                store: &::tx_di_core::DashMap<::std::any::TypeId, ::tx_di_core::CompRef>,
            ) -> Self {
                let app_config = ::tx_di_core::inject_from_store::<::tx_di_core::AppAllConfig>(store);
                let config = if let Some(value) = app_config.get_value(#config_key) {
                    <Self as ::serde::Deserialize>::deserialize(value.clone())
                        .unwrap_or_else(|e| {
                            let empty_table = ::tx_di_core::Value::Table(::tx_di_core::map::Map::new());
                            <Self as ::serde::Deserialize>::deserialize(empty_table)
                                .expect("[di] 配置组件反序列化失败")
                        })
                } else {
                    let empty_table = ::tx_di_core::Value::Table(::tx_di_core::map::Map::new());
                    <Self as ::serde::Deserialize>::deserialize(empty_table)
                        .expect("[di] 配置组件反序列化失败")
                };
                // 注意：inner_init 需要 &mut BuildContext，此处跳过
                // 所有现有 inner_init 实现均不使用 ctx 参数
                config
            }
        }
    } else {
        // ── 非配置组件 ─────────────────────────────────────────────
        quote! {
            fn build(
                store: &::tx_di_core::DashMap<::std::any::TypeId, ::tx_di_core::CompRef>,
            ) -> Self {
                let mut ctx = Self {
                    #( #build_fields ),*
                };
                if let Err(e) = <Self as ::tx_di_core::CompInit>::inner_init(&mut ctx, store) {
                    panic!("[di] 组件 '{}' 初始化失败: {}", stringify!(#struct_name), e);
                }
                ::tracing::debug!("{} build 成功",stringify!(#struct_name));
                ctx
            }
        }
    };

    let output = quote! {
        // ── 原始结构体定义（已去掉 #[tx_cst] 属性） ───────────────────────
        #clean_input

        // ── CompInit impl（默认空实现，用户可手动覆盖） ───────────────────
        #comp_init_impl

        // ── ComponentDescriptor impl ──────────────────────────────────────
        impl ::tx_di_core::ComponentDescriptor for #struct_name {
            const DEP_IDS: &'static [fn() -> ::std::any::TypeId] = &[
                #( #dep_type_ids ),*
            ];

            const SCOPE: ::tx_di_core::Scope = #scope_const;

            #build_impl
        }

        // ── linkme 注册条目 ───────────────────────────────────────────────
        // factory 返回 Box<T>，tx-di-core 内部包 Arc<T>
        #[::tx_di_core::linkme::distributed_slice(::tx_di_core::COMPONENT_REGISTRY)]
        #[linkme(crate = ::tx_di_core::linkme)]
        #[allow(non_upper_case_globals)]
        #vis static #meta_ident: ::tx_di_core::ComponentMeta = ::tx_di_core::ComponentMeta {
            type_id: || ::std::any::TypeId::of::<#struct_name>(),
            deps: &[ #( #dep_type_ids ),* ],
            name: ::std::stringify!(#struct_name),
            scope: #scope_const,
            // build — 统一 store-based factory_fn 签名
            factory_fn: Some((|store: &::tx_di_core::DashMap<::std::any::TypeId, ::tx_di_core::CompRef>| {
                ::std::boxed::Box::new(
                    <#struct_name as ::tx_di_core::ComponentDescriptor>::build(store)
                ) as ::std::boxed::Box<dyn ::std::any::Any + ::std::marker::Send + ::std::marker::Sync>
            }) as fn(&::tx_di_core::DashMap<::std::any::TypeId, ::tx_di_core::CompRef>) -> ::std::boxed::Box<dyn ::std::any::Any + ::std::marker::Send + ::std::marker::Sync>),
            init_sort_fn: <#struct_name as ::tx_di_core::CompInit>::init_sort,
            init_fn: Some(<#struct_name as ::tx_di_core::CompInit>::init),
            async_init_fn: Some(<#struct_name as ::tx_di_core::CompInit>::async_init),
        };
    };

    Ok(output)
}

// ── 解析 `#[tx_comp(...)]` 参数 ────────────────────────────────────────────

fn parse_component_attr(attr_tokens: TokenStream) -> SynResult<CompAttr> {
    if attr_tokens.is_empty() {
        return Ok(CompAttr { scope: ScopeAttr::Singleton, has_init: false, conf: None });
    }

    struct AttrArgs { scope: ScopeAttr, has_init: bool, conf: Option<Option<String>> }

    impl Parse for AttrArgs {
        fn parse(input: ParseStream) -> SynResult<Self> {
            let mut scope = ScopeAttr::Singleton;
            let mut has_init = false;
            let mut conf = None;
            loop {
                if input.is_empty() { break; }
                let key: Ident = input.parse()?;

                if key == "scope" {
                    if input.peek(Token![=]) {
                        let _eq: Token![=] = input.parse()?;
                        let value: Expr = input.parse()?;
                        let ident_str = match &value {
                            Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
                            _ => value.to_token_stream().to_string(),
                        };
                        scope = match ident_str.as_str() {
                            "Singleton" => ScopeAttr::Singleton,
                            "Prototype" => ScopeAttr::Prototype,
                            other => return Err(syn::Error::new_spanned(&value,
                                format!("未知的 scope `{}`", other))),
                        };
                    } else {
                        scope = ScopeAttr::Prototype;
                    }
                }
                else if key == "init" { has_init = true; }
                else if key == "conf" {
                    if input.peek(Token![=]) {
                        let _eq: Token![=] = input.parse()?;
                        let value: Expr = input.parse()?;
                        let key_str = match &value {
                            Expr::Lit(lit) => if let syn::Lit::Str(s) = &lit.lit { s.value() }
                                else { return Err(syn::Error::new_spanned(&value, "conf 值必须是字符串")); },
                            _ => return Err(syn::Error::new_spanned(&value, "conf 值必须是字符串")),
                        };
                        conf = Some(Some(key_str));
                    } else { conf = Some(None); }
                }
                else {
                    return Err(syn::Error::new_spanned(key,
                        "#[tx_comp] 支持 scope / init / conf 参数"));
                }

                if input.peek(Token![,]) { let _: Token![,] = input.parse()?; }
                else { break; }
            }
            Ok(AttrArgs { scope, has_init, conf })
        }
    }

    let args: AttrArgs = syn::parse(attr_tokens)?;
    Ok(CompAttr { scope: args.scope, has_init: args.has_init, conf: args.conf })
}

#[derive(Debug)]
struct CompAttr {
    scope: ScopeAttr,
    has_init: bool,
    conf: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeAttr { Singleton, Prototype }

const TX_CST: &str = "tx_cst";

fn extract_inject_expr(attrs: &[Attribute]) -> SynResult<Option<Expr>> {
    for attr in attrs {
        if attr.path().is_ident(TX_CST) {
            let expr: Expr = attr.parse_args()?;
            return Ok(Some(expr));
        }
    }
    Ok(None)
}

fn has_skip_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident(TX_CST) {
            if let Ok(ident) = attr.parse_args::<Ident>() { return ident == "skip"; }
        }
        false
    })
}
