//! 文件聚合

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct File { pub id: u64, pub config_id: Option<u64>, pub name: Option<String>, pub file_path: String, pub url: String, pub file_type: Option<String>, pub size: i32, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<File>, anyhow::Error>;
    async fn find_page(&self, page: u64, page_size: u64) -> Result<(Vec<File>, u64), anyhow::Error>;
    async fn save(&self, file: &File) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
