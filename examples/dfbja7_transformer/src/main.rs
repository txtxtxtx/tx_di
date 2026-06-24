//! # dfbja7_transformer
//!
//! NANO4S设备协议解析与MQTT转发服务
//!
//! 使用 tx-di-core 框架实现依赖注入和生命周期管理。

mod config;
mod model;
mod mqtt;
mod protocol;
mod server;
mod util;

use tx_di_core::BuildContext;

// 必须导入插件 crate 触发 linkme 注册
#[allow(unused_imports)]
use tx_di_log;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // let path = r"D:\proj\tx_di\examples\dfbja7_transformer\config\config.toml";
    let app = BuildContext::new(None::<String>)
        .build()?
        .ins_run()
        .await?;
    Ok(app.waiting_exit().await)
}
