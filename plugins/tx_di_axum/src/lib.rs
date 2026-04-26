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
    use tx_di_core::{BuildContext,App};
    use tx_di_log;
    use super::*;

    #[tokio::test]
    async fn it_works() {
        // tx_di_log::LogConfig::init_sort();
        // D:\proj\tx_di\configs\di-config.toml
        // C:\a_me\proj\rust\tx_di\configs\di-config.toml
        let mut ctx = BuildContext::new(Some(r"C:\a_me\proj\rust\tx_di\configs\di-config.toml"));
        BuildContext::debug_registry().expect("TODO: panic message");
        ctx.build_and_run().await.expect("TODO: panic message");
    }
    
}
