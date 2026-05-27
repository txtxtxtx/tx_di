//! 文件服务插件核心组件
//!
//! 封装文件存储后端的初始化和访问逻辑。
//! 根据 `FileConfig.backend` 自动选择本地或 S3 存储后端。

use crate::config::FileConfig;
use crate::storage::{FileStorage, LocalFileStorage};
#[cfg(feature = "s3")]
use crate::storage::S3FileStorage;
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
///         storage.upload(...).await?;
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

            let storage: Arc<dyn FileStorage> = match config.backend {
                crate::config::StorageBackend::Local => {
                    tracing::info!(base_path = %config.base_path, "初始化本地文件存储");
                    Arc::new(LocalFileStorage::new(
                        &config.base_path,
                        &config.base_url,
                    ))
                }
                #[cfg(feature = "s3")]
                crate::config::StorageBackend::S3 => {
                    tracing::info!(
                        bucket = %config.s3.bucket,
                        region = %config.s3.region,
                        endpoint = %config.s3.endpoint,
                        "初始化 S3 文件存储"
                    );
                    let s3 = S3FileStorage::new(
                        &config.s3.bucket,
                        &config.s3.region,
                        if config.s3.endpoint.is_empty() { None } else { Some(&config.s3.endpoint) },
                        if config.s3.access_key.is_empty() { None } else { Some(&config.s3.access_key) },
                        if config.s3.secret_key.is_empty() { None } else { Some(&config.s3.secret_key) },
                        config.s3.force_path_style,
                    )
                    .await?;
                    Arc::new(s3)
                }
                #[cfg(not(feature = "s3"))]
                crate::config::StorageBackend::S3 => {
                    Err("S3 存储后端需要启用 's3' feature flag")?
                }
            };

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
