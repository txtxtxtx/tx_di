use async_trait::async_trait;
use std::any::Any;

use crate::system::dictionary::model::aggregate::{DictData, DictType};
use crate::system::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use tx_common::page::Page;
use tx_error::{AppResult, CodeMsg};

/// Dictionary 仓储错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("REPOSITORY")]
pub enum DictionaryRepositoryError {
    #[err(10007, "数据库异常")]
    DatabaseDict,
    #[err(10107, "记录不存在")]
    NotFoundDict,
    #[err(10205, "字典类型已存在")]
    DuplicateDictType,
}

#[async_trait]
pub trait DictTypeRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<DictType>>;
    async fn find_by_type(&self, dict_type: &str) -> AppResult<Option<DictType>>;
    async fn find_page(
        &self,
        query: &DictTypeQuery,
        page: Page<DictType>,
    ) -> AppResult<Page<DictType>>;
    async fn find_all(&self, query: &DictTypeQuery) -> AppResult<Vec<DictType>>;
    async fn insert(&self, dict_type: &DictType) -> AppResult<()>;
    async fn update(&self, dict_type: &DictType) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
    async fn exists_by_type(&self, dict_type: &str) -> AppResult<bool>;
}

#[async_trait]
pub trait DictDataRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<DictData>>;
    async fn find_by_type(&self, dict_type: &str) -> AppResult<Vec<DictData>>;
    async fn find_by_types(&self, dict_types: &[String]) -> AppResult<Vec<DictData>>;
    async fn find_page(
        &self,
        query: &DictDataQuery,
        page: Page<DictData>,
    ) -> AppResult<Page<DictData>>;
    async fn insert(&self, data: &DictData) -> AppResult<()>;
    async fn update(&self, data: &DictData) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
}
