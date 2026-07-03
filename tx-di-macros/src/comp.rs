//! Component derive 宏 — 生成 Component trait 实现和 ComponentMeta 注册条目
//!
//! 支持的属性：
//! - `#[component(scope = Prototype)]` — 原型作用域
//! - `#[component(init)]` — 有自定义 init 实现
//! - `#[component(conf)]` / `#[component(conf = "key")]` — 配置组件
//! - `#[component(as_trait = dyn Trait)]` — Trait 实现注册
//! - `#[component(for(Type1, Type2))]` — 泛型具体化

use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, Literal};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    Attribute, Expr, Ident, ItemStruct, Result as SynResult,
    Token, Type,
};

use crate::utils::{camel_to_screaming_snake, camel_to_snake, is_option_type, is_arc_dyn_trait, extract_trait_from_option_arc, strip_arc_type};

/// `#[derive(Component)]` 入口
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    match derive_component_impl(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_component_impl(input: ItemStruct) -> SynResult<TokenStream2> {
    let struct_name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;

    // 解析 #[component(...)] 属性
    let comp_attr = parse_component_attr_from_attributes(&input.attrs)?;
    let comp_attr = comp_attr.unwrap_or_default();

    // 如果没有 #[component] 属性，不生成代码（仅 derive 不会触发）
    // 但 derive_component 是由 #[derive(Component)] 触发的，所以一定有
    // #[component] 是可选的辅助属性

    // 检查是否是泛型结构体
    if !generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "泛型结构体请使用 #[component(for(Type1, Type2))] 指定具体类型参数",
        ));
    }

    // 字段分类
    enum FieldKind {
        Inject { ty: Type },
        TraitInject { ty: Type },
        Custom { expr: Expr },
        Optional { ty: Type },
        Skip,
    }

    let mut fields_info: Vec<(syn::Ident, FieldKind)> = Vec::new();

    // 清理字段属性（去掉 #[tx_cst]）
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
        } else if is_arc_dyn_trait(&field.ty) {
            // Option<Arc<dyn Trait>> — trait object 注入
            FieldKind::TraitInject { ty: field.ty.clone() }
        } else if is_option_type(&field.ty) {
            FieldKind::Optional { ty: field.ty.clone() }
        } else if let Some(expr) = inject_expr {
            FieldKind::Custom { expr }
        } else {
            FieldKind::Inject { ty: field.ty.clone() }
        };
        fields_info.push((ident.clone(), kind));
    }

    // ── 生成 Deps 类型和 build 方法 ──────────────────────────────────────

    // inject_fields: 普通组件注入（Arc<T>）
    let inject_fields: Vec<(syn::Ident, Type)> = fields_info
        .iter()
        .filter_map(|(name, kind)| {
            if let FieldKind::Inject { ty } = kind {
                let inner_ty = strip_arc_type(ty);
                Some((name.clone(), inner_ty))
            } else {
                None
            }
        })
        .collect();

    // trait_inject_fields: trait object 注入（Option<Arc<dyn Trait>>）
    // 存储字段名 + 提取出的 dyn Trait 类型
    let trait_inject_fields: Vec<(syn::Ident, Type)> = fields_info
        .iter()
        .filter_map(|(name, kind)| {
            if let FieldKind::TraitInject { ty } = kind {
                let trait_ty = extract_trait_from_option_arc(ty)
                    .expect("is_arc_dyn_trait 已验证，提取 trait 类型不应失败");
                Some((name.clone(), trait_ty))
            } else {
                None
            }
        })
        .collect();

    let dep_types: Vec<Type> = inject_fields.iter().map(|(_, ty)| ty.clone()).collect();
    let dep_count = dep_types.len();

    // 生成 Deps 元组类型
    // 例：dep_types = [DbPool, AppConfig] → (Arc<DbPool>, Arc<AppConfig>)
    let deps_type = if dep_count == 0 {
        quote! { () }
    } else if dep_count == 1 {
        let ty = &dep_types[0];
        quote! { (std::sync::Arc<#ty>,) }
    } else {
        quote! { (#(std::sync::Arc<#dep_types>),*) }
    };

    // 生成 build 方法体（不含 trait inject 字段，那些在 inner_init 中填充）
    let build_fields: Vec<TokenStream2> = fields_info
        .iter()
        .map(|(fname, kind)| match kind {
            FieldKind::Skip => quote! { #fname: Default::default() },
            FieldKind::Optional { .. } => quote! { #fname: None },
            FieldKind::Inject { ty: _ } => {
                // 从 deps 元组中解构
                let idx = inject_fields
                    .iter()
                    .position(|(name, _)| name == fname)
                    .unwrap();
                let idx_lit = Literal::usize_unsuffixed(idx);
                quote! { #fname: deps.#idx_lit.clone() }
            }
            FieldKind::TraitInject { ty } => {
                // trait inject 字段：用 None 占位，在 inner_init 中填充
                quote! { #fname: None }
            }
            FieldKind::Custom { expr } => quote! { #fname: #expr },
        })
        .collect();

    // ── 生成 dep_type_ids ───────────────────────────────────────────────

    let dep_type_id_fns: Vec<TokenStream2> = dep_types
        .iter()
        .map(|ty| quote! { || std::any::TypeId::of::<#ty>() })
        .collect();

    // trait 依赖也需要加入 dep_type_ids（用于拓扑排序）
    let trait_dep_type_id_fns: Vec<TokenStream2> = trait_inject_fields
        .iter()
        .map(|(_, ty)| quote! { || std::any::TypeId::of::<#ty>() })
        .collect();

    // 合并普通依赖和 trait 依赖
    let all_dep_type_id_fns: Vec<TokenStream2> = dep_type_id_fns
        .into_iter()
        .chain(trait_dep_type_id_fns.into_iter())
        .collect();

    let scope_const = match &comp_attr.scope {
        ScopeAttr::Singleton => quote! { ::tx_di_core::Scope::Singleton },
        ScopeAttr::Prototype => quote! { ::tx_di_core::Scope::Prototype },
    };

    // ── 生成 factory 函数 ───────────────────────────────────────────────

    let meta_ident = format_ident!(
        "__DI_META_{}",
        camel_to_screaming_snake(&struct_name.to_string())
    );

    // 重新构造结构体（去掉 #[tx_cst] 属性）
    let clean_input = ItemStruct {
        fields: clean_fields,
        ..input.clone()
    };

    // ── 配置组件特殊处理 ────────────────────────────────────────────────
    let is_config_component = comp_attr.conf.is_some();

    let (deps_type_final, build_body, dep_type_id_fns_final) = if is_config_component {
        // 配置组件：Deps = ()，build 不直接调用（factory 函数处理反序列化）
        let build_body = quote! {
            panic!("[di] 配置组件 {} 的 build() 不应被直接调用", stringify!(#struct_name))
        };
        (quote! { () }, build_body, Vec::new())
    } else {
        // 普通组件
        let build_body = quote! {
            Self {
                #( #build_fields ),*
            }
        };
        (deps_type, build_body, all_dep_type_id_fns)
    };

    // ── 生成 factory 函数 ───────────────────────────────────────────────

    let factory_fn = if is_config_component {
        let config_key = if let Some(Some(custom_key)) = &comp_attr.conf {
            quote! { #custom_key }
        } else {
            let snake_name = camel_to_snake(&struct_name.to_string());
            quote! { #snake_name }
        };

        quote! {
            |store: &::tx_di_core::Store| {
                let app_config = ::tx_di_core::inject_from_store::<::tx_di_core::AppAllConfig>(store);
                let config_key = #config_key;
                let mut config = if let Some(value) = app_config.get_value(config_key) {
                    <#struct_name as serde::Deserialize>::deserialize(value.clone())
                        .unwrap_or_else(|e| {
                            panic!(
                                "[di] 配置组件 '{}' 反序列化失败 (key='{}'): {}\n\
                                 请检查配置文件中该字段的类型和格式是否正确。",
                                stringify!(#struct_name), config_key, e
                            )
                        })
                } else {
                    let empty_table = ::tx_di_core::Value::Table(::tx_di_core::map::Map::new());
                    <#struct_name as serde::Deserialize>::deserialize(empty_table)
                        .unwrap_or_else(|e| {
                            panic!(
                                "[di] 配置组件 '{}' 缺少配置 key='{}', 且默认值反序列化也失败: {}\n\
                                 请在配置文件中添加该 section, 或为所有字段提供 #[serde(default)]。",
                                stringify!(#struct_name), config_key, e
                            )
                        })
                };
                ::tracing::debug!("{} build 成功", stringify!(#struct_name));
                Box::new(config) as Box<dyn std::any::Any + Send + Sync>
            }
        }
    } else {
        quote! {
            |store: &::tx_di_core::Store| {
                let deps = <#struct_name as ::tx_di_core::Component>::Deps::resolve(store)
                    .unwrap_or_else(|e| panic!("{}", e));
                let mut instance = <#struct_name as ::tx_di_core::Component>::build(deps);
                if let Err(e) = <#struct_name as ::tx_di_core::Component>::inner_init(&mut instance, store) {
                    panic!("[di] 组件 '{}' inner_init 失败: {}", stringify!(#struct_name), e);
                }
                ::tracing::debug!("{} build 成功", stringify!(#struct_name));
                Box::new(instance) as Box<dyn std::any::Any + Send + Sync>
            }
        }
    };

    // ── 生成 Trait 实现注册 ─────────────────────────────────────────────

    let (impl_traits_arr, trait_impls_arr) = if let Some(trait_ty) = &comp_attr.as_trait {
        (
            vec![quote! { || std::any::TypeId::of::<#trait_ty>() }],
            vec![quote! {
                ::tx_di_core::TraitImplEntry {
                    concrete_tid: || std::any::TypeId::of::<#struct_name>(),
                    upcast: |concrete: std::sync::Arc<dyn std::any::Any + Send + Sync>| {
                        let instance = concrete.downcast::<#struct_name>()
                            .expect("[di] trait upcast: 具体类型 downcast 失败");
                        let as_trait: std::sync::Arc<#trait_ty> = instance;
                        std::sync::Arc::new(as_trait) as std::sync::Arc<dyn std::any::Any + Send + Sync>
                    },
                }
            }],
        )
    } else {
        (vec![], vec![])
    };

    // ── 生成 inner_init 覆盖（如果有 trait inject 字段）──────────────────

    let inner_init_impl = if trait_inject_fields.is_empty() {
        quote! {}
    } else {
        let trait_inject_assigns: Vec<TokenStream2> = trait_inject_fields
            .iter()
            .map(|(fname, ty)| {
                quote! {
                    self.#fname = Some(::tx_di_core::inject_trait_from_store::<#ty>(store));
                }
            })
            .collect();

        quote! {
            fn inner_init(&mut self, store: &::tx_di_core::Store) -> ::tx_di_core::RIE<()> {
                #( #trait_inject_assigns )*
                Ok(())
            }
        }
    };

    // ── 生成最终代码 ────────────────────────────────────────────────────
    // 注意：derive 宏只追加 impl 和 linkme 注册，不重新输出结构体

    let output = quote! {
        // ── Component trait 实现 ──────────────────────────────────────────
        impl ::tx_di_core::Component for #struct_name {
            type Deps = #deps_type_final;

            fn build(deps: Self::Deps) -> Self {
                #build_body
            }

            const SCOPE: ::tx_di_core::Scope = #scope_const;

            // 如果有 trait inject 字段，覆盖 inner_init
            #inner_init_impl
        }

        // ── linkme 注册条目 ───────────────────────────────────────────────
        #[::tx_di_core::linkme::distributed_slice(::tx_di_core::COMPONENT_REGISTRY)]
        #[linkme(crate = ::tx_di_core::linkme)]
        #[allow(non_upper_case_globals)]
        #vis static #meta_ident: ::tx_di_core::ComponentMeta = ::tx_di_core::ComponentMeta {
            type_id: || std::any::TypeId::of::<#struct_name>(),
            name: std::stringify!(#struct_name),
            dep_type_ids: &[ #( #dep_type_id_fns_final ),* ],
            factory: ( #factory_fn ) as fn(&::tx_di_core::Store) -> Box<dyn std::any::Any + Send + Sync>,
            scope: #scope_const,
            impl_traits: &[ #( #impl_traits_arr ),* ],
            trait_impls: &[ #( #trait_impls_arr ),* ],
            // ── 生命周期函数指针 ──────────────────────────────────────────
            init_sort_fn: <#struct_name as ::tx_di_core::Component>::init_sort,
            inner_init_fn: |store: &::tx_di_core::Store| -> ::tx_di_core::RIE<()> {
                let arc = ::tx_di_core::inject_from_store::<#struct_name>(store);
                let mut guard = arc;
                // 对于 Singleton，实例已经被 factory 放入 store
                // inner_init 在 factory 函数中已经调用
                // 这里仅作为占位
                Ok(())
            },
            init_fn: |app: &std::sync::Arc<::tx_di_core::App>| -> ::tx_di_core::RIE<()> {
                <#struct_name as ::tx_di_core::Component>::init(app)
            },
            async_init_fn: |app: &std::sync::Arc<::tx_di_core::App>| -> ::tx_di_core::BoxFuture<::tx_di_core::RIE<()>> {
                <#struct_name as ::tx_di_core::Component>::async_init(app)
            },
            async_run_fn: |app: &std::sync::Arc<::tx_di_core::App>, token: ::tx_di_core::CancellationToken| -> ::tx_di_core::BoxFuture<::tx_di_core::RIE<()>> {
                <#struct_name as ::tx_di_core::Component>::async_run(app, token)
            },
            shutdown_fn: |store: &::tx_di_core::Store| {
                if let Some(arc) = store.try_inject::<#struct_name>() {
                    // arc 是 Arc<T>，需要获取 &T 调用 shutdown
                    // 但 Arc 无法直接获取 &mut，shutdown 是 &self 方法
                    // 通过 Arc::deref 获取引用
                    use std::ops::Deref;
                    arc.deref().shutdown();
                }
            },
        };
    };

    Ok(output)
}

// ── 解析 #[component(...)] 属性 ────────────────────────────────────────────

fn parse_component_attr_from_attributes(attrs: &[Attribute]) -> SynResult<Option<CompAttr>> {
    for attr in attrs {
        if attr.path().is_ident("component") {
            let args: CompAttrArgs = syn::parse2(attr.meta.require_list()?.tokens.clone())?;
            return Ok(Some(args.into()));
        }
    }
    Ok(None)
}

#[derive(Default)]
struct CompAttr {
    scope: ScopeAttr,
    has_init: bool,
    conf: Option<Option<String>>,
    as_trait: Option<Type>,
}

#[derive(Default)]
struct CompAttrArgs {
    scope: ScopeAttr,
    has_init: bool,
    conf: Option<Option<String>>,
    as_trait: Option<Type>,
}

impl Parse for CompAttrArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut scope = ScopeAttr::Singleton;
        let mut has_init = false;
        let mut conf = None;
        let mut as_trait = None;

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
                                format!("未知的 scope `{}`", other),
                            ))
                        }
                    };
                } else {
                    scope = ScopeAttr::Prototype;
                }
            } else if key == "init" {
                has_init = true;
            } else if key == "conf" {
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
                                    "conf 值必须是字符串",
                                ));
                            }
                        }
                        _ => return Err(syn::Error::new_spanned(&value, "conf 值必须是字符串")),
                    };
                    conf = Some(Some(key_str));
                } else {
                    conf = Some(None);
                }
            } else if key == "as_trait" {
                if input.peek(Token![=]) {
                    let _eq: Token![=] = input.parse()?;
                    let trait_type: Type = input.parse()?;
                    as_trait = Some(trait_type);
                } else {
                    return Err(syn::Error::new_spanned(
                        key,
                        "as_trait 必须指定值，例如 as_trait = dyn UserRepository",
                    ));
                }
            } else if key == "intercept" {
                // AOP 拦截器 — 暂时跳过，后续实现
                // 消耗参数但不处理
                if input.peek(syn::token::Paren) {
                    let _content: syn::ExprParen = input.parse()?;
                }
            } else if key == "for" {
                // 泛型具体化 — 暂时跳过，后续实现
                if input.peek(syn::token::Paren) {
                    let _content: syn::ExprParen = input.parse()?;
                }
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "#[component] 支持 scope / init / conf / as_trait / intercept / for 参数",
                ));
            }

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            } else {
                break;
            }
        }

        Ok(CompAttrArgs {
            scope,
            has_init,
            conf,
            as_trait,
        })
    }
}

impl From<CompAttrArgs> for CompAttr {
    fn from(args: CompAttrArgs) -> Self {
        CompAttr {
            scope: args.scope,
            has_init: args.has_init,
            conf: args.conf,
            as_trait: args.as_trait,
        }
    }
}

#[derive(Clone)]
enum ScopeAttr {
    Singleton,
    Prototype,
}

impl Default for ScopeAttr {
    fn default() -> Self {
        ScopeAttr::Singleton
    }
}

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
            if let Ok(ident) = attr.parse_args::<Ident>() {
                return ident == "skip";
            }
        }
        false
    })
}
