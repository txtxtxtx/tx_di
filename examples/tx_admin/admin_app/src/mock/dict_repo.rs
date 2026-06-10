use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::dictionary::model::aggregate::{DictData, DictType};
use admin_domain::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use admin_domain::dictionary::repository::{DictDataRepository, DictTypeRepository};
use admin_domain::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

pub struct MockDictTypeRepository {
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
    async fn find_by_id(&self, id: u64) -> Result<Option<DictType>, RepositoryError> {
        let dict_types = self.dict_types.read().unwrap();
        Ok(dict_types.get(&id).filter(|d| d.audit.deleted == 0).cloned())
    }

    async fn find_by_type(&self, dict_type: &str) -> Result<Option<DictType>, RepositoryError> {
        let dict_types = self.dict_types.read().unwrap();
        Ok(dict_types
            .values()
            .find(|d| d.dict_type == dict_type && d.audit.deleted == 0)
            .cloned())
    }

    async fn find_page(
        &self,
        query: &DictTypeQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<DictType>, RepositoryError> {
        let dict_types = self.dict_types.read().unwrap();
        let filtered: Vec<DictType> = dict_types
            .values()
            .filter(|d| d.audit.deleted == 0)
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
            .take(page.page_size as usize)
            .collect();

        Ok(PageResponse::new(list, total, page.page, page.page_size))
    }

    async fn find_all(&self, query: &DictTypeQuery) -> Result<Vec<DictType>, RepositoryError> {
        let dict_types = self.dict_types.read().unwrap();
        Ok(dict_types
            .values()
            .filter(|d| d.audit.deleted == 0)
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

    async fn insert(&self, dict_type: &DictType) -> Result<(), RepositoryError> {
        let mut dict_types = self.dict_types.write().unwrap();
        dict_types.insert(dict_type.id, dict_type.clone());
        Ok(())
    }

    async fn update(&self, dict_type: &DictType) -> Result<(), RepositoryError> {
        let mut dict_types = self.dict_types.write().unwrap();
        dict_types.insert(dict_type.id, dict_type.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> Result<(), RepositoryError> {
        let mut dict_types = self.dict_types.write().unwrap();
        if let Some(dt) = dict_types.get_mut(&id) {
            dt.audit.deleted = 1;
            Ok(())
        } else {
            Err(RepositoryError::NotFound(format!("DictType {} not found", id)))
        }
    }

    async fn exists_by_type(&self, dict_type: &str) -> Result<bool, RepositoryError> {
        let dict_types = self.dict_types.read().unwrap();
        Ok(dict_types
            .values()
            .any(|d| d.dict_type == dict_type && d.audit.deleted == 0))
    }
}

pub struct MockDictDataRepository {
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
    async fn find_by_id(&self, id: u64) -> Result<Option<DictData>, RepositoryError> {
        let dict_data = self.dict_data.read().unwrap();
        Ok(dict_data.get(&id).filter(|d| d.audit.deleted == 0).cloned())
    }

    async fn find_by_type(&self, dict_type: &str) -> Result<Vec<DictData>, RepositoryError> {
        let dict_data = self.dict_data.read().unwrap();
        Ok(dict_data
            .values()
            .filter(|d| d.dict_type == dict_type && d.audit.deleted == 0)
            .cloned()
            .collect())
    }

    async fn find_page(
        &self,
        query: &DictDataQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<DictData>, RepositoryError> {
        let dict_data = self.dict_data.read().unwrap();
        let filtered: Vec<DictData> = dict_data
            .values()
            .filter(|d| d.audit.deleted == 0)
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
            .take(page.page_size as usize)
            .collect();

        Ok(PageResponse::new(list, total, page.page, page.page_size))
    }

    async fn insert(&self, data: &DictData) -> Result<(), RepositoryError> {
        let mut dict_data = self.dict_data.write().unwrap();
        dict_data.insert(data.id, data.clone());
        Ok(())
    }

    async fn update(&self, data: &DictData) -> Result<(), RepositoryError> {
        let mut dict_data = self.dict_data.write().unwrap();
        dict_data.insert(data.id, data.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> Result<(), RepositoryError> {
        let mut dict_data = self.dict_data.write().unwrap();
        if let Some(dd) = dict_data.get_mut(&id) {
            dd.audit.deleted = 1;
            Ok(())
        } else {
            Err(RepositoryError::NotFound(format!("DictData {} not found", id)))
        }
    }
}
