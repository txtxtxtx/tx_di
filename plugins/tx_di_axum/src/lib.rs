pub mod bound;
mod comp;
mod config;
pub mod e;
pub mod err;
mod layers;
mod metrics;

pub use bound::DiComp;
pub use comp::*;
pub use config::*;
pub use e::WebErr;
pub use err::WebErrCode;
pub use layers::{
    add_arc_layer, add_layer, body_limit::BodySizeLimitLayer, security::SecurityHeadersLayer,
};
pub use metrics::{MetricsLayer, metrics_router, register_collector};

/// 统一路由器类型
///
/// 用户始终使用 `tx_di_axum::Router` 注册路由。
pub type Router = axum::Router;
#[cfg(test)]
mod tests {
    use tx_di_core::BuildContext;
    #[allow(unused)]
    // use super::*;
    #[tokio::test]
    async fn it_works() {
        // D:\proj\tx_di\configs\di-config.toml
        // C:\a_me\proj\rust\tx_di\configs\di-config.toml
        let ctx = BuildContext::new(Some(r"D:\proj\tx_di\configs\di-config.toml")).unwrap();
        // 运行 app
        let app = ctx.build().unwrap().ins_run().await.unwrap();
        // 等待退出
        app.waiting_exit().await;
    }
}
