use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::file::model::aggregate::{File, FileConfig};
use admin_domain::file::model::value_object::FileQuery;
use admin_domain::file::repository::{FileConfigRepository, FileRepository};
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::RepositoryError;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::{SysFile, SysFileConfig};

/// Toasty 实现的 FileRepository
#[tx_comp(as_trait = dyn FileRepository)]
pub struct ToastyFileRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyFileRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(f: &SysFile) -> File {
        File::restore(
            f.id as u64,
            if f.config_id == 0 { None } else { Some(f.config_id) },
            f.name.clone(),
            f.file_path.clone(),
            f.url.clone(),
            if f.file_type.is_empty() { None } else { Some(f.file_type.clone()) },
            f.size,
            AuditFields {
                creator: if f.creator.is_empty() { None } else { Some(f.creator.clone()) },
                create_time: f.created_at.parse().unwrap_or_default(),
                updater: if f.updater.is_empty() { None } else { Some(f.updater.clone()) },
                update_time: f.updated_at.parse().unwrap_or_default(),
                deleted: if f.deleted == 1 { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl FileRepository for ToastyFileRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<File>> {
        let mut db = self.plugin.db().clone();
        match SysFile::get_by_id(&mut db, id as i64).await {
            Ok(f) if f.deleted == 0 => Ok(Some(Self::to_domain(&f))),
            _ => Ok(None),
        }
    }

    async fn find_page(&self, query: &FileQuery, page: Page<File>) -> AppResult<Page<File>> {
        let mut db = self.plugin.db().clone();
        let all = SysFile::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        let filtered: Vec<&SysFile> = all
            .iter()
            .filter(|f| f.deleted == 0)
            .filter(|f| {
                if let Some(ref name) = query.name {
                    if !f.name.contains(name.as_str()) { return false; }
                }
                if let Some(ref file_type) = query.file_type {
                    if !f.file_type.contains(file_type.as_str()) { return false; }
                }
                if let Some(config_id) = query.config_id {
                    if f.config_id != config_id { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let list: Vec<File> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(Self::to_domain)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn insert(&self, file: &File) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysFile::create()
            .id(file.id as i64)
            .config_id(file.config_id.unwrap_or(0))
            .name(file.name.clone())
            .file_path(file.path.clone())
            .url(file.url.clone())
            .file_type(file.file_type.clone().unwrap_or_default())
            .size(file.size)
            .creator(file.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(file.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(file.audit.deleted as i32)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn update(&self, file: &File) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysFile::get_by_id(&mut db, file.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;
        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .name(file.name.clone())
            .file_path(file.path.clone())
            .url(file.url.clone())
            .file_type(file.file_type.clone().unwrap_or_default())
            .size(file.size)
            .updater(file.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(file.audit.deleted as i32)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut file = SysFile::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        file.update()
            .deleted(1)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn find_file_path(&self, id: u64) -> AppResult<String> {
        let mut db = self.plugin.db().clone();
        let file = SysFile::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        if file.deleted != 0 {
            return Err(RepositoryError::NotFound.into());
        }
        Ok(file.file_path)
    }
}

/// Toasty 实现的 FileConfigRepository
#[tx_comp(as_trait = dyn FileConfigRepository)]
pub struct ToastyFileConfigRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyFileConfigRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(c: &SysFileConfig) -> FileConfig {
        FileConfig::restore(
            c.id,
            c.name.clone(),
            c.storage,
            if c.remark.is_empty() { None } else { Some(c.remark.clone()) },
            c.master,
            c.config.clone(),
            AuditFields {
                creator: if c.creator.is_empty() { None } else { Some(c.creator.clone()) },
                create_time: c.created_at.parse().unwrap_or_default(),
                updater: if c.updater.is_empty() { None } else { Some(c.updater.clone()) },
                update_time: c.updated_at.parse().unwrap_or_default(),
                deleted: if c.deleted == 1 { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl FileConfigRepository for ToastyFileConfigRepository {
    async fn find_by_id(&self, id: i32) -> AppResult<Option<FileConfig>> {
        let mut db = self.plugin.db().clone();
        match SysFileConfig::get_by_id(&mut db, id).await {
            Ok(c) if c.deleted == 0 => Ok(Some(Self::to_domain(&c))),
            _ => Ok(None),
        }
    }

    async fn find_master(&self) -> AppResult<Option<FileConfig>> {
        let mut db = self.plugin.db().clone();
        let all = SysFileConfig::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .find(|c| c.deleted == 0 && c.master == 1)
            .map(Self::to_domain))
    }

    async fn find_all(&self) -> AppResult<Vec<FileConfig>> {
        let mut db = self.plugin.db().clone();
        let all = SysFileConfig::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|c| c.deleted == 0)
            .map(Self::to_domain)
            .collect())
    }

    async fn insert(&self, config: &FileConfig) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysFileConfig::create()
            .id(config.id)
            .name(config.name.clone())
            .storage(config.storage)
            .remark(config.remark.clone().unwrap_or_default())
            .master(config.master)
            .config(config.config.clone())
            .creator(config.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(config.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(config.audit.deleted as i32)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn update(&self, config: &FileConfig) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysFileConfig::get_by_id(&mut db, config.id)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .name(config.name.clone())
            .storage(config.storage)
            .remark(config.remark.clone().unwrap_or_default())
            .master(config.master)
            .config(config.config.clone())
            .updater(config.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(config.audit.deleted as i32)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn soft_delete(&self, id: i32) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut config = SysFileConfig::get_by_id(&mut db, id)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        config.update()
            .deleted(1)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }
}
