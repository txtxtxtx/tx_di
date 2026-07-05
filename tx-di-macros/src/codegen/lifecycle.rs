//! 生成生命周期方法覆写（init / async_init / async_run / shutdown）
//!
//! 当 `#[component(...)]` 中指定了 `app_init` / `app_async_init` /
//! `app_async_run` / `shutdown` 标志时，生成对应的 `fn` 覆盖，
//! 委托给用户定义的同名回调函数：
//!
//! | 属性 | 回调函数 | 覆写的 trait 方法 |
//! |------|----------|-------------------|
//! | `app_init` | `app_init(comp: Arc<Self>, app: &Arc<App>)` | `init` |
//! | `app_async_init` | `app_async_init(comp: Arc<Self>, app: &Arc<App>)` | `async_init` |
//! | `app_async_run` | `app_async_run(comp: Arc<Self>, app: &Arc<App>, token)` | `async_run` |
//! | `shutdown` | `shutdown(&self)` | `shutdown` |

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

/// 生成 `#[inline] fn init(app: &Arc<App>) -> RIE<()>` 覆写
///
/// 从 App 的 Store 中取出组件实例 `Arc<Self>`，
/// 传递给用户定义的 `app_init(comp, app)`。
/// 使用 `self::` 前缀明确引用模块级自由函数，避免与 trait 方法 `Component::init` 冲突。
fn gen_app_init_impl(ctx: &CodeGenContext) -> TokenStream2 {
    if !ctx.comp_attr.has_app_init {
        return quote! {};
    }
    quote! {
        #[inline]
        fn init(app: &::std::sync::Arc<::tx_di_core::App>) -> ::tx_di_core::RIE<()> {
            let comp: ::std::sync::Arc<Self> = ::tx_di_core::inject_from_store(&app.store);
            self::app_init(comp, app)
        }
    }
}

/// 生成 `#[inline] fn async_init(app: &Arc<App>) -> BoxFuture<RIE<()>>` 覆写
fn gen_app_async_init_impl(ctx: &CodeGenContext) -> TokenStream2 {
    if !ctx.comp_attr.has_app_async_init {
        return quote! {};
    }
    quote! {
        #[inline]
        fn async_init(app: &::std::sync::Arc<::tx_di_core::App>) -> ::tx_di_core::BoxFuture<::tx_di_core::RIE<()>> {
            let comp: ::std::sync::Arc<Self> = ::tx_di_core::inject_from_store(&app.store);
            self::app_async_init(comp, app)
        }
    }
}

/// 生成 `#[inline] fn async_run(app, token) -> BoxFuture<RIE<()>>` 覆写
fn gen_app_async_run_impl(ctx: &CodeGenContext) -> TokenStream2 {
    if !ctx.comp_attr.has_app_async_run {
        return quote! {};
    }
    quote! {
        #[inline]
        fn async_run(
            app: &::std::sync::Arc<::tx_di_core::App>,
            token: ::tx_di_core::CancellationToken,
        ) -> ::tx_di_core::BoxFuture<::tx_di_core::RIE<()>> {
            let comp: ::std::sync::Arc<Self> = ::tx_di_core::inject_from_store(&app.store);
            self::app_async_run(comp, app, token)
        }
    }
}

/// 生成 `#[inline] fn shutdown(&self)` 覆写
///
/// `shutdown` 既是回调名也是 trait 方法名（签名相同），
/// 使用 `self::` 前缀强制指向模块级自由函数，避免递归。
fn gen_shutdown_impl(ctx: &CodeGenContext) -> TokenStream2 {
    if !ctx.comp_attr.has_shutdown {
        return quote! {};
    }
    quote! {
        #[inline]
        fn shutdown(&self) {
            self::shutdown(self)
        }
    }
}
