mod plugin;
mod interfaces;

use tx_di_core::BuildContext;

#[allow(unused_imports)]
use tx_di_axum;
#[allow(unused_imports)]
use tx_di_log;
#[allow(unused_imports)]
use admin_app;
use tx_error::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    let config_path = r"D:\proj\tx_di\examples\tx_admin\config\config.toml";
    let app = BuildContext::new(Some(config_path)).build()?;
    let app = app.ins_run().await?;

    Ok(app.waiting_exit().await)
}
