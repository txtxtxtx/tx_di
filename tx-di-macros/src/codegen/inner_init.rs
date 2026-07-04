//! 生成 `inner_init` 方法
//!
//! 当存在 trait inject 字段或 `#[component(init)]` 时，覆盖默认的
//! `inner_init` 实现：
//! - 可选 trait inject 字段：`self.field = Some(inject_trait_from_store(...))`
//! - 必选 trait inject 字段：`ptr::write` 覆盖零值占位
//! - `#[component(init)]`：调用用户定义的 `__di_component_init`

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::codegen::CodeGenContext;

/// 生成 `inner_init` 方法实现；无需覆盖时返回空 TokenStream
pub fn gen_inner_init(ctx: &CodeGenContext) -> TokenStream2 {
    let has_trait_inject =
        !ctx.trait_inject_fields.is_empty() || !ctx.required_trait_fields.is_empty();

    if !has_trait_inject && !ctx.comp_attr.has_init {
        return quote! {};
    }

    // 可选 trait inject 字段赋值
    let optional_assigns: Vec<TokenStream2> = ctx
        .trait_inject_fields
        .iter()
        .map(|(fname, ty)| {
            quote! {
                self.#fname = Some(::tx_di_core::inject_trait_from_store::<#ty>(store));
            }
        })
        .collect();

    // 必选 trait inject 字段赋值（ptr::write 覆盖零值占位，避免 drop 无效 Arc）
    let required_assigns: Vec<TokenStream2> = ctx
        .required_trait_fields
        .iter()
        .map(|(fname, ty)| {
            quote! {
                // 用 ptr::write 覆盖零值占位，避免 drop 无效 Arc
                unsafe {
                    ::core::ptr::write(
                        &mut self.#fname,
                        ::tx_di_core::inject_trait_from_store::<#ty>(store),
                    );
                }
            }
        })
        .collect();

    if ctx.comp_attr.has_init {
        quote! {
            fn inner_init(&mut self, store: &::tx_di_core::Store) -> ::tx_di_core::RIE<()> {
                #( #optional_assigns )*
                #( #required_assigns )*
                __di_component_init(self, store)
            }
        }
    } else {
        quote! {
            fn inner_init(&mut self, store: &::tx_di_core::Store) -> ::tx_di_core::RIE<()> {
                #( #optional_assigns )*
                #( #required_assigns )*
                Ok(())
            }
        }
    }
}
