use tx_di_core::BuildContext;

#[allow(unused_imports)]
use tx_di_axum;
use tx_di_axum::WebConfig;

#[allow(unused_imports)]
use tx_di_log;
use tx_error::AppResult;
// #[allow(unused_imports)]
// use tx_di_file;
// #[allow(unused_imports)]
// use tx_di_toasty;
#[tokio::main]
async fn main() ->AppResult<()> {
    let config_path = r"D:\proj\tx_di\examples\tx_admin\config\config.toml";
    let app = BuildContext::new(Some(config_path)).build()?;
    let app = app.ins_run().await?;
    tracing::info!("管理后台已启动，配置:\n {:?}", app.inject::<WebConfig>());
    Ok(app.waiting_exit().await)
}
