//! `#[intercept]` 属性宏实现
//!
//! 执行流程：CallContext → before 检查 → body → after → 返回原值

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ImplItemFn, Pat};

pub fn intercept_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ImplItemFn);

    let visibility = &input.vis;
    let constness = input.sig.constness.as_ref();
    let unsafety = input.sig.unsafety.as_ref();
    let is_async = input.sig.asyncness.is_some();
    let fn_name = &input.sig.ident;
    let generics = &input.sig.generics;
    let params = &input.sig.inputs;
    let output = &input.sig.output;
    let body = &input.block;

    // ── 参数生成 ──────────────────────────────────────────────────
    let mut arg_calls = Vec::new();
    for param in params.iter() {
        if let FnArg::Typed(pt) = param {
            if let Pat::Ident(pat_ident) = &*pt.pat {
                let arg_name = pat_ident.ident.to_string();
                let arg_ident = &pat_ident.ident;
                let ty = &pt.ty;
                let arg_val = gen_arg_value(arg_ident, ty);
                arg_calls.push(quote! {
                    .with_arg(#arg_name, #arg_val)
                });
            }
        }
    }

    let is_result_ret = is_result_type(output);

    // ── before ────────────────────────────────────────────────────
    let before_block = if is_result_ret {
        quote! {
            if let Err(e) = __chain.before_all(&__ctx) {
                return ::std::result::Result::Err(::std::convert::Into::into(e));
            }
        }
    } else {
        quote! {
            __chain.before_all(&__ctx).unwrap_or_else(|e| {
                panic!("[di] 拦截器拒绝 method={}: {}", stringify!(#fn_name), e)
            });
        }
    };

    // ── body + after ──────────────────────────────────────────────
    // body 包入闭包防 return/? 绕过 after
    let body_then_after = if is_result_ret {
        quote! {
            let __body_result = #body;
            match &__body_result {
                Ok(_v) => {
                    let __res = ::tx_di_core::aop::CallResult::Ok;
                    __chain.after_all(&__ctx, &__res);
                }
                Err(e) => {
                    let msg = format!("{}", e);
                    let __res = ::tx_di_core::aop::CallResult::Err(msg);
                    __chain.after_all(&__ctx, &__res);
                }
            }
            __body_result
        }
    } else {
        quote! {
            let __body_result = #body;
            __chain.after_all(&__ctx, &::tx_di_core::aop::CallResult::Ok);
            __body_result
        }
    };

    let async_prefix = if is_async { quote! { async } } else { quote! {} };

    let output_tokens = quote! {
        #visibility #constness #unsafety #async_prefix fn #fn_name #generics (#params) #output {
            let __ctx = ::tx_di_core::aop::CallContext::new(stringify!(#fn_name)) #(#arg_calls)*;
            let __chain = __get_chain();
            #before_block
            #body_then_after
        }
    };

    output_tokens.into()
}

// ── 辅助 ──────────────────────────────────────────────────────────────────

fn gen_arg_value(arg_ident: &syn::Ident, ty: &syn::Type) -> proc_macro2::TokenStream {
    let ty_str = quote! { #ty }.to_string();

    if ty_str == "i64" || ty_str == "i32" || ty_str == "i16" || ty_str == "i8" {
        return quote! { ::tx_di_core::aop::ArgValue::I64(#arg_ident as i64) };
    }
    if ty_str == "u64" || ty_str == "u32" || ty_str == "u16" || ty_str == "u8" || ty_str == "usize" {
        return quote! { ::tx_di_core::aop::ArgValue::U64(#arg_ident as u64) };
    }
    if ty_str == "f64" || ty_str == "f32" {
        return quote! { ::tx_di_core::aop::ArgValue::F64(#arg_ident as f64) };
    }
    if ty_str == "bool" {
        return quote! { ::tx_di_core::aop::ArgValue::Bool(#arg_ident) };
    }
    if ty_str == "String" || (ty_str.starts_with("&") && ty_str.ends_with("str")) {
        return quote! { ::tx_di_core::aop::ArgValue::Str(#arg_ident.to_string()) };
    }

    quote! {
        {
            let __json = ::tx_di_core::serde_json::to_string(&#arg_ident)
                .unwrap_or_else(|_| "<序列化失败>".to_string());
            ::tx_di_core::aop::ArgValue::Serialized {
                type_id: ::std::any::TypeId::of::<#ty>(),
                type_name: ::std::any::type_name::<#ty>(),
                json: __json,
            }
        }
    }
}

fn is_result_type(output: &syn::ReturnType) -> bool {
    match output {
        syn::ReturnType::Type(_, ty) => {
            if let syn::Type::Path(tp) = ty.as_ref() {
                if let Some(seg) = tp.path.segments.last() {
                    return seg.ident == "Result";
                }
            }
            false
        }
        syn::ReturnType::Default => false,
    }
}
