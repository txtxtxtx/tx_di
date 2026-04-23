mod config;
mod comp;

pub use comp::*;
pub use config::*;

#[cfg(test)]
mod tests {
    use log::{debug, error, info, warn};
    use tx_di_core::BuildContext;

    #[test]
    fn it_works() {
        // Some(r"D:\proj\tx_di\configs\test_log.toml")
        //
        let _ctx = BuildContext::new(Some(r"D:\proj\tx_di\configs\test_log.toml"));
        BuildContext::debug_registry().expect("TODO: panic message");
        debug!("测试日志框架加载");
        info!("测试日志框架加载");
        warn!("测试日志框架加载");
        error!("测试日志框架加载");
    }
}
