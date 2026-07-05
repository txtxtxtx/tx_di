//! 生成 `linkme::distributed_slice(COMPONENT_REGISTRY)` 注册条目
//!
//! 生成一个 `static` 变量，其类型为 `ComponentMeta`，包含：
//! - `type_id` / `name`
//! - `dep_type_ids`（普通依赖 + trait 依赖，用于拓扑排序）
//! - `factory`（由 `factory` 模块生成的闭包）
//! - `scope`
//! - `impl_traits` / `trait_impls`（由 `as_trait` 生成）
//! - 生命周期函数指针（init_sort / inner_init / init / async_init / async_run / shutdown）

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::codegen::CodeGenContext;
use crate::name_utils::camel_to_screaming_snake;

/// 生成 linkme 注册条目
///
/// `factory_fn` 为 `factory` 模块生成的闭包 TokenStream。
pub fn gen_meta_entry(ctx: &CodeGenContext, factory_fn: TokenStream2) -> TokenStream2 {
    let struct_name = &ctx.struct_name;
    let vis = &ctx.vis;
    let scope_const = ctx.comp_attr.scope_tokens();
    let is_config_component = ctx.comp_attr.is_config_component();

    let meta_ident = format_ident!(
        "__DI_META_{}",
        camel_to_screaming_snake(&struct_name.to_string())
    );

    // ── dep_type_ids：普通依赖 + trait 依赖（配置组件为空）──────────────
    let dep_type_id_fns: Vec<TokenStream2> = if is_config_component {
        Vec::new()
    } else {
        let mut fns: Vec<TokenStream2> = ctx
            .inject_fields
            .iter()
            .map(|(_, ty)| quote! { || std::any::TypeId::of::<#ty>() })
            .collect();
        let trait_fns: Vec<TokenStream2> = ctx
            .trait_inject_fields
            .iter()
            .chain(ctx.required_trait_fields.iter())
            .map(|(_, ty)| quote! { || std::any::TypeId::of::<#ty>() })
            .collect();
        fns.extend(trait_fns);
        fns
    };

    // ── Trait 实现注册（as_trait）──────────────────────────────────────
    let (impl_traits_arr, trait_impls_arr) = if let Some(trait_ty) = &ctx.comp_attr.as_trait {
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

    quote! {
        // ── linkme 注册条目 ───────────────────────────────────────────────
        #[::tx_di_core::linkme::distributed_slice(::tx_di_core::COMPONENT_REGISTRY)]
        #[linkme(crate = ::tx_di_core::linkme)]
        #[allow(non_upper_case_globals)]
        #vis static #meta_ident: ::tx_di_core::ComponentMeta = ::tx_di_core::ComponentMeta {
            type_id: || std::any::TypeId::of::<#struct_name>(),
            name: std::stringify!(#struct_name),
            dep_type_ids: &[ #( #dep_type_id_fns ),* ],
            factory: ( #factory_fn ) as fn(&::tx_di_core::Store) -> Box<dyn std::any::Any + Send + Sync>,
            scope: #scope_const,
            impl_traits: &[ #( #impl_traits_arr ),* ],
            trait_impls: &[ #( #trait_impls_arr ),* ],
            // ── 生命周期函数指针 ──────────────────────────────────────────
            init_sort_fn: <#struct_name as ::tx_di_core::Component>::init_sort,
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
    }
}
