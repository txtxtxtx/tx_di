use std::sync::Arc;

use crate::dictionary::model::aggregate::{DictData, DictType};
use crate::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use crate::dictionary::repository::{DictDataRepository, DictTypeRepository};
use crate::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};
use admin_common::id;

pub struct DictTypeService {
    dict_type_repo: Arc<dyn DictTypeRepository>,
}

impl DictTypeService {
    pub fn new(dict_type_repo: Arc<dyn DictTypeRepository>) -> Self {
        Self { dict_type_repo }
    }

    pub async fn create_dict_type(
        &self,
        name: String,
        dict_type: String,
        creator: Option<String>,
    ) -> Result<DictType, RepositoryError> {
        if self.dict_type_repo.exists_by_type(&dict_type).await? {
            return Err(RepositoryError::Duplicate(format!(
                "Dict type '{}' already exists",
                dict_type
            )));
        }
        let id = id::next_id();
        let dt = DictType::create(id, name, dict_type, creator);
        self.dict_type_repo.insert(&dt).await?;
        Ok(dt)
    }

    pub async fn update_dict_type(
        &self,
        id: u64,
        name: String,
        dict_type: String,
        remark: Option<String>,
        updater: Option<String>,
    ) -> Result<DictType, RepositoryError> {
        let mut dt = self
            .dict_type_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("DictType {} not found", id)))?;
        dt.update_info(name, dict_type, remark, updater);
        self.dict_type_repo.update(&dt).await?;
        Ok(dt)
    }

    pub async fn delete_dict_type(&self, id: u64, updater: Option<String>) -> Result<(), RepositoryError> {
        let mut dt = self
            .dict_type_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("DictType {} not found", id)))?;
        dt.soft_delete(updater);
        self.dict_type_repo.update(&dt).await?;
        Ok(())
    }

    pub async fn get_dict_type_page(
        &self,
        query: &DictTypeQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<DictType>, RepositoryError> {
        self.dict_type_repo.find_page(query, page).await
    }

    pub async fn get_all_dict_types(
        &self,
        query: &DictTypeQuery,
    ) -> Result<Vec<DictType>, RepositoryError> {
        self.dict_type_repo.find_all(query).await
    }
}

pub struct DictDataService {
    dict_data_repo: Arc<dyn DictDataRepository>,
}

impl DictDataService {
    pub fn new(dict_data_repo: Arc<dyn DictDataRepository>) -> Self {
        Self { dict_data_repo }
    }

    pub async fn create_dict_data(
        &self,
        sort: i32,
        label: String,
        value: String,
        dict_type: String,
        creator: Option<String>,
    ) -> Result<DictData, RepositoryError> {
        let id = id::next_id();
        let dd = DictData::create(id, sort, label, value, dict_type, creator);
        self.dict_data_repo.insert(&dd).await?;
        Ok(dd)
    }

    pub async fn update_dict_data(
        &self,
        id: u64,
        sort: i32,
        label: String,
        value: String,
        dict_type: String,
        color_type: Option<String>,
        css_class: Option<String>,
        remark: Option<String>,
        updater: Option<String>,
    ) -> Result<DictData, RepositoryError> {
        let mut dd = self
            .dict_data_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("DictData {} not found", id)))?;
        dd.update_info(sort, label, value, dict_type, color_type, css_class, remark, updater);
        self.dict_data_repo.update(&dd).await?;
        Ok(dd)
    }

    pub async fn delete_dict_data(&self, id: u64, updater: Option<String>) -> Result<(), RepositoryError> {
        let mut dd = self
            .dict_data_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("DictData {} not found", id)))?;
        dd.soft_delete(updater);
        self.dict_data_repo.update(&dd).await?;
        Ok(())
    }

    pub async fn get_dict_data_page(
        &self,
        query: &DictDataQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<DictData>, RepositoryError> {
        self.dict_data_repo.find_page(query, page).await
    }

    pub async fn get_by_dict_type(&self, dict_type: &str) -> Result<Vec<DictData>, RepositoryError> {
        self.dict_data_repo.find_by_type(dict_type).await
    }
}
