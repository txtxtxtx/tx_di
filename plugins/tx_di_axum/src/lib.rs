mod config;
mod comp;
pub mod bound;
pub mod e;
pub mod err;
mod layers;

pub use config::*;
pub use comp::*;
pub use bound::DiComp;
pub use e::WebErr;
pub use err::WebErrCode;
pub use layers::{add_arc_layer, add_layer};

/// aide 重新导出，方便用户使用 `JsonSchema` 等派生宏
#[cfg(feature = "api-doc")]
pub use aide;

/// 统一路由器类型
///
/// - `api-doc` 启用时：`aide::axum::ApiRouter`（自动生成 OpenAPI 文档）
/// - `api-doc` 禁用时：`axum::Router`
///
/// 用户始终使用 `tx_di_axum::Router` 注册路由，无需关心底层实现。
#[cfg(feature = "api-doc")]
pub type Router = aide::axum::ApiRouter;

#[cfg(not(feature = "api-doc"))]
pub type Router = axum::Router;
#[cfg(test)]
mod tests {
    use tx_di_core::{BuildContext};
    #[allow(unused)]
    use tx_di_log;
    // use super::*;

    #[tokio::test]
    async fn it_works() {
        // D:\proj\tx_di\configs\di-config.toml
        // C:\a_me\proj\rust\tx_di\configs\di-config.toml
        let ctx = BuildContext::new(Some(r"D:\proj\tx_di\configs\di-config.toml"));
        // 运行 app
        let app = ctx.build()
            .unwrap()
            .ins_run()
            .await.unwrap();
        // 等待退出
        app.waiting_exit().await;
    }
    
}
