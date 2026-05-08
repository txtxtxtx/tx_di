mod config;
mod comp;
mod bound;
mod e;
mod r;
mod layers;

pub use config::*;
pub use comp::*;
pub use r::R;
#[cfg(test)]
mod tests {
    use tx_di_core::{BuildContext};
    #[allow(unused)]
    use tx_di_log;
    // use super::*;

    #[tokio::test]
    async fn it_works() {
        // tx_di_log::LogConfig::init_sort();
        // D:\proj\tx_di\configs\di-config.toml
        // C:\a_me\proj\rust\tx_di\configs\di-config.toml
        let ctx = BuildContext::new(Some(r"C:\a_me\proj\rust\tx_di\configs\di-config.toml"));
        let app = ctx.build().expect("TODO: panic message");
        let app = app.ins_run().await.expect("TODO: panic message");
        app.waiting_exit().await;
        // BuildContext::debug_registry().expect("TODO: panic message");
        // ctx.build_and_run().await.expect("TODO: panic message");
    }
    
}
