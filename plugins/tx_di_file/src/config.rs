//! 文件存储配置

use serde::{Deserialize, Serialize};
use tx_di_core::{Component, RIE, Store};

/// 存储后端类型
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    /// 本地文件系统
    Local,
    /// AWS S3（或兼容 S3 协议的对象存储，如 MinIO）
    S3,
    /// 数据库存储（文件内容存入 infrust_file_content 表）
    Database,
}

impl Default for StorageBackend {
    fn default() -> Self {
        Self::Local
    }
}

impl From<i32> for StorageBackend {
    fn from(v: i32) -> Self {
        match v {
            1 => Self::S3,
            2 => Self::Database,
            0 => Self::Local,
            _ => unreachable!("只能是 0 1 2 "),
        }
    }
}

impl From<StorageBackend> for i32 {
    fn from(b: StorageBackend) -> i32 {
        match b {
            StorageBackend::Local => 0,
            StorageBackend::S3 => 1,
            StorageBackend::Database => 2,
        }
    }
}

/// S3 存储后端配置
#[derive(Debug, Clone, Deserialize, Serialize)]
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

/// 单个存储后端的完整配置
///
/// 用于 TOML `[[file_config.extra_storages]]` 数组配置，
/// 也用于从 DB JSON 反序列化创建动态后端。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    /// 后端名称（映射为 `sys:<name>` 或 `user:<name>`）
    pub name: String,
    /// 存储后端类型
    #[serde(default)]
    pub backend: StorageBackend,
    /// 本地存储根路径（backend = "local" 时生效）
    #[serde(default)]
    pub base_path: String,
    /// 文件访问基础 URL
    #[serde(default)]
    pub base_url: String,
    /// S3 配置（backend = "s3" 时生效）
    #[serde(default)]
    pub s3: S3Config,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            backend: StorageBackend::Local,
            base_path: "uploads".to_string(),
            base_url: String::new(),
            s3: S3Config::default(),
        }
    }
}

/// 文件存储统一配置
///
/// ```toml
/// [file_config]
/// base_path = "./uploads"
/// base_url = "http://localhost:8080/files"
/// max_file_size = 10485760    # 10MB
/// allowed_extensions = ["jpg", "png", "pdf"]
///
/// # 额外存储后端（可选，可多个）
/// [[file_config.extra_storages]]
/// name = "s3-main"
/// backend = "s3"
/// bucket = "my-bucket"
/// region = "ap-southeast-1"
/// endpoint = "http://localhost:9000"
/// ```
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf, init, init_sort = i32::MIN + 3)]
pub struct FileConfig {
    /// 本地存储根路径（sys:local 使用）
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

    /// 额外存储后端列表
    #[serde(default)]
    pub extra_storages: Vec<StorageConfig>,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            base_path: default_base_path(),
            base_url: String::new(),
            max_file_size: 0,
            allowed_extensions: Vec::new(),
            extra_storages: vec![],
        }
    }
}

/// `#[component(init)]` 回调：配置加载后打印日志
fn init(this: &mut FileConfig, _store: &Store) -> RIE<()> {
    tracing::info!(
        base_path = %this.base_path,
        max_file_size = this.max_file_size,
        "文件存储配置已加载"
    );
    Ok(())
}

fn default_base_path() -> String {
    "./uploads".to_string()
}
