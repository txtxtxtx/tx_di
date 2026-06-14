use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::file::model::aggregate::{File, FileConfig};
use admin_domain::file::model::value_object::FileQuery;
use admin_domain::file::repository::{FileConfigRepository, FileRepository};
use admin_domain::shared::repository::RepositoryError;
use admin_domain::shared::model::value_object::DeletedStatus;
use tx_common::page::Page;
use tx_di_core::{tx_comp, tx_cst};
use tx_error::AppResult;

#[tx_comp(as_trait = dyn FileRepository)]
pub struct MockFileRepository {
    #[tx_cst(RwLock::new(HashMap::new()))]
    files: RwLock<HashMap<u64, File>>,
}

impl MockFileRepository {
    pub fn new() -> Self {
        Self {
            files: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MockFileRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileRepository for MockFileRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<File>> {
        let files = self.files.read().unwrap();
        Ok(files.get(&id).filter(|f| f.audit.deleted == DeletedStatus::Normal).cloned())
    }

    async fn find_page(
        &self,
        query: &FileQuery,
        page: Page<File>,
    ) -> AppResult<Page<File>> {
        let files = self.files.read().unwrap();
        let filtered: Vec<File> = files
            .values()
            .filter(|f| f.audit.deleted == DeletedStatus::Normal)
            .filter(|f| {
                if let Some(ref name) = query.name {
                    if !f.name.contains(name.as_str()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let list = filtered
            .into_iter()
            .skip(offset)
            .take(page.size as usize)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn insert(&self, file: &File) -> AppResult<()> {
        let mut files = self.files.write().unwrap();
        files.insert(file.id, file.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut files = self.files.write().unwrap();
        if let Some(file) = files.get_mut(&id) {
            file.audit.deleted = DeletedStatus::Deleted;
            Ok(())
        } else {
            Err(RepositoryError::NotFound)?
        }
    }

    async fn find_file_path(&self, id: u64) -> AppResult<String> {
        let files = self.files.read().unwrap();
        files.get(&id)
            .filter(|f| f.audit.deleted == DeletedStatus::Normal)
            .map(|f| f.path.clone())
            .ok_or_else(|| RepositoryError::NotFound.into())
    }
}

#[tx_comp(as_trait = dyn FileConfigRepository)]
pub struct MockFileConfigRepository {
    #[tx_cst(RwLock::new(HashMap::new()))]
    configs: RwLock<HashMap<i32, FileConfig>>,
}

impl MockFileConfigRepository {
    pub fn new() -> Self {
        Self {
            configs: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MockFileConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileConfigRepository for MockFileConfigRepository {
    async fn find_by_id(&self, id: i32) -> AppResult<Option<FileConfig>> {
        let configs = self.configs.read().unwrap();
        Ok(configs.get(&id).filter(|c| c.audit.deleted == DeletedStatus::Normal).cloned())
    }

    async fn find_master(&self) -> AppResult<Option<FileConfig>> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .find(|c| c.master == 1 && c.audit.deleted == DeletedStatus::Normal)
            .cloned())
    }

    async fn find_all(&self) -> AppResult<Vec<FileConfig>> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .filter(|c| c.audit.deleted == DeletedStatus::Normal)
            .cloned()
            .collect())
    }

    async fn insert(&self, config: &FileConfig) -> AppResult<()> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(config.id, config.clone());
        Ok(())
    }

    async fn update(&self, config: &FileConfig) -> AppResult<()> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(config.id, config.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: i32) -> AppResult<()> {
        let mut configs = self.configs.write().unwrap();
        if let Some(config) = configs.get_mut(&id) {
            config.audit.deleted = DeletedStatus::Deleted;
            Ok(())
        } else {
            Err(RepositoryError::NotFound)?
        }
    }
}
