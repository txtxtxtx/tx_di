//! 文件存储配置

use serde::Deserialize;
use tx_di_core::{tx_comp, CompInit, InnerContext, RIE};

/// 存储后端类型
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    /// 本地文件系统
    Local,
    /// AWS S3（或兼容 S3 协议的对象存储，如 MinIO）
    S3,
}

impl Default for StorageBackend {
    fn default() -> Self {
        Self::Local
    }
}

/// S3 存储后端配置
#[derive(Debug, Clone, Deserialize)]
pub struct S3Config {
    /// S3 Bucket 名称
    #[serde(default)]
    pub bucket: String,
    /// AWS 区域，如 `ap-southeast-1`
    #[serde(default)]
    pub region: String,
    /// S3 兼容端点（MinIO 等），留空使用 AWS 官方端点
    #[serde(default)]
    pub endpoint: String,
    /// Access Key（留空则使用默认凭证链）
    #[serde(default)]
    pub access_key: String,
    /// Secret Key（留空则使用默认凭证链）
    #[serde(default)]
    pub secret_key: String,
    /// 是否强制使用路径风格（MinIO 通常需要设为 true）
    #[serde(default)]
    pub force_path_style: bool,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            region: "ap-southeast-1".to_string(),
            endpoint: String::new(),
            access_key: String::new(),
            secret_key: String::new(),
            force_path_style: false,
        }
    }
}

/// 文件存储统一配置
///
/// ```toml
/// [file_config]
/// backend = "local"
/// base_path = "./uploads"
/// max_file_size = 10485760    # 10MB
/// allowed_extensions = ["jpg", "png", "pdf"]
/// [file_config.s3]
/// bucket = "my-bucket"
/// region = "ap-southeast-1"
/// ```
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
pub struct FileConfig {
    /// 存储后端：`"local"` 或 `"s3"`
    #[serde(default)]
    pub backend: StorageBackend,

    /// 本地存储根路径（backend = "local" 时生效）
    #[serde(default = "default_base_path")]
    pub base_path: String,

    /// 文件访问基础 URL（用于生成下载链接）
    #[serde(default)]
    pub base_url: String,

    /// 单个文件最大大小（字节），0 表示不限制
    #[serde(default)]
    pub max_file_size: u64,

    /// 允许的文件扩展名列表（小写），空列表表示不限制
    #[serde(default)]
    pub allowed_extensions: Vec<String>,

    /// S3 配置（backend = "s3" 时生效）
    #[serde(default)]
    pub s3: S3Config,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Local,
            base_path: default_base_path(),
            base_url: String::new(),
            max_file_size: 0,
            allowed_extensions: Vec::new(),
            s3: S3Config::default(),
        }
    }
}

impl CompInit for FileConfig {
    fn inner_init(&mut self, _ctx: &InnerContext) -> RIE<()> {
        tracing::info!(
            backend = ?self.backend,
            base_path = %self.base_path,
            max_file_size = self.max_file_size,
            "文件存储配置已加载"
        );

        // 本地存储时确保目录存在
        if self.backend == StorageBackend::Local {
            std::fs::create_dir_all(&self.base_path).ok();
        }
        Ok(())
    }

    fn init_sort() -> i32 {
        i32::MIN + 3
    }
}

fn default_base_path() -> String {
    "./uploads".to_string()
}
