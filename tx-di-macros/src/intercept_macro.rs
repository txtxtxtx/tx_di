//! `#[intercept]` 属性宏实现
//!
//! 标记在 Component 的方法上，生成包裹代码：
//! 1. 构建 `CallContext`（含 Debug 参数 + 原始参数引用）
//! 2. 调用 `Self::interceptor_chain().before_all(&mut ctx)`
//! 3. 提取被拦截器覆写后的参数
//! 4. 执行业务逻辑
//! 5. 调用 `Self::interceptor_chain().after_all(&ctx, &result)`

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// `#[intercept]` 属性宏入口
pub fn intercept_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let output = generate_intercepted_fn(&input_fn);
    output.into()
}

/// 生成拦截包裹函数
fn generate_intercepted_fn(input_fn: &ItemFn) -> TokenStream2 {
    let fn_name = &input_fn.sig.ident;
    let vis = &input_fn.vis;
    let constness = input_fn.sig.constness;
    let unsafety = input_fn.sig.unsafety;
    let generics = &input_fn.sig.generics;
    let output = &input_fn.sig.output;
    let body = &input_fn.block;

    // 分离 self 参数和普通参数
    let mut regular_params: Vec<(syn::Pat, &syn::Type)> = Vec::new();

    for param in &input_fn.sig.inputs {
        match param {
            syn::FnArg::Receiver(_) => {}
            syn::FnArg::Typed(pat_type) => {
                let pat = pat_type.pat.as_ref().clone();
                let ty = &pat_type.ty;
                regular_params.push((pat, ty));
            }
        }
    }

    // 生成 mut 重声明（让参数可被覆写）
    let param_mut_decls: Vec<TokenStream2> = regular_params
        .iter()
        .map(|(pat, _)| {
            quote! { let mut #pat = #pat; }
        })
        .collect();

    // 生成 with_arg + with_raw 调用
    let arg_raw_calls: Vec<TokenStream2> = regular_params
        .iter()
        .map(|(pat, _)| {
            quote! {
                .with_arg(::tx_di_core::aop::ArgValue::Other(
                    ::core::format_args!("{:?}", &#pat).to_string()
                ))
                .with_raw(#pat.clone())
            }
        })
        .collect();

    // 生成参数提取代码（拦截器覆写后回写）
    let extraction_code: Vec<TokenStream2> = regular_params
        .iter()
        .enumerate()
        .map(|(i, (pat, ty))| {
            let idx = syn::Index::from(i);
            quote! {
                if let Some(v) = ctx.get_raw_mut::<#ty>(#idx) {
                    #pat = ::std::mem::take(v);
                }
            }
        })
        .collect();

    // 方法参数列表
    let params = &input_fn.sig.inputs;

    // 生成包裹函数
    let result = quote! {
        #vis #constness #unsafety fn #fn_name #generics (#params) #output {
            #(#param_mut_decls)*

            // Phase 1: before 拦截
            let mut ctx = ::tx_di_core::aop::CallContext::new(stringify!(#fn_name))
                #(#arg_raw_calls)*;

            Self::interceptor_chain().before_all(&mut ctx)
                .unwrap_or_else(|e| {
                    panic!("[di] 拦截器拒绝 method={}: {}", stringify!(#fn_name), e)
                });

            // 提取被拦截器覆写后的参数
            #(#extraction_code)*

            // Phase 2: 执行业务逻辑
            let __result = #body;

            // Phase 3: after 拦截
            let __ctx = ::tx_di_core::aop::CallContext::new(stringify!(#fn_name));
            match &__result {
                Ok(_) => Self::interceptor_chain().after_all(
                    &__ctx,
                    &::tx_di_core::aop::CallResult::Ok,
                ),
                Err(e) => Self::interceptor_chain().after_all(
                    &__ctx,
                    &::tx_di_core::aop::CallResult::Err(e.to_string()),
                ),
            }

            __result
        }
    };

    result
}
