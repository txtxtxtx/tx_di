//! 应用入口
//!
//! 组件模块与插件注册统一放在 `lib`（`src/lib.rs`），bin 仅负责启动循环：
//! `app_loop!` 负责：启动拉取 Nacos 配置并合并本地 → 启动 App →
//! 注册 http/gRPC 端点 → 监听（退出信号 / 配置变更）→ 配置变更时
//! 优雅关闭 App（进程不退出）并用新配置重启。

// 引用 lib：触发 `AdminPlugin` 等组件 linkme 注册（见 src/lib.rs）
#[allow(unused_imports)]
use admin_api;
use tx_error::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    tx_di_nacos::app_loop! {
        config = resolve_config_path(),
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

/// 解析本地配置文件路径。
///
/// 优先级：环境变量 `CONFIG_PATH` > 当前目录 `config/config.toml` >
/// 仓库约定路径 `examples/tx_admin/config/config.toml`（向后兼容开发习惯）。
/// 生产部署推荐显式设置 `CONFIG_PATH`（绝对路径），避免依赖进程工作目录。
fn resolve_config_path() -> &'static str {
    if let Ok(p) = std::env::var("CONFIG_PATH") {
        if !p.trim().is_empty() {
            return Box::leak(p.into_boxed_str());
        }
    }
    if std::path::Path::new("config/config.toml").exists() {
        return "config/config.toml";
    }
    "examples/tx_admin/config/config.toml"
}
