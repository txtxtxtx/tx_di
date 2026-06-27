mod plugin;
mod operate_log;
mod interfaces;
pub mod error;
pub mod auth;

use tx_di_core::BuildContext;

#[allow(unused_imports)]
use admin_app;
#[allow(unused_imports)]
use admin_infra;
#[allow(unused_imports)]
use tx_di_axum;
#[allow(unused_imports)]
use tx_di_file;
#[allow(unused_imports)]
use tx_di_job;
#[allow(unused_imports)]
use tx_di_log;
#[allow(unused_imports)]
use tx_di_sa_token;
#[allow(unused_imports)]
use tx_di_toasty;
/// 确保 infra 层插件被编译引入
// #[allow(unused_imports)]
// use admin_infra::plugin::DbInitPlugin;
use tx_error::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    let config_path = r"C:\a_me\proj\rust\tx_di\examples\tx_admin\config\config.toml";
    // let config_path = r"D:\proj\tx_di\examples\tx_admin\config\config.toml";
    let app = BuildContext::new(Some(config_path)).build()?;
    let app = app.ins_run().await?;

    // 注册内置任务处理器
    {
        use tx_di_job::{ExecutionStatus, JobPlugin, JobResult};
        let job_plugin = app.inject::<JobPlugin>();
        job_plugin.register_handler("noop", |_param| JobResult {
            status: ExecutionStatus::Success,
            result: Some("ok".to_string()),
            error: None,
        });
        job_plugin.register_handler("echo", |param| JobResult {
            status: ExecutionStatus::Success,
            result: Some(param.unwrap_or("").to_string()),
            error: None,
        });
    }

    Ok(app.waiting_exit().await)
}
