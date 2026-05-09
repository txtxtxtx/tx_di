mod config;
mod comp;
mod bound;
mod e;
mod r;
mod layers;

pub use config::*;
pub use comp::*;
pub use r::R;
pub use layers::{add_arc_layer, add_layer};
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
