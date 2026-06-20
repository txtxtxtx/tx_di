mod plugin;
mod interfaces;
pub mod error;
pub mod auth;
pub mod scheduler;

use tx_di_core::BuildContext;

#[allow(unused_imports)]
use tx_di_axum;
#[allow(unused_imports)]
use tx_di_log;
#[allow(unused_imports)]
use admin_app;
#[allow(unused_imports)]
use admin_infra;
#[allow(unused_imports)]
use tx_di_toasty;
#[allow(unused_imports)]
use tx_di_sa_token;
#[allow(unused_imports)]
use tx_di_file;

use tx_error::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    let config_path = r"C:\a_me\proj\rust\tx_di\examples\tx_admin\config\config.toml";
    // let config_path = r"D:\proj\tx_di\examples\tx_admin\config\config.toml";
    let app = BuildContext::new(Some(config_path)).build()?;
    let app = app.ins_run().await?;

    Ok(app.waiting_exit().await)
}
