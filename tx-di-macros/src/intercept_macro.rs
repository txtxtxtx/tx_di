//! `#[intercept]` 属性宏实现
//!
//! 标记在 Component 的方法上，生成包裹代码：
//! 1. 构建 `CallContext`（含 Debug 参数）
//! 2. 调用 `Self::interceptor_chain().before_all(&ctx)`
//! 3. 执行业务逻辑
//! 4. 调用 `Self::interceptor_chain().after_all(&ctx, &mut result)`
//!
//! 支持 `async fn` 和非 `Result` 返回类型。
//! 不支持 `unsafe`、`extern` 函数。

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn intercept_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let output = generate_intercepted_fn(&input_fn);
    output.into()
}

fn generate_intercepted_fn(input_fn: &ItemFn) -> TokenStream2 {
    let fn_name = &input_fn.sig.ident;
    let vis = &input_fn.vis;
    let constness = input_fn.sig.constness;
    let unsafety = input_fn.sig.unsafety;
    let generics = &input_fn.sig.generics;
    let output = &input_fn.sig.output;
    let body = &input_fn.block;
    let is_async = input_fn.sig.asyncness.is_some();

    // 分离 self 和普通参数
    let mut regular_params: Vec<syn::Pat> = Vec::new();
    for param in &input_fn.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = param {
            regular_params.push(pat_type.pat.as_ref().clone());
        }
    }

    // 生成 with_arg 调用（只传 Debug 字符串，不传 raw）
    let arg_calls: Vec<TokenStream2> = regular_params.iter().map(|pat| {
        quote! {
            .with_arg(::tx_di_core::aop::ArgValue::Other(
                ::std::format!("{:?}", &#pat)
            ))
        }
    }).collect();

    let params = &input_fn.sig.inputs;

    // 检测返回类型是否为 Result（简化处理：总是尝试 Ok/Err 匹配）
    let is_result_ret = is_result_return_type(&input_fn.sig.output);

    let after_block = if is_result_ret {
        quote! {
            let mut __cr = match &__result {
                Ok(_) => ::tx_di_core::aop::CallResult::Ok,
                Err(e) => ::tx_di_core::aop::CallResult::Err(::std::format!("{}", e)),
            };
            __chain.after_all(&__ctx, &mut __cr);
        }
    } else {
        quote! {
            let mut __cr = ::tx_di_core::aop::CallResult::Ok;
            __chain.after_all(&__ctx, &mut __cr);
        }
    };

    let async_prefix = if is_async { quote! { async } } else { quote! {} };

    quote! {
        #vis #constness #unsafety #async_prefix fn #fn_name #generics (#params) #output {
            // Phase 1: before 拦截
            let __ctx = ::tx_di_core::aop::CallContext::new(stringify!(#fn_name))
                #(#arg_calls)*;

            let __key = self as *const Self as usize;
            let __chain = ::tx_di_core::aop::get_interceptor_chain(__key)
                .expect("[di] 拦截器链未初始化：请确认组件已通过 #[component(intercept(...))] 声明，且 App 已运行初始化阶段");
            __chain.before_all(&__ctx).unwrap_or_else(|e| {
                panic!("[di] 拦截器拒绝 method={}: {}", stringify!(#fn_name), e)
            });

            // Phase 2: 执行业务逻辑（包裹函数本身已是 async，原 body 直接在异步上下文中执行）
            let __result = #body;

            // Phase 3: after 拦截（可加工 CallResult）
            #after_block

            __result
        }
    }
}

/// 判断返回类型是否为 Result / RIE / AppResult（统一使用 RIE）
///
/// 因为 `RIE<T> = AppResult<T> = Result<T, AppError>`，
/// 所以返回 `RIE<T>` 的方法也支持 Ok/Err 匹配。
fn is_result_return_type(output: &syn::ReturnType) -> bool {
    match output {
        syn::ReturnType::Type(_, ty) => {
            let s = quote! { #ty }.to_string();
            // 直接写 Result、通过 RIE 别名、或完整路径
            s.starts_with("Result ") || s.starts_with("Result<")
                || s.starts_with("::std::result::Result ")
                || s.starts_with("::core::result::Result ")
                || s.starts_with("RIE ") || s.starts_with("RIE<")
                || s.starts_with("AppResult ") || s.starts_with("AppResult<")
                || s.starts_with("::tx_di_core::RIE ")
                || s.starts_with("::tx_di_core::RIE<")
        }
        _ => false,
    }
}
