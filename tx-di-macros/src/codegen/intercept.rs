//! 生成 AOP 拦截器相关代码
//!
//! - 生成 `interceptor_chain()` 关联函数（返回 `Mutex<Option<Arc<InterceptorChain>>>`）
//! - 生成 `init` 覆写：注入拦截器并初始化链

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::codegen::CodeGenContext;

/// 生成 `init` 覆写（初始化拦截器链 + 调用用户 app_init）
///
/// 拦截器链按「组件实例指针」存入全局表（见 `tx_di_core::aop::set_interceptor_chain`），
/// 由 `#[intercept]` 方法通过 `self` 指针取出，从而支持同进程多 App 互不干扰。
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
            self::app_init(comp, app)
        }
    } else {
        quote! { ::tx_di_core::RIE::Ok(()) }
    };

    quote! {
        #[inline]
        fn init(app: &::std::sync::Arc<::tx_di_core::App>) -> ::tx_di_core::RIE<()> {
            let comp: ::std::sync::Arc<Self> = ::tx_di_core::inject_from_store(&app.store);
            let mut chain = ::tx_di_core::aop::InterceptorChain::new();
            #(#push_code)*
            let __key = ::std::sync::Arc::as_ptr(&comp) as usize;
            ::tx_di_core::aop::set_interceptor_chain(__key, ::std::sync::Arc::new(chain));
            #user_init
        }
    }
}
