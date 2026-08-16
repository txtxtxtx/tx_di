//! 应用入口
//!
//! 组件模块与插件注册统一放在 `lib`（`src/lib.rs`），bin 仅负责启动循环：
//! `app_loop!` 负责：启动拉取 Nacos 配置并合并本地 → 启动 App →
//! 注册 http/gRPC 端点 → 监听（退出信号 / 配置变更）→ 配置变更时
//! 优雅关闭 App（进程不退出）并用新配置重启。

// 引用 lib：触发 `AdminPlugin` 等组件 linkme 注册（见 src/lib.rs）
// 依赖 linkme 的链接副作用，不能删除；clippy 无法识别，需显式抑制该 lint。
#[allow(unused_imports, clippy::single_component_path_imports)]
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
/// 优先级：
/// 1. 环境变量 `CONFIG_PATH`（显式指定，相对路径基于 CWD 绝对化，不检查存在性）
/// 2. 按存在性探测候选路径（覆盖不同 CWD 运行场景）：
///    - `<CWD>/config/config.toml`（从 `examples/tx_admin` 目录运行）
///    - `<CWD>/configs/config.toml`
///    - `<CWD>/examples/tx_admin/config/config.toml`（从仓库根运行）
/// 3. 兜底 `<CWD>/examples/tx_admin/config/config.toml`（不存在时由后续读取报错定位）
///
/// 统一返回**绝对路径**，避免后续 `app_loop!` / `AppAllConfig`
/// 读取时因进程工作目录变化而失效。
fn resolve_config_path() -> &'static str {
    // 1. 显式指定：直接用，不探测存在性（用户显式指定路径即应以此为准）
    if let Ok(p) = std::env::var("CONFIG_PATH")
        && !p.trim().is_empty()
    {
        return Box::leak(absolutize(&p).into_boxed_str());
    }

    // 2. 候选路径：先直接判断（等价于 CWD 下），再按 CWD.join 判断
    let cwd = std::env::current_dir().ok();
    let candidates = [
        "config/config.toml"
    ];
    for cand in candidates {
        let path = std::path::Path::new(cand);
        let exists = path.exists()
            || cwd
                .as_ref()
                .map(|c| c.join(cand).exists())
                .unwrap_or(false);
        if exists {
            return Box::leak(absolutize(cand).into_boxed_str());
        }
    }

    // 3. 兜底：返回约定路径（绝对化），由后续读取报错定位
    Box::leak(absolutize("config/config.toml").into_boxed_str())
}

/// 将路径转换为绝对路径（相对路径基于进程 CWD 锚定；CWD 获取失败时原样返回）。
fn absolutize(raw: &str) -> String {
    let path = std::path::Path::new(raw);
    if path.is_absolute() {
        return raw.to_string();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(raw).to_string_lossy().into_owned())
        .unwrap_or_else(|_| raw.to_string())
}
