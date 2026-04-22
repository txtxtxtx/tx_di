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
        let ctx = BuildContext::new(Some("configs/test_log.toml"));
        BuildContext::debug_registry();
        sleep(std::time::Duration::from_secs(1));
        debug!("测试日志框架加载");
        info!("测试日志框架加载");
        warn!("测试日志框架加载");
        error!("测试日志框架加载");
    }
}
