//! `app_loop!` 宏 — 应用启动编排循环
//!
//! 将「拉取远程配置 → 与本地合并 → 启动 App → 注册端点 → 等待
//! （退出信号 / 配置变更）→ 优雅关闭（不退出进程）→ 重启」封装为单个宏，
//! 调用方无需关心配置中心与重启编排细节。
//!
//! **不创建 tokio runtime**：须在 `#[tokio::main] async fn main()` 中使用，
//! 由外层 `#[tokio::main]` 驱动。
//!
//! # 用法
//!
//! ```rust,ignore
//! #[tokio::main]
//! async fn main() -> AppResult<()> {
//!     tx_di_nacos::app_loop! {
//!         config = r"config/config.toml",
//!         startup = |app: std::sync::Arc<tx_di_core::App>| -> tx_di_core::RIE<()> {
//!             // ins_run 完成后的业务初始化（job handler 注册等）
//!             Ok(())
//!         },
//!     }
//! }
//! ```
//!
//! # 行为
//!
//! - `[registry_config].enabled = true`：启动拉取 + 合并配置，监听主配置变更，
//!   变更时优雅关闭当前 App（进程不退出）并用新配置重启。
//! - `enabled = false` 或 Nacos 不可达：退化为「本地配置启动一次 + 传统退出」，行为与
//!   `BuildContext::new(path)?.build()?.ins_run()` + `waiting_exit()` 一致。

#[macro_export]
macro_rules! app_loop {
    (
        config = $config_path:expr
        $(, startup = $startup:expr)?
        $(,)?
    ) => {{
        use std::sync::Arc;
        use tx_di_core::{App, BuildContext};
        use tx_di_nacos::NacosClient;

        let __config_path: &str = $config_path;

        // 1. 读取本地 bootstrap（仅 [registry_config]）
        let __bootstrap = tx_di_nacos::load_bootstrap(__config_path)?;

        // 2. 连接配置中心（单连接；enabled=false → None 纯本地模式）
        let __client = NacosClient::connect_if_enabled(&__bootstrap).await?;
        let __data_id = __bootstrap.config_data_id();
        let mut __config_rx = match &__client {
            Some(c) => Some(c.watch_config(&__data_id)),
            None => None,
        };

        // 3. 主循环：启动 → 等待 → 优雅关闭 → 重启
        loop {
            // 3.1 拉取 + 合并（远程覆盖本地；失败/不存在 → 用本地）
            let __local = tx_di_nacos::load_local_toml(__config_path)?;
            let __merged = match &__client {
                Some(c) => {
                    let __remote = c.pull_config(&__data_id).await?;
                    match c.merge_config(__local.clone(), __remote) {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!("远程配置解析失败，使用本地配置: {e}");
                            __local
                        }
                    }
                }
                None => __local,
            };

            // 3.2 启动 App（组件按合并后配置初始化；init+async_init 已完成）
            let __app = BuildContext::with_config(__merged)?
                .build()?
                .ins_run()
                .await?;

            // 3.3 业务初始化（可选）
            $( $startup(__app.clone())?; )?

            // 3.4 注册 http/gRPC 端点（插件侧 register_endpoints → 取走）
            let __instance_id = match &__client {
                Some(c) => {
                    let __eps = tx_di_nacos::take_endpoints();
                    if __eps.is_empty() {
                        tracing::warn!("未注册任何端点，跳过服务注册");
                        None
                    } else {
                        Some(c.register_service(__eps).await?)
                    }
                }
                None => None,
            };

            // 3.5 等待：退出信号 / 主配置变更
            let __exit = tokio::select! {
                _ = App::wait_exit_signal() => true,
                _ = async {
                    match __config_rx.as_mut() {
                        Some(rx) => {
                            let _ = rx.changed().await;
                        }
                        None => std::future::pending::<()>().await,
                    }
                } => false,
            };

            // 3.6 注销服务 + 优雅关闭当前实例（不退出进程）
            if let (Some(c), Some(id)) = (&__client, &__instance_id) {
                if let Err(e) = c.deregister(id).await {
                    tracing::warn!("服务注销失败: {e}");
                }
            }
            __app.graceful_shutdown().await?;

            if __exit {
                break;
            }
            tracing::info!("配置已变更，使用新配置重启中...");
        }

        tx_di_core::RIE::Ok(())
    }};
}
