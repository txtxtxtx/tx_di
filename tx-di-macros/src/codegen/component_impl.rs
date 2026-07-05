//! 生成 `impl Component for T` 块
//!
//! 包含：`type Deps`、`build()` 方法体、`SCOPE` 常量、
//! `inner_init` 方法（委托 `inner_init` 模块）、`init_sort` 方法、
//! 以及生命周期覆写（委托 `lifecycle` 模块）。

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Type;

use crate::classify::fields::FieldKind;
use crate::codegen::inner_init::gen_inner_init;
use crate::codegen::lifecycle::{gen_lifecycle_overrides, LifecycleOverrides};
use crate::codegen::CodeGenContext;

/// 生成 `impl ::tx_di_core::Component for #struct_name { ... }`
pub fn gen_component_impl(ctx: &CodeGenContext) -> TokenStream2 {
    let struct_name = &ctx.struct_name;
    let scope_const = ctx.comp_attr.scope_tokens();
    let is_config_component = ctx.comp_attr.is_config_component();

    // ── Deps 元组类型 ────────────────────────────────────────────────
    let dep_types: Vec<&Type> = ctx
        .inject_fields
        .iter()
        .map(|(_, ty)| ty)
        .collect();
    let dep_count = dep_types.len();

    let deps_type = if dep_count == 0 {
        quote! { () }
    } else if dep_count == 1 {
        let ty = &dep_types[0];
        quote! { (std::sync::Arc<#ty>,) }
    } else {
        quote! { (#(std::sync::Arc<#dep_types>),*) }
    };

    // ── build 方法体（不含 trait inject 字段，那些在 inner_init 中填充）──
    let build_fields: Vec<TokenStream2> = ctx
        .fields_info
        .iter()
        .map(|(fname, kind)| match kind {
            FieldKind::Skip => quote! { #fname: Default::default() },
            FieldKind::Optional { .. } => quote! { #fname: None },
            FieldKind::Inject { .. } => {
                // 从 deps 元组中解构
                let idx = ctx
                    .inject_fields
                    .iter()
                    .position(|(name, _)| name == fname)
                    .unwrap();
                let idx_lit = proc_macro2::Literal::usize_unsuffixed(idx);
                quote! { #fname: deps.#idx_lit.clone() }
            }
            FieldKind::TraitInject { .. } => {
                // 可选 trait inject：用 None 占位，在 inner_init 中填充
                quote! { #fname: None }
            }
            FieldKind::TraitInjectRequired { .. } => {
                // 必选 trait inject：用零值占位，inner_init 中通过 ptr::write 覆盖（避免 drop）
                // SAFETY: zeroed Arc<dyn Trait> 是托管内存的无效状态，
                // 但 inner_init 紧接着就会用 ptr::write 写入真实值，不会读取或 drop 该占位值。
                quote! {
                    #fname: unsafe { ::core::mem::zeroed() }
                }
            }
            FieldKind::Custom { expr } => quote! { #fname: #expr },
        })
        .collect();

    // ── 配置组件 vs 普通组件 ─────────────────────────────────────────
    let (deps_type_final, build_body) = if is_config_component {
        // 配置组件：Deps = ()，build 不直接调用（factory 函数处理反序列化）
        let build_body = quote! {
            panic!("[di] 配置组件 {} 的 build() 不应被直接调用", stringify!(#struct_name))
        };
        (quote! { () }, build_body)
    } else {
        let build_body = quote! {
            Self {
                #( #build_fields ),*
            }
        };
        (deps_type, build_body)
    };

    // ── inner_init 与 init_sort 覆盖 ─────────────────────────────────
    let inner_init_impl = gen_inner_init(ctx);
    let init_sort_override = ctx.comp_attr.init_sort.map(|val| {
        let lit = proc_macro2::Literal::i32_suffixed(val);
        quote! {
            fn init_sort() -> i32 { #lit }
        }
    });

    // ── 生命周期覆写（app_init / app_async_init / app_async_run / shutdown）─
    let LifecycleOverrides {
        app_init,
        app_async_init,
        app_async_run,
        shutdown,
    } = gen_lifecycle_overrides(ctx);

    quote! {
        // ── Component trait 实现 ──────────────────────────────────────────
        impl ::tx_di_core::Component for #struct_name {
            type Deps = #deps_type_final;

            fn build(deps: Self::Deps) -> Self {
                #build_body
            }

            const SCOPE: ::tx_di_core::Scope = #scope_const;

            // inner_init: trait inject 字段填充 | #[component(init)] 回调
            #inner_init_impl

            // 生命周期覆写: #[component(app_init / app_async_init / app_async_run / shutdown)]
            #app_init
            #app_async_init
            #app_async_run
            #shutdown

            // init_sort: #[component(init_sort = N)] 自定义排序
            #init_sort_override
        }
    }
}

