//! 文件服务插件核心组件
//!
//! 封装文件存储后端的初始化和访问逻辑。
//! 通过 OpenDAL 统一支持本地文件系统和 S3 等多种后端。

use crate::config::FileConfig;
use crate::storage::{FileStorage, OpendalStorage};
use std::sync::{Arc, OnceLock};
use tokio_util::sync::CancellationToken;
use tx_di_core::App;
use tx_di_core::{CompInit, RIE, tx_comp};

/// 文件服务插件
///
/// 封装文件存储后端的生命周期管理，对外暴露 `Arc<dyn FileStorage>`。
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
///         let storage = self.file_plugin.storage();
///         storage.upload("test.txt", b"hello", None).await?;
///     }
/// }
/// ```
#[derive(Debug)]
#[tx_comp(init)]
pub struct FilePlugin {
    /// 配置引用
    pub config: Arc<FileConfig>,

    /// 存储后端（延迟初始化）
    #[tx_cst(OnceLock::new())]
    storage: OnceLock<Arc<dyn FileStorage>>,
}

impl FilePlugin {
    /// 获取存储后端实例
    ///
    /// 必须在 `async_init` 完成后调用。
    pub fn storage(&self) -> Arc<dyn FileStorage> {
        self.storage
            .get()
            .cloned()
            .expect("FilePlugin: 存储后端还未初始化")
    }
}

impl CompInit for FilePlugin {
    tx_di_core::async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let plugin = ctx.inject::<FilePlugin>();
            let config = plugin.config.clone();

            if plugin.storage.get().is_some() {
                tracing::warn!("FilePlugin: storage already initialized, skipping");
                return Ok(());
            }

            let storage: Arc<dyn FileStorage> = Arc::new(
                OpendalStorage::new(&config)
                    .map_err(|e| anyhow::anyhow!(e))?
            );

            if plugin.storage.set(storage).is_err() {
                tracing::warn!("FilePlugin: storage concurrently initialized");
            }

            Ok(())
        }
    );

    fn init_sort() -> i32 {
        i32::MIN + 3
    }
}
