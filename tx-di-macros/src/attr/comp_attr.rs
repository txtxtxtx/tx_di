//! `#[component(...)]` 结构体属性解析
//!
//! 支持的参数：
//! - `scope = Prototype` / `scope = Singleton` — 作用域
//! - `init` — 自定义 `inner_init` 实现（回调 `fn init(&mut self, store)`)
//! - `init_sort = N` — 自定义初始化排序
//! - `conf` / `conf = "key"` — 配置组件
//! - `as_trait = dyn Trait` — Trait 实现注册
//! - `app_init` — 覆写 `init` 生命周期（回调 `fn app_init(comp, app)`)
//! - `app_async_init` — 覆写 `async_init` 生命周期（回调 `fn app_async_init(comp, app)`)
//! - `app_async_run` — 覆写 `async_run` 生命周期（回调 `fn app_async_run(comp, app, token)`)
//! - `shutdown` — 覆写 `shutdown` 生命周期（回调 `fn shutdown(&self)`)
//! - `intercept(...)` / `for(...)` — 占位（后续实现）

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    Attribute, Expr, Ident, Result as SynResult, Token, Type,
};

/// 从属性列表中解析 `#[component(...)]`，未找到则返回 `None`
pub fn parse_component_attr_from_attributes(attrs: &[Attribute]) -> SynResult<Option<CompAttr>> {
    for attr in attrs {
        if attr.path().is_ident("component") {
            let args: CompAttrArgs = syn::parse2(attr.meta.require_list()?.tokens.clone())?;
            return Ok(Some(args.into()));
        }
    }
    Ok(None)
}

/// 解析后的 `#[component(...)]` 属性
#[derive(Default)]
pub struct CompAttr {
    pub scope: ScopeAttr,
    pub has_init: bool,
    pub has_app_init: bool,
    pub has_app_async_init: bool,
    pub has_app_async_run: bool,
    pub has_shutdown: bool,
    /// 初始化排序表达式（原样输出到生成的代码中）
    pub init_sort: Option<Expr>,
    pub conf: Option<Option<String>>,
    pub as_trait: Option<Type>,
    /// 拦截器类型列表（用于 AOP）
    pub interceptors: Vec<Type>,
}

impl CompAttr {
    /// 是否为配置组件
    pub fn is_config_component(&self) -> bool {
        self.conf.is_some()
    }

    /// 是否有任何生命周期覆写被启用
    #[allow(dead_code)]
    pub fn has_any_lifecycle(&self) -> bool {
        self.has_init
            || self.has_app_init
            || self.has_app_async_init
            || self.has_app_async_run
            || self.has_shutdown
    }

    /// 生成 scope 对应的 TokenStream
    pub fn scope_tokens(&self) -> TokenStream2 {
        match self.scope {
            ScopeAttr::Singleton => quote! { ::tx_di_core::Scope::Singleton },
            ScopeAttr::Prototype => quote! { ::tx_di_core::Scope::Prototype },
        }
    }
}

/// `#[component(...)]` 的解析中间结构
#[derive(Default)]
struct CompAttrArgs {
    scope: ScopeAttr,
    has_init: bool,
    has_app_init: bool,
    has_app_async_init: bool,
    has_app_async_run: bool,
    has_shutdown: bool,
    init_sort: Option<Expr>,
    conf: Option<Option<String>>,
    as_trait: Option<Type>,
    interceptors: Vec<Type>,
}

impl From<CompAttrArgs> for CompAttr {
    fn from(args: CompAttrArgs) -> Self {
        CompAttr {
            scope: args.scope,
            has_init: args.has_init,
            has_app_init: args.has_app_init,
            has_app_async_init: args.has_app_async_init,
            has_app_async_run: args.has_app_async_run,
            has_shutdown: args.has_shutdown,
            init_sort: args.init_sort,
            conf: args.conf,
            as_trait: args.as_trait,
            interceptors: args.interceptors,
        }
    }
}

/// 作用域属性
#[derive(Clone, Default)]
pub enum ScopeAttr {
    #[default]
    Singleton,
    Prototype,
}

impl Parse for CompAttrArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut scope = ScopeAttr::Singleton;
        let mut has_init = false;
        let mut has_app_init = false;
        let mut has_app_async_init = false;
        let mut has_app_async_run = false;
        let mut has_shutdown = false;
        let mut init_sort = None;
        let mut conf = None;
        let mut as_trait = None;
        let mut interceptors = Vec::new();

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
            } else if key == "app_init" {
                has_app_init = true;
            } else if key == "app_async_init" {
                has_app_async_init = true;
            } else if key == "app_async_run" {
                has_app_async_run = true;
            } else if key == "shutdown" {
                has_shutdown = true;
            } else if key == "init_sort" {
                if input.peek(Token![=]) {
                    let _eq: Token![=] = input.parse()?;
                    let value: Expr = input.parse()?;
                    init_sort = Some(value);
                } else {
                    return Err(syn::Error::new_spanned(&key, "init_sort 必须指定值，如 init_sort = i32::MAX"));
                }
            } else if key == "conf" {
                if input.peek(Token![=]) {
                    let _eq: Token![=] = input.parse()?;
                    let value: Expr = input.parse()?;
                    let key_str = match &value {
                        Expr::Lit(lit) => {
                            if let syn::Lit::Str(s) = &lit.lit {
                                s.value()
                            } else {
                                return Err(syn::Error::new_spanned(&value, "conf 值必须是字符串"));
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
                // AOP 拦截器 — 解析拦截器类型列表
                let content;
                syn::parenthesized!(content in input);
                use syn::punctuated::Punctuated;
                let types: Punctuated<Type, Token![,]> =
                    content.parse_terminated(Type::parse, Token![,])?;
                interceptors = types.into_iter().collect();
            } else if key == "for" {
                return Err(syn::Error::new_spanned(
                    key,
                    "#[component(for(...))] 尚未实现。\n\
                     替代方案: 使用 newtype 包装具体类型，或手动为具体类型实现 Component trait。",
                ));
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "#[component] 支持 scope / init / app_init / app_async_init / app_async_run / shutdown / init_sort / conf / as_trait / intercept 参数",
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
            has_app_init,
            has_app_async_init,
            has_app_async_run,
            has_shutdown,
            init_sort,
            conf,
            as_trait,
            interceptors,
        })
    }
}

