//! 拦截器链初始化代码生成
//!
//! `gen_static_and_helper` 生成模块级的 static + 辅助函数；
//! `gen_init_override` 生成 impl Component 内的 init 覆写。

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::codegen::CodeGenContext;

/// 生成类型级拦截器链 static + `__get_chain` 辅助函数
///
/// 这些必须作为模块级 item 输出（不能放 impl 块内），供 `#[intercept]` 方法引用。
pub fn gen_static_and_helper(ctx: &CodeGenContext) -> TokenStream2 {
    if ctx.comp_attr.interceptors.is_empty() {
        return quote! {};
    }

    quote! {
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static __INTERCEPTOR_CHAIN: ::std::sync::OnceLock<::std::sync::Arc<::tx_di_core::aop::InterceptorChain>>
            = ::std::sync::OnceLock::new();

        #[doc(hidden)]
        #[inline]
        #[allow(non_snake_case)]
        fn __get_chain() -> &'static ::std::sync::Arc<::tx_di_core::aop::InterceptorChain> {
            __INTERCEPTOR_CHAIN.get().expect("拦截器链未初始化：请确认 init 阶段已执行")
        }
    }
}

/// 生成 impl Component 内的 init 覆写
pub fn gen_init_override(ctx: &CodeGenContext) -> TokenStream2 {
    let interceptors = &ctx.comp_attr.interceptors;
    if interceptors.is_empty() {
        return quote! {};
    }

    let has_app_init = ctx.comp_attr.has_app_init;

    let push_code: Vec<TokenStream2> = interceptors
        .iter()
        .enumerate()
        .map(|(i, ty)| {
            let var_name = Ident::new(
                &format!("_interceptor_{}", i),
                proc_macro2::Span::call_site(),
            );
            quote! {
                let #var_name: ::std::sync::Arc<dyn ::tx_di_core::aop::Interceptor> =
                    ::tx_di_core::inject_from_store::<#ty>(&app.store);
                chain.push_arc(#var_name);
            }
        })
        .collect();

    let user_init = if has_app_init {
        quote! { self::app_init(comp, app)?; }
    } else {
        quote! {}
    };

    quote! {
        #[inline]
        fn init(app: &::std::sync::Arc<::tx_di_core::App>) -> ::tx_di_core::RIE<()> {
            let comp: ::std::sync::Arc<Self> = ::tx_di_core::inject_from_store(&app.store);
            let mut chain = ::tx_di_core::aop::InterceptorChain::new();
            #(#push_code)*
            __INTERCEPTOR_CHAIN
                .set(::std::sync::Arc::new(chain))
                .map_err(|_| ::tx_di_core::AppError::with_context(
                    ::tx_di_core::DiErr::InjectError,
                    "拦截器链重复初始化（init 被多次调用）",
                ))?;
            #user_init
            Ok(())
        }
    }
}
