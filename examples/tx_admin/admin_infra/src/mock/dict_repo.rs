use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::dictionary::model::aggregate::{DictData, DictType};
use admin_domain::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use admin_domain::dictionary::repository::{DictDataRepository, DictTypeRepository};
use admin_domain::shared::repository::RepositoryError;
use admin_domain::shared::model::value_object::DeletedStatus;
use tx_common::page::Page;
use tx_di_core::{tx_comp, tx_cst};
use tx_error::AppResult;

#[tx_comp(as_trait = dyn DictTypeRepository)]
pub struct MockDictTypeRepository {
    #[tx_cst(RwLock::new(HashMap::new()))]
    dict_types: RwLock<HashMap<u64, DictType>>,
}

impl MockDictTypeRepository {
    pub fn new() -> Self {
        Self {
            dict_types: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MockDictTypeRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DictTypeRepository for MockDictTypeRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<DictType>> {
        let dict_types = self.dict_types.read().unwrap();
        Ok(dict_types.get(&id).filter(|d| d.audit.deleted == DeletedStatus::Normal).cloned())
    }

    async fn find_by_type(&self, dict_type: &str) -> AppResult<Option<DictType>> {
        let dict_types = self.dict_types.read().unwrap();
        Ok(dict_types
            .values()
            .find(|d| d.dict_type == dict_type && d.audit.deleted == DeletedStatus::Normal)
            .cloned())
    }

    async fn find_page(
        &self,
        query: &DictTypeQuery,
        page: Page<DictType>,
    ) -> AppResult<Page<DictType>> {
        let dict_types = self.dict_types.read().unwrap();
        let filtered: Vec<DictType> = dict_types
            .values()
            .filter(|d| d.audit.deleted == DeletedStatus::Normal)
            .filter(|d| {
                if let Some(ref name) = query.name {
                    if !d.name.contains(name.as_str()) {
                        return false;
                    }
                }
                if let Some(status) = query.status {
                    if d.status != status {
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

    async fn find_all(&self, query: &DictTypeQuery) -> AppResult<Vec<DictType>> {
        let dict_types = self.dict_types.read().unwrap();
        Ok(dict_types
            .values()
            .filter(|d| d.audit.deleted == DeletedStatus::Normal)
            .filter(|d| {
                if let Some(status) = query.status {
                    if d.status != status {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect())
    }

    async fn insert(&self, dict_type: &DictType) -> AppResult<()> {
        let mut dict_types = self.dict_types.write().unwrap();
        dict_types.insert(dict_type.id, dict_type.clone());
        Ok(())
    }

    async fn update(&self, dict_type: &DictType) -> AppResult<()> {
        let mut dict_types = self.dict_types.write().unwrap();
        dict_types.insert(dict_type.id, dict_type.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut dict_types = self.dict_types.write().unwrap();
        if let Some(dt) = dict_types.get_mut(&id) {
            dt.audit.deleted = DeletedStatus::Deleted;
            Ok(())
        } else {
            Err(RepositoryError::NotFound)?
        }
    }

    async fn exists_by_type(&self, dict_type: &str) -> AppResult<bool> {
        let dict_types = self.dict_types.read().unwrap();
        Ok(dict_types
            .values()
            .any(|d| d.dict_type == dict_type && d.audit.deleted == DeletedStatus::Normal))
    }
}

#[tx_comp(as_trait = dyn DictDataRepository)]
pub struct MockDictDataRepository {
    #[tx_cst(RwLock::new(HashMap::new()))]
    dict_data: RwLock<HashMap<u64, DictData>>,
}

impl MockDictDataRepository {
    pub fn new() -> Self {
        Self {
            dict_data: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MockDictDataRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DictDataRepository for MockDictDataRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<DictData>> {
        let dict_data = self.dict_data.read().unwrap();
        Ok(dict_data.get(&id).filter(|d| d.audit.deleted == DeletedStatus::Normal).cloned())
    }

    async fn find_by_type(&self, dict_type: &str) -> AppResult<Vec<DictData>> {
        let dict_data = self.dict_data.read().unwrap();
        Ok(dict_data
            .values()
            .filter(|d| d.dict_type == dict_type && d.audit.deleted == DeletedStatus::Normal)
            .cloned()
            .collect())
    }

    async fn find_page(
        &self,
        query: &DictDataQuery,
        page: Page<DictData>,
    ) -> AppResult<Page<DictData>> {
        let dict_data = self.dict_data.read().unwrap();
        let filtered: Vec<DictData> = dict_data
            .values()
            .filter(|d| d.audit.deleted == DeletedStatus::Normal)
            .filter(|d| {
                if let Some(ref dict_type) = query.dict_type {
                    if d.dict_type != *dict_type {
                        return false;
                    }
                }
                if let Some(ref label) = query.label {
                    if !d.label.contains(label.as_str()) {
                        return false;
                    }
                }
                if let Some(status) = query.status {
                    if d.status != status {
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

    async fn insert(&self, data: &DictData) -> AppResult<()> {
        let mut dict_data = self.dict_data.write().unwrap();
        dict_data.insert(data.id, data.clone());
        Ok(())
    }

    async fn update(&self, data: &DictData) -> AppResult<()> {
        let mut dict_data = self.dict_data.write().unwrap();
        dict_data.insert(data.id, data.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut dict_data = self.dict_data.write().unwrap();
        if let Some(dd) = dict_data.get_mut(&id) {
            dd.audit.deleted = DeletedStatus::Deleted;
            Ok(())
        } else {
            Err(RepositoryError::NotFound)?
        }
    }

    async fn find_by_types(&self, dict_types: &[String]) -> AppResult<Vec<DictData>> {
        let dict_data = self.dict_data.read().unwrap();
        Ok(dict_data
            .values()
            .filter(|d| d.audit.deleted == DeletedStatus::Normal)
            .filter(|d| dict_types.contains(&d.dict_type))
            .cloned()
            .collect())
    }
}
