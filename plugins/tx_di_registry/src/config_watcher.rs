//! 配置变更监听器
//!
//! 在 `RegistryPlugin::app_async_run` 中启动，持续监听 Nacos 配置变更，
//! 并通过 `DynamicConfig` 更新本地内存中的配置。

use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::traits::ConfigCenter;

/// 配置变更监听器
pub struct ConfigWatcher {
    config_center: Arc<dyn ConfigCenter>,
    /// 已订阅的 data_id 列表
    subscriptions: Vec<(String, String)>, // (data_id, group)
}

impl ConfigWatcher {
    /// 创建配置监听器
    pub fn new(config_center: Arc<dyn ConfigCenter>) -> Self {
        Self {
            config_center,
            subscriptions: Vec::new(),
        }
    }

    /// 订阅一个配置变更
    pub fn subscribe(&mut self, data_id: impl Into<String>, group: impl Into<String>) {
        self.subscriptions.push((data_id.into(), group.into()));
    }

    /// 启动监听循环（在 `app_async_run` 中调用）
    pub async fn run(self, token: CancellationToken) {
        if self.subscriptions.is_empty() {
            info!("ConfigWatcher: 无已订阅配置，跳过监听");
            return;
        }

        let cc = self.config_center;
        let subs = self.subscriptions;
        let tk = token.clone();

        for (data_id, group) in &subs {
            let did = data_id.clone();
            let grp = group.clone();
            let cc = cc.clone();
            let tk_sub = tk.clone();
            tokio::spawn(async move {
                info!("ConfigWatcher: 开始监听配置 {}/{}", did, grp);
                // 首次获取并记录
                match cc.get_config(&did, &grp).await {
                    Ok(Some(val)) => info!("ConfigWatcher: 首次加载 {}/{} = {}", did, grp, &val[..val.len().min(200)]),
                    Ok(None) => info!("ConfigWatcher: {}/{} 不存在", did, grp),
                    Err(e) => error!("ConfigWatcher: 首次获取 {}/{} 失败: {}", did, grp, e),
                }

                // 订阅变更
                let did_cb = did.clone();
                let grp_cb = grp.clone();
                let callback = move |new_val: String| {
                    info!("ConfigWatcher: 配置 {}/{} 已变更: {}", did_cb, grp_cb, &new_val[..new_val.len().min(200)]);
                    // TODO: 将 new_val 反序列化后更新对应 DynamicConfig
                };

                tokio::select! {
                    _ = cc.listen_config(&did, &grp, Box::new(callback)) => {},
                    _ = tk_sub.cancelled() => {
                        info!("ConfigWatcher: {}/{} 监听已取消", did, grp);
                    }
                }
            });
        }

        // 等待取消信号
        tk.cancelled().await;
        info!("ConfigWatcher: 所有监听已停止");
    }
}
