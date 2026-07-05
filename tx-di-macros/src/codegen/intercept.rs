//! 生成 AOP 拦截器相关代码
//!
//! - 生成 `interceptor_chain()` 关联函数（返回 `Mutex<Option<Arc<InterceptorChain>>>`）
//! - 生成 `init` 覆写：注入拦截器并初始化链

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::codegen::CodeGenContext;

/// 生成 `interceptor_chain()` 关联函数
pub fn gen_interceptor_chain_fn(ctx: &CodeGenContext) -> TokenStream2 {
    let struct_name = &ctx.struct_name;
    if ctx.comp_attr.interceptors.is_empty() {
        return quote! {};
    }
    quote! {
        impl #struct_name {
            #[doc(hidden)]
            fn interceptor_chain() -> &'static ::std::sync::Mutex<
                ::std::option::Option<::std::sync::Arc<::tx_di_core::aop::InterceptorChain>>
            > {
                static CHAIN: ::std::sync::Mutex<
                    ::std::option::Option<::std::sync::Arc<::tx_di_core::aop::InterceptorChain>>
                > = ::std::sync::Mutex::new(::std::option::Option::None);
                &CHAIN
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

    let has_app_init = ctx.comp_attr.has_app_init;

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
            let mut chain = ::tx_di_core::aop::InterceptorChain::new();
            #(#push_code)*
            *Self::interceptor_chain().lock().unwrap() = ::std::option::Option::Some(
                ::std::sync::Arc::new(chain)
            );
            #user_init
        }
    }
}
