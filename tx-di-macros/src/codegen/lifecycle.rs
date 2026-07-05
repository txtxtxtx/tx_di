//! 生成生命周期方法覆写（init / async_init / async_run / shutdown）
//!
//! 当 `#[component(...)]` 中指定了 `app_init` / `app_async_init` /
//! `app_async_run` / `shutdown` 标志时，生成对应的 `fn` 覆盖，
//! 委托给用户定义的回调函数：
//!
//! | 属性 | 回调函数 | 覆写的 trait 方法 |
//! |------|----------|-------------------|
//! | `app_init` | `__di_component_app_init(comp: Arc<Self>, app: &Arc<App>)` | `init` |
//! | `app_async_init` | `__di_component_async_init(comp: Arc<Self>, app: &Arc<App>)` | `async_init` |
//! | `app_async_run` | `__di_component_async_run(comp: Arc<Self>, app: &Arc<App>, token)` | `async_run` |
//! | `shutdown` | `__di_component_shutdown(&self)` | `shutdown` |

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::codegen::CodeGenContext;

/// 生成所有生命周期覆写的 TokenStream，按需拼接到 impl 块中
pub fn gen_lifecycle_overrides(ctx: &CodeGenContext) -> LifecycleOverrides {
    let app_init = gen_app_init_impl(ctx);
    let app_async_init = gen_app_async_init_impl(ctx);
    let app_async_run = gen_app_async_run_impl(ctx);
    let shutdown = gen_shutdown_impl(ctx);
    LifecycleOverrides {
        app_init,
        app_async_init,
        app_async_run,
        shutdown,
    }
}

pub struct LifecycleOverrides {
    pub app_init: TokenStream2,
    pub app_async_init: TokenStream2,
    pub app_async_run: TokenStream2,
    pub shutdown: TokenStream2,
}

/// 生成 `fn init(app: &Arc<App>) -> RIE<()>` 覆写
///
/// 从 App 的 Store 中取出组件实例 `Arc<Self>`，
/// 传递给用户定义的 `__di_component_app_init(comp, app)`。
fn gen_app_init_impl(ctx: &CodeGenContext) -> TokenStream2 {
    if !ctx.comp_attr.has_app_init {
        return quote! {};
    }
    quote! {
        fn init(app: &::std::sync::Arc<::tx_di_core::App>) -> ::tx_di_core::RIE<()> {
            let comp: ::std::sync::Arc<Self> = ::tx_di_core::inject_from_store(&app.store);
            __di_component_app_init(comp, app)
        }
    }
}

/// 生成 `fn async_init(app: &Arc<App>) -> BoxFuture<RIE<()>>` 覆写
fn gen_app_async_init_impl(ctx: &CodeGenContext) -> TokenStream2 {
    if !ctx.comp_attr.has_app_async_init {
        return quote! {};
    }
    quote! {
        fn async_init(app: &::std::sync::Arc<::tx_di_core::App>) -> ::tx_di_core::BoxFuture<::tx_di_core::RIE<()>> {
            let comp: ::std::sync::Arc<Self> = ::tx_di_core::inject_from_store(&app.store);
            __di_component_async_init(comp, app)
        }
    }
}

/// 生成 `fn async_run(app, token) -> BoxFuture<RIE<()>>` 覆写
fn gen_app_async_run_impl(ctx: &CodeGenContext) -> TokenStream2 {
    if !ctx.comp_attr.has_app_async_run {
        return quote! {};
    }
    quote! {
        fn async_run(
            app: &::std::sync::Arc<::tx_di_core::App>,
            token: ::tx_di_core::CancellationToken,
        ) -> ::tx_di_core::BoxFuture<::tx_di_core::RIE<()>> {
            let comp: ::std::sync::Arc<Self> = ::tx_di_core::inject_from_store(&app.store);
            __di_component_async_run(comp, app, token)
        }
    }
}

/// 生成 `fn shutdown(&self)` 覆写
///
/// shutdown 的 `&self` 由 trait 方法签名自然提供，不绕 Arc。
fn gen_shutdown_impl(ctx: &CodeGenContext) -> TokenStream2 {
    if !ctx.comp_attr.has_shutdown {
        return quote! {};
    }
    quote! {
        fn shutdown(&self) {
            __di_component_shutdown(self)
        }
    }
}
