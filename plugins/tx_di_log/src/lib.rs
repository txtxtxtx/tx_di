mod config;
mod comp;

use std::thread::sleep;
use log::{debug, error, info, warn};
pub use config::*;
pub use comp::*;

#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use log::{debug, error, info, warn};
    use tx_di_core::BuildContext;
    use super::*;

    #[test]
    fn it_works() {
        // Some(r"D:\proj\tx_di\configs\test_log.toml")
        //
        let ctx = BuildContext::new(Some(r"D:\proj\tx_di\configs\test_log.toml"));
        BuildContext::debug_registry();
        debug!("测试日志框架加载");
        info!("测试日志框架加载");
        warn!("测试日志框架加载");
        error!("测试日志框架加载");
    }
}
