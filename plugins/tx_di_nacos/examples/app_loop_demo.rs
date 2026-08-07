//! `app_loop!` 完整链路演示：启动拉取配置 → 启动 App → 监听配置变更 → 优雅重启
//!
//! 用法：
//!   1. 设置环境变量（或修改下方 RegistryConfig）：
//!      NACOS_USER=nacos NACOS_PASS=xxx
//!   2. 运行：cargo run -p tx_di_nacos --example app_loop_demo
//!   3. 观察日志：App 启动 #N（每次配置变更后重启，N 递增）
//!   4. 用 Nacos 控制台 / API 修改配置 `tx_admin_app_loop.toml`，
//!      观察进程不退出并打印"配置已变更，使用新配置重启中..."
//!
//! 退出：Ctrl+C（等待退出信号后 break）

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tx_di_core::{Component, DepsTuple, RIE};

/// 演示组件：每次启动打印启动次数
#[derive(Component)]
#[component(init)]
pub struct AppLoopDemo {
    /// 全局启动计数器（进程内累计，重启不清零，用于观察重启次数）
    #[tx_cst(Arc::new(AtomicU32::new(0)))]
    pub boot_counter: Arc<AtomicU32>,
}

fn init(this: &mut AppLoopDemo, _store: &tx_di_core::Store) -> RIE<()> {
    let n = this.boot_counter.fetch_add(1, Ordering::SeqCst) + 1;
    println!("[demo] App 启动 #{n}");
    Ok(())
}

#[tokio::main]
async fn main() -> tx_error::AppResult<()> {
    // 初始化 tracing（让宏内日志可见）
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tx_di_nacos::app_loop! {
        config = r"plugins/tx_di_nacos/examples/app_loop_demo.toml",
        startup = |app: Arc<App>| -> RIE<()> {
            let demo = app.inject::<AppLoopDemo>();
            println!("[demo] startup 回调：boot_counter={}", demo.boot_counter.load(Ordering::SeqCst));
            Ok(())
        },
    }
}
