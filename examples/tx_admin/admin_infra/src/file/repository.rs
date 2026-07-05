use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::file::model::aggregate::{File, FileConfig};
use admin_domain::file::model::value_object::FileQuery;
use admin_domain::file::repository::{FileConfigRepository, FileRepository};
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::{RepositoryError, db_err};
use tx_common::page::Page;
use tx_di_core::{Component, DepsTuple};
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::{SysFile, SysFileConfig};
use crate::common::{Deleted, StorageType};

/// Toasty 实现的 FileRepository
#[derive(Component)]
#[component(as_trait = dyn FileRepository)]
pub struct ToastyFileRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyFileRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(f: &SysFile) -> File {
        File::restore(
            f.id,
            if f.config_id == 0 { None } else { Some(f.config_id) },
            f.name.clone(),
            f.file_path.clone(),
            f.url.clone(),
            if f.file_type.is_empty() { None } else { Some(f.file_type.clone()) },
            f.size,
            AuditFields {
                creator: if f.creator.is_empty() { None } else { Some(f.creator.clone()) },
                create_time: f.created_at,
                updater: if f.updater.is_empty() { None } else { Some(f.updater.clone()) },
                update_time: f.updated_at,
                deleted: if f.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl FileRepository for ToastyFileRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<File>> {
        let mut db = self.plugin.db().clone();
        match SysFile::get_by_id(&mut db, id).await {
            Ok(f) if f.deleted == Deleted::No => Ok(Some(Self::to_domain(&f))),
            _ => Ok(None),
        }
    }

    async fn find_page(&self, query: &FileQuery, page: Page<File>) -> AppResult<Page<File>> {
        let mut db = self.plugin.db().clone();
        let all = SysFile::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseFile))?;

        let filtered: Vec<&SysFile> = all
            .iter()
            .filter(|f| f.deleted == Deleted::No)
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
        SysFile::create()
            .id(file.id)
            .config_id(file.config_id.unwrap_or(0))
            .name(file.name.clone())
            .file_path(file.path.clone())
            .url(file.url.clone())
            .file_type(file.file_type.clone().unwrap_or_default())
            .size(file.size)
            .creator(file.audit.creator.clone().unwrap_or_default())
            .updater(file.audit.updater.clone().unwrap_or_default())
            .deleted(Deleted::from(file.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseFile))?;
        Ok(())
    }

    async fn update(&self, file: &File) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysFile::get_by_id(&mut db, file.id)
            .await
            .map_err(|_| RepositoryError::NotFoundFile)?;
        existing
            .update()
            .name(file.name.clone())
            .file_path(file.path.clone())
            .url(file.url.clone())
            .file_type(file.file_type.clone().unwrap_or_default())
            .size(file.size)
            .updater(file.audit.updater.clone().unwrap_or_default())
            .deleted(Deleted::from(file.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseFile))?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut file = SysFile::get_by_id(&mut db, id)
            .await
            .map_err(|_| RepositoryError::NotFoundFile)?;

        file.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseFile))?;
        Ok(())
    }

    async fn find_file_path(&self, id: u64) -> AppResult<String> {
        let mut db = self.plugin.db().clone();
        let file = SysFile::get_by_id(&mut db, id)
            .await
            .map_err(|_| RepositoryError::NotFoundFile)?;

        if file.deleted != Deleted::No {
            return Err(RepositoryError::NotFoundFile.into());
        }
        Ok(file.file_path)
    }
}

/// Toasty 实现的 FileConfigRepository
#[derive(Component)]
#[component(as_trait = dyn FileConfigRepository)]
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
            c.storage.into(),
            if c.remark.is_empty() { None } else { Some(c.remark.clone()) },
            c.master,
            c.config.clone(),
            AuditFields {
                creator: if c.creator.is_empty() { None } else { Some(c.creator.clone()) },
                create_time: c.created_at,
                updater: if c.updater.is_empty() { None } else { Some(c.updater.clone()) },
                update_time: c.updated_at,
                deleted: if c.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl FileConfigRepository for ToastyFileConfigRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<FileConfig>> {
        let mut db = self.plugin.db().clone();
        match SysFileConfig::get_by_id(&mut db, id).await {
            Ok(c) if c.deleted == Deleted::No => Ok(Some(Self::to_domain(&c))),
            _ => Ok(None),
        }
    }

    async fn find_master(&self) -> AppResult<Option<FileConfig>> {
        let mut db = self.plugin.db().clone();
        let all = SysFileConfig::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseFile))?;

        Ok(all
            .iter()
            .find(|c| c.deleted == Deleted::No && c.master == 1)
            .map(Self::to_domain))
    }

    async fn find_all(&self) -> AppResult<Vec<FileConfig>> {
        let mut db = self.plugin.db().clone();
        let all = SysFileConfig::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseFile))?;

        Ok(all
            .iter()
            .filter(|c| c.deleted == Deleted::No)
            .map(Self::to_domain)
            .collect())
    }

    async fn insert(&self, config: &FileConfig) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        SysFileConfig::create()
            .id(config.id)
            .name(config.name.clone())
            .storage(StorageType::from(config.storage))
            .remark(config.remark.clone().unwrap_or_default())
            .master(config.master)
            .config(config.config.clone())
            .creator(config.audit.creator.clone().unwrap_or_default())
            .updater(config.audit.updater.clone().unwrap_or_default())
            .deleted(Deleted::from(config.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseFile))?;
        Ok(())
    }

    async fn update(&self, config: &FileConfig) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysFileConfig::get_by_id(&mut db, config.id)
            .await
            .map_err(|_| RepositoryError::NotFoundFile)?;

        existing
            .update()
            .name(config.name.clone())
            .storage(StorageType::from(config.storage))
            .remark(config.remark.clone().unwrap_or_default())
            .master(config.master)
            .config(config.config.clone())
            .updater(config.audit.updater.clone().unwrap_or_default())
            .deleted(Deleted::from(config.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseFile))?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut config = SysFileConfig::get_by_id(&mut db, id)
            .await
            .map_err(|_| RepositoryError::NotFoundFile)?;

        config.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseFile))?;
        Ok(())
    }
}
