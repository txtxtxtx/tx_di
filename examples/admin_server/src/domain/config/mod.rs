//! 配置聚合

use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum ConfigType {
    #[column(variant = 0)] Custom,
    #[column(variant = 1)] BuiltIn,
}
impl std::fmt::Display for ConfigType { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { ConfigType::BuiltIn => write!(f, "built_in"), ConfigType::Custom => write!(f, "custom") } } }

#[derive(Debug, Clone)]
pub struct Config { pub id: u64, pub category: Option<String>, pub config_type: ConfigType, pub name: String, pub config_key: String, pub value: Option<String>, pub visible: bool, pub remark: Option<String>, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[async_trait]
pub trait ConfigRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Config>, anyhow::Error>;
    async fn find_by_key(&self, key: &str) -> Result<Option<Config>, anyhow::Error>;
    async fn find_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<Config>, u64), anyhow::Error>;
    async fn save(&self, config: &Config) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
