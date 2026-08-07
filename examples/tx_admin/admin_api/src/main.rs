mod plugin;
mod operate_log;
mod interfaces;
mod nacos;
pub mod error;
pub mod auth;

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
use tx_error::AppResult;

/// 应用入口
///
/// `app_loop!` 负责：启动拉取 Nacos 配置并合并本地 → 启动 App →
/// 注册 http/gRPC 端点 → 监听（退出信号 / 配置变更）→ 配置变更时
/// 优雅关闭 App（进程不退出）并用新配置重启。
#[tokio::main]
async fn main() -> AppResult<()> {
    tx_di_nacos::app_loop! {
        config = r"C:\a_me\proj\rust\tx_di\examples\tx_admin\config\config.toml",
        startup = |app: std::sync::Arc<tx_di_core::App>| -> tx_di_core::RIE<()> {
            // 注册内置任务处理器
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
            Ok(())
        },
    }
}
