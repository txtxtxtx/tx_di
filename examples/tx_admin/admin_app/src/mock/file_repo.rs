use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::file::model::aggregate::{File, FileConfig};
use admin_domain::file::model::value_object::FileQuery;
use admin_domain::file::repository::{FileConfigRepository, FileRepository};
use admin_domain::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

pub struct MockFileRepository {
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
    async fn find_by_id(&self, id: u64) -> Result<Option<File>, RepositoryError> {
        let files = self.files.read().unwrap();
        Ok(files.get(&id).filter(|f| f.audit.deleted == 0).cloned())
    }

    async fn find_page(
        &self,
        query: &FileQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<File>, RepositoryError> {
        let files = self.files.read().unwrap();
        let filtered: Vec<File> = files
            .values()
            .filter(|f| f.audit.deleted == 0)
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
            .take(page.page_size as usize)
            .collect();

        Ok(PageResponse::new(list, total, page.page, page.page_size))
    }

    async fn insert(&self, file: &File) -> Result<(), RepositoryError> {
        let mut files = self.files.write().unwrap();
        files.insert(file.id, file.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> Result<(), RepositoryError> {
        let mut files = self.files.write().unwrap();
        if let Some(file) = files.get_mut(&id) {
            file.audit.deleted = 1;
            Ok(())
        } else {
            Err(RepositoryError::NotFound(format!("File {} not found", id)))
        }
    }
}

pub struct MockFileConfigRepository {
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
    async fn find_by_id(&self, id: i32) -> Result<Option<FileConfig>, RepositoryError> {
        let configs = self.configs.read().unwrap();
        Ok(configs.get(&id).filter(|c| c.audit.deleted == 0).cloned())
    }

    async fn find_master(&self) -> Result<Option<FileConfig>, RepositoryError> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .find(|c| c.master == 1 && c.audit.deleted == 0)
            .cloned())
    }

    async fn find_all(&self) -> Result<Vec<FileConfig>, RepositoryError> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .filter(|c| c.audit.deleted == 0)
            .cloned()
            .collect())
    }

    async fn insert(&self, config: &FileConfig) -> Result<(), RepositoryError> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(config.id, config.clone());
        Ok(())
    }

    async fn update(&self, config: &FileConfig) -> Result<(), RepositoryError> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(config.id, config.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: i32) -> Result<(), RepositoryError> {
        let mut configs = self.configs.write().unwrap();
        if let Some(config) = configs.get_mut(&id) {
            config.audit.deleted = 1;
            Ok(())
        } else {
            Err(RepositoryError::NotFound(format!("FileConfig {} not found", id)))
        }
    }
}
