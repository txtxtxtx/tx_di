//! 文件服务插件核心组件
//!
//! 封装文件存储后端的生命周期管理，支持多后端共存。
//! 系统配置的后端 key 以 `sys:` 为前缀，用户动态添加的以 `user:` 为前缀。

use crate::config::FileConfig;
use crate::error::FilePluginErr;
use crate::storage::{FileStorage, OpendalStorage};
use crate::{sys_key, SYS_PREFIX};
use dashmap::DashMap;
use std::sync::Arc;
use tx_di_core::App;
use tx_di_core::{Component, DepsTuple, RIE};
use tx_error::{AppError, AppResult};

/// 文件服务插件
///
/// 管理多个存储后端，提供动态添加/移除/查询的能力。
///
/// # DI 注入方式
///
/// ```rust,ignore
/// #[tx_comp(init)]
/// pub struct MyService {
///     pub file_plugin: Arc<FilePlugin>,  // 自动注入
/// }
///
/// impl MyService {
///     async fn do_something(&self) {
///         let storage = self.file_plugin.default_storage().unwrap();
///         storage.upload("test.txt", b"hello", None).await?;
///     }
/// }
/// ```
#[derive(Debug, Component)]
#[component(app_async_init, init_sort = i32::MIN + 3)]
pub struct FilePlugin {
    /// 配置引用
    pub config: Arc<FileConfig>,

    /// 存储后端容器（key 依前缀区分：`sys:` 系统，`user:` 用户自定义）
    #[tx_cst(DashMap::new())]
    pub backends: DashMap<String, Arc<dyn FileStorage>>,
}

impl FilePlugin {
    /// 获取指定 key 的存储后端
    pub fn get_storage(&self, key: &str) -> Option<Arc<dyn FileStorage>> {
        self.backends.get(key).map(|r| r.value().clone())
    }

    /// 获取默认存储后端（`sys:local`）
    pub fn default_storage(&self) -> Option<Arc<dyn FileStorage>> {
        self.get_storage(&sys_key("local"))
    }

    /// 添加一个存储后端
    ///
    /// - `key` 建议使用 `sys_key()` 或 `user_key()` 辅助方法构造
    /// - 如果 key 已存在，返回旧值
    pub fn add_storage(&self, key: String, storage: Arc<dyn FileStorage>) -> Option<Arc<dyn FileStorage>> {
        self.backends.insert(key, storage).map(|r| r.clone())
    }

    /// 移除一个用户自定义存储后端
    ///
    /// # 错误
    /// - `sys:` 前缀的后端不可移除，会返回 `FilePluginErr::CannotRemoveSystemStorage`
    /// - key 不存在会返回 `FilePluginErr::StorageNotFound`
    pub fn remove_storage(&self, key: &str) -> AppResult<Arc<dyn FileStorage>> {
        if key.starts_with(SYS_PREFIX) {
            return Err(AppError::with_context(
                FilePluginErr::CannotRemoveSystemStorage,
                key.to_string(),
            ));
        }
        self.backends
            .remove(key)
            .map(|(_, v)| v)
            .ok_or_else(|| {
                AppError::with_context(FilePluginErr::StorageNotFound, key.to_string())
            })
    }

    /// 列出所有存储后端 key
    pub fn storage_keys(&self) -> Vec<String> {
        self.backends.iter().map(|r| r.key().clone()).collect()
    }

    /// 列出指定前缀的存储后端 key
    pub fn storage_keys_by_prefix(&self, prefix: &str) -> Vec<String> {
        self.backends
            .iter()
            .filter(|r| r.key().starts_with(prefix))
            .map(|r| r.key().clone())
            .collect()
    }

    /// 列出所有系统存储后端 key
    pub fn sys_storage_keys(&self) -> Vec<String> {
        self.storage_keys_by_prefix(SYS_PREFIX)
    }

    /// 列出所有用户存储后端 key
    pub fn user_storage_keys(&self) -> Vec<String> {
        self.storage_keys_by_prefix(crate::USER_PREFIX)
    }
}

tx_di_core::async_method!(
    /// `#[component(app_async_init)]` 回调：初始化存储后端
    fn app_async_init(comp: Arc<FilePlugin>, _app: Arc<App>) -> RIE<()> {
        let config = comp.config.clone();

        if !comp.backends.is_empty() {
            tracing::warn!("FilePlugin: backends already initialized, skipping");
            return Ok(());
        }

        // ── 1. 注册系统默认本地存储 sys:local ──────────────
        let local = OpendalStorage::new_local(&config.base_path, &config.base_url)?;
        comp.backends.insert(sys_key("local"), Arc::new(local));

        // ── 2. 注册配置文件中的额外后端 sys:<name> ───────────
        for extra in &config.extra_storages {
            let key = sys_key(&extra.name);
            match OpendalStorage::from_storage_config(extra) {
                Ok(storage) => {
                    comp.backends.insert(key, Arc::new(storage));
                }
                Err(e) => {
                    tracing::error!(
                        name = %extra.name,
                        backend = ?extra.backend,
                        error = %e,
                        "额外存储后端初始化失败，跳过"
                    );
                }
            }
        }

        tracing::info!(
            local_path = %config.base_path,
            backend_count = comp.backends.len(),
            extra_count = config.extra_storages.len(),
            "文件存储后端已初始化"
        );

        Ok(())
    }
);
