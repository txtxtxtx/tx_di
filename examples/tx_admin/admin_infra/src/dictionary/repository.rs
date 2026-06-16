use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::dictionary::model::aggregate::{DictData, DictType};
use admin_domain::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use admin_domain::dictionary::repository::{DictDataRepository, DictTypeRepository};
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::{RepositoryError, db_err};
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::{SysDictData, SysDictType};
use crate::common::{Status, Deleted};

/// Toasty 实现的 DictTypeRepository
#[tx_comp(as_trait = dyn DictTypeRepository)]
pub struct ToastyDictTypeRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyDictTypeRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(d: &SysDictType) -> DictType {
        DictType::restore(
            d.id as u64,
            d.name.clone(),
            d.dict_type.clone(),
            i32::from(d.status),
            if d.remark.is_empty() { None } else { Some(d.remark.clone()) },
            AuditFields {
                creator: if d.creator.is_empty() { None } else { Some(d.creator.clone()) },
                create_time: d.created_at.parse().unwrap_or_default(),
                updater: if d.updater.is_empty() { None } else { Some(d.updater.clone()) },
                update_time: d.updated_at.parse().unwrap_or_default(),
                deleted: if d.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl DictTypeRepository for ToastyDictTypeRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<DictType>> {
        let mut db = self.plugin.db().clone();
        match SysDictType::get_by_id(&mut db, id as i64).await {
            Ok(d) if d.deleted == Deleted::No => Ok(Some(Self::to_domain(&d))),
            _ => Ok(None),
        }
    }

    async fn find_by_type(&self, dict_type: &str) -> AppResult<Option<DictType>> {
        let mut db = self.plugin.db().clone();
        let dt = SysDictType::filter_by_dict_type(dict_type)
            .first()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;
        match dt {
            Some(d) if d.deleted == Deleted::No => Ok(Some(Self::to_domain(&d))),
            _ => Ok(None),
        }
    }

    async fn find_page(&self, query: &DictTypeQuery, page: Page<DictType>) -> AppResult<Page<DictType>> {
        let mut db = self.plugin.db().clone();
        let all = SysDictType::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;

        let filtered: Vec<&SysDictType> = all
            .iter()
            .filter(|d| d.deleted == Deleted::No)
            .filter(|d| {
                if let Some(ref name) = query.name {
                    if !d.name.contains(name.as_str()) { return false; }
                }
                if let Some(ref dict_type) = query.dict_type {
                    if !d.dict_type.contains(dict_type.as_str()) { return false; }
                }
                if let Some(status) = query.status {
                    if i32::from(d.status) != status { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let list: Vec<DictType> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(Self::to_domain)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn find_all(&self, query: &DictTypeQuery) -> AppResult<Vec<DictType>> {
        let mut db = self.plugin.db().clone();
        let all = SysDictType::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;

        Ok(all
            .iter()
            .filter(|d| d.deleted == Deleted::No)
            .filter(|d| {
                if let Some(ref name) = query.name {
                    if !d.name.contains(name.as_str()) { return false; }
                }
                if let Some(ref dict_type) = query.dict_type {
                    if !d.dict_type.contains(dict_type.as_str()) { return false; }
                }
                if let Some(status) = query.status {
                    if i32::from(d.status) != status { return false; }
                }
                true
            })
            .map(Self::to_domain)
            .collect())
    }

    async fn insert(&self, dict_type: &DictType) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysDictType::create()
            .id(dict_type.id as i64)
            .name(dict_type.name.clone())
            .dict_type(dict_type.dict_type.clone())
            .status(Status::from(dict_type.status))
            .remark(dict_type.remark.clone().unwrap_or_default())
            .creator(dict_type.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(dict_type.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(dict_type.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;
        Ok(())
    }

    async fn update(&self, dict_type: &DictType) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysDictType::get_by_id(&mut db, dict_type.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFoundDict)?;

        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .name(dict_type.name.clone())
            .dict_type(dict_type.dict_type.clone())
            .status(Status::from(dict_type.status))
            .remark(dict_type.remark.clone().unwrap_or_default())
            .updater(dict_type.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(dict_type.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut dict_type = SysDictType::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFoundDict)?;

        dict_type.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;
        Ok(())
    }

    async fn exists_by_type(&self, dict_type: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let dt = SysDictType::filter_by_dict_type(dict_type)
            .first()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;
        Ok(dt.map(|d| d.deleted == Deleted::No).unwrap_or(false))
    }
}

/// Toasty 实现的 DictDataRepository
#[tx_comp(as_trait = dyn DictDataRepository)]
pub struct ToastyDictDataRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyDictDataRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(d: &SysDictData) -> DictData {
        DictData::restore(
            d.id as u64,
            d.sort,
            d.label.clone(),
            d.value.clone(),
            d.dict_type.clone(),
            i32::from(d.status),
            if d.color_type.is_empty() { None } else { Some(d.color_type.clone()) },
            if d.css_class.is_empty() { None } else { Some(d.css_class.clone()) },
            if d.remark.is_empty() { None } else { Some(d.remark.clone()) },
            AuditFields {
                creator: if d.creator.is_empty() { None } else { Some(d.creator.clone()) },
                create_time: d.created_at.parse().unwrap_or_default(),
                updater: if d.updater.is_empty() { None } else { Some(d.updater.clone()) },
                update_time: d.updated_at.parse().unwrap_or_default(),
                deleted: if d.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl DictDataRepository for ToastyDictDataRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<DictData>> {
        let mut db = self.plugin.db().clone();
        match SysDictData::get_by_id(&mut db, id as i64).await {
            Ok(d) if d.deleted == Deleted::No => Ok(Some(Self::to_domain(&d))),
            _ => Ok(None),
        }
    }

    async fn find_by_type(&self, dict_type: &str) -> AppResult<Vec<DictData>> {
        let mut db = self.plugin.db().clone();
        let all = SysDictData::filter_by_dict_type(dict_type)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;

        Ok(all
            .iter()
            .filter(|d| d.deleted == Deleted::No)
            .map(Self::to_domain)
            .collect())
    }

    async fn find_by_types(&self, dict_types: &[String]) -> AppResult<Vec<DictData>> {
        let mut db = self.plugin.db().clone();
        let all = SysDictData::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;

        Ok(all
            .iter()
            .filter(|d| d.deleted == Deleted::No && dict_types.contains(&d.dict_type))
            .map(Self::to_domain)
            .collect())
    }

    async fn find_page(&self, query: &DictDataQuery, page: Page<DictData>) -> AppResult<Page<DictData>> {
        let mut db = self.plugin.db().clone();
        let all = SysDictData::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;

        let filtered: Vec<&SysDictData> = all
            .iter()
            .filter(|d| d.deleted == Deleted::No)
            .filter(|d| {
                if let Some(ref dict_type) = query.dict_type {
                    if d.dict_type != *dict_type { return false; }
                }
                if let Some(ref label) = query.label {
                    if !d.label.contains(label.as_str()) { return false; }
                }
                if let Some(status) = query.status {
                    if i32::from(d.status) != status { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let list: Vec<DictData> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(Self::to_domain)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn insert(&self, data: &DictData) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysDictData::create()
            .id(data.id as i64)
            .sort(data.sort)
            .label(data.label.clone())
            .value(data.value.clone())
            .dict_type(data.dict_type.clone())
            .status(Status::from(data.status))
            .color_type(data.color_type.clone().unwrap_or_default())
            .css_class(data.css_class.clone().unwrap_or_default())
            .remark(data.remark.clone().unwrap_or_default())
            .creator(data.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(data.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(data.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;
        Ok(())
    }

    async fn update(&self, data: &DictData) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysDictData::get_by_id(&mut db, data.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFoundDict)?;

        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .sort(data.sort)
            .label(data.label.clone())
            .value(data.value.clone())
            .dict_type(data.dict_type.clone())
            .status(Status::from(data.status))
            .color_type(data.color_type.clone().unwrap_or_default())
            .css_class(data.css_class.clone().unwrap_or_default())
            .remark(data.remark.clone().unwrap_or_default())
            .updater(data.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(data.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut data = SysDictData::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFoundDict)?;

        data.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseDict))?;
        Ok(())
    }
}
