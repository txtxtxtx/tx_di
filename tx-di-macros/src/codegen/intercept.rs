//! 生成 AOP 拦截器相关代码
//!
//! 当 `#[component(intercept(T1, T2, ...))]` 指定拦截器时：
//! - 生成 `interceptor_chain()` 关联函数（返回 `&'static InterceptorChain`）
//! - 生成 `init` 覆写：在 App 阶段精确注入拦截器并初始化链

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::codegen::CodeGenContext;

/// 生成 `interceptor_chain()` 关联函数（用于 `#[intercept]` 属性宏调用）
pub fn gen_interceptor_chain_fn(ctx: &CodeGenContext) -> TokenStream2 {
    let struct_name = &ctx.struct_name;
    if ctx.comp_attr.interceptors.is_empty() {
        return quote! {};
    }
    quote! {
        impl #struct_name {
            #[doc(hidden)]
            fn interceptor_chain() -> &'static ::tx_di_core::aop::InterceptorChain {
                static CHAIN: ::std::sync::OnceLock<::tx_di_core::aop::InterceptorChain> =
                    ::std::sync::OnceLock::new();
                CHAIN.get().expect(
                    "[di] interceptor chain 未初始化"
                )
            }
        }
    }
}

/// 生成 `init` 覆写（初始化拦截器链 + 调用用户 app_init）
pub fn gen_interceptor_init_override(ctx: &CodeGenContext) -> TokenStream2 {
    let interceptors = &ctx.comp_attr.interceptors;
    if interceptors.is_empty() {
        return quote! {};
    }

    let struct_name = &ctx.struct_name;
    let has_app_init = ctx.comp_attr.has_app_init;

    // 为每个拦截器类型生成 inject 和 push 代码
    let push_code: Vec<TokenStream2> = interceptors
        .iter()
        .enumerate()
        .map(|(i, ty)| {
            let var_name = Ident::new(&format!("_interceptor_{}", i), proc_macro2::Span::call_site());
            quote! {
                let #var_name: ::std::sync::Arc<dyn ::tx_di_core::aop::Interceptor> =
                    ::tx_di_core::inject_from_store::<#ty>(&app.store);
                chain.push_arc(#var_name);
            }
        })
        .collect();

    // 用户 app_init 回调（如果有）
    let user_init = if has_app_init {
        quote! {
            let comp: ::std::sync::Arc<Self> = ::tx_di_core::inject_from_store(&app.store);
            self::app_init(comp, app)
        }
    } else {
        quote! { ::tx_di_core::RIE::Ok(()) }
    };

    quote! {
        #[inline]
        fn init(app: &::std::sync::Arc<::tx_di_core::App>) -> ::tx_di_core::RIE<()> {
            // 初始化拦截器链
            let mut chain = ::tx_di_core::aop::InterceptorChain::new();
            #(#push_code)*
            static CHAIN: ::std::sync::OnceLock<::tx_di_core::aop::InterceptorChain> =
                ::std::sync::OnceLock::new();
            CHAIN.set(chain).unwrap_or_else(|_| {
                panic!("[di] {}: interceptor chain 重复初始化", stringify!(#struct_name))
            });

            // 用户自定义 app_init（如果有）
            #user_init
        }
    }
}
