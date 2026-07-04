//! `#[component(...)]` 结构体属性解析
//!
//! 支持的参数：
//! - `scope = Prototype` / `scope = Singleton` — 作用域
//! - `init` — 有自定义 init 实现（回调 `__di_component_init`）
//! - `init_sort = N` — 自定义初始化排序
//! - `conf` / `conf = "key"` — 配置组件
//! - `as_trait = dyn Trait` — Trait 实现注册
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
    pub init_sort: Option<i32>,
    pub conf: Option<Option<String>>,
    pub as_trait: Option<Type>,
}

impl CompAttr {
    /// 是否为配置组件
    pub fn is_config_component(&self) -> bool {
        self.conf.is_some()
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
    init_sort: Option<i32>,
    conf: Option<Option<String>>,
    as_trait: Option<Type>,
}

impl From<CompAttrArgs> for CompAttr {
    fn from(args: CompAttrArgs) -> Self {
        CompAttr {
            scope: args.scope,
            has_init: args.has_init,
            init_sort: args.init_sort,
            conf: args.conf,
            as_trait: args.as_trait,
        }
    }
}

/// 作用域属性
#[derive(Clone)]
pub enum ScopeAttr {
    Singleton,
    Prototype,
}

impl Default for ScopeAttr {
    fn default() -> Self {
        ScopeAttr::Singleton
    }
}

impl Parse for CompAttrArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut scope = ScopeAttr::Singleton;
        let mut has_init = false;
        let mut init_sort = None;
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
            } else if key == "init_sort" {
                if input.peek(Token![=]) {
                    let _eq: Token![=] = input.parse()?;
                    let value: Expr = input.parse()?;
                    let raw = match &value {
                        // 正数：init_sort = 100
                        Expr::Lit(lit) => {
                            if let syn::Lit::Int(i) = &lit.lit {
                                i.base10_parse::<i64>().map(|v| v as i32)
                            } else {
                                return Err(syn::Error::new_spanned(&value, "init_sort 值必须是整数"));
                            }
                        }
                        // 负数：init_sort = -2147483648
                        Expr::Unary(u) => {
                            if let syn::UnOp::Neg(_) = &u.op {
                                if let Expr::Lit(lit) = &*u.expr {
                                    if let syn::Lit::Int(i) = &lit.lit {
                                        let v: i64 = i.base10_parse::<i64>().map_err(|e| {
                                            syn::Error::new_spanned(&value, e)
                                        })?;
                                        Ok((-v) as i32)
                                    } else {
                                        return Err(syn::Error::new_spanned(&value, "init_sort 值必须是整数"));
                                    }
                                } else {
                                    return Err(syn::Error::new_spanned(&value, "init_sort 值必须是整数"));
                                }
                            } else {
                                return Err(syn::Error::new_spanned(&value, "init_sort 值必须是整数"));
                            }
                        }
                        _ => return Err(syn::Error::new_spanned(&value, "init_sort 值必须是整数")),
                    };
                    init_sort = Some(raw.map_err(|e| syn::Error::new_spanned(&value, e))?);
                } else {
                    return Err(syn::Error::new_spanned(&key, "init_sort 必须指定值，如 init_sort = -2147483648"));
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
                // AOP 拦截器 — 暂时跳过，后续实现
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
                    "#[component] 支持 scope / init / init_sort / conf / as_trait / intercept / for 参数",
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
            init_sort,
            conf,
            as_trait,
        })
    }
}

