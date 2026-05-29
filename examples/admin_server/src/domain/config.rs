//! 配置聚合

use serde::{Deserialize, Serialize};
use toasty::Model;

/// 配置类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum ConfigType {
    /// 系统内置
    #[column(variant = 1)]
    BuiltIn,
    /// 自定义
    #[column(variant = 2)]
    Custom,
}

/// 配置参数实体
#[derive(Debug, Clone, Model)]
#[table = "infra_config"]
pub struct Config {
    #[key]
    #[auto]
    pub id: u64,
    /// 配置分组
    pub category: Option<String>,
    /// 配置类型
    pub config_type: ConfigType,
    /// 配置名称
    pub name: String,
    /// 配置键
    #[unique]
    pub config_key: String,
    /// 配置值
    pub value: Option<String>,
    /// 是否可见
    #[default(true)]
    pub visible: bool,
    /// 备注
    pub remark: Option<String>,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}

#[async_trait::async_trait]
pub trait ConfigRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Config>, anyhow::Error>;
    async fn find_by_key(&self, key: &str) -> Result<Option<Config>, anyhow::Error>;
    async fn find_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<Config>, u64), anyhow::Error>;
    async fn save(&self, config: &Config) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
