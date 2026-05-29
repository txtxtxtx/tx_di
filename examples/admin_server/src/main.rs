//! admin_server — DDD 后台管理系统入口

mod admin_plugin;
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interfaces;

use tx_di_core::BuildContext;

// 引入插件以触发 linkme 自动注册
#[allow(unused_imports)]
use tx_di_axum;
use tx_di_axum::WebConfig;
#[allow(unused_imports)]
use tx_di_log;
#[allow(unused_imports)]
use tx_di_file;
#[allow(unused_imports)]
use tx_di_toasty;
/// 启动管理后台服务
///
/// ```bash
/// cargo run -p admin_server
/// ```
#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let config_path = std::env::var("ADMIN_CONFIG")
        .ok()
        .or_else(|| {
            // 在当前目录及上级目录寻找 config/config.toml
            let candidates = [
                "examples/admin_server/config/config.toml",
                "config/config.toml",
            ];
            candidates
                .iter()
                .find(|p| std::path::Path::new(p).exists())
                .map(|s| s.to_string())
        });

    let app = BuildContext::new(config_path.as_deref())
        .build()?
        .ins_run()
        .await?;

    tracing::info!("管理后台已启动，访问 {}",app.inject::<WebConfig>().address());

    app.waiting_exit().await;
    Ok(())
}
