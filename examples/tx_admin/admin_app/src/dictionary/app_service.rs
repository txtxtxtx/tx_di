use std::collections::HashMap;
use std::sync::Arc;

use crate::dictionary::dto::{dict_type_to_response, dict_data_to_response};
use admin_domain::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use admin_domain::dictionary::service::{DictDataService, DictTypeService};
use admin_proto::{
    CreateDictTypeRequest, UpdateDictTypeRequest, ListDictTypesRequest, DictTypeResponse,
    CreateDictDataRequest, UpdateDictDataRequest, ListDictDataRequest, DictDataResponse,
};
use tx_di_core::{Component, DepsTuple};
use tx_error::AppResult;
use tx_common::page::Page;

#[derive(Component)]
pub struct DictTypeAppService {
    dict_type_service: Arc<DictTypeService>,
}

impl DictTypeAppService {
    /// 创建字典类型应用服务实例
    pub fn new(dict_type_service: Arc<DictTypeService>) -> Self {
        Self { dict_type_service }
    }

    /// 创建新字典类型
    pub async fn create_dict_type(
        &self,
        req: CreateDictTypeRequest,
        creator: Option<String>,
    ) -> AppResult<DictTypeResponse> {
        let dt = self
            .dict_type_service
            .create_dict_type(req.name, req.dict_type, creator)
            .await?;
        Ok(dict_type_to_response(dt))
    }

    /// 更新字典类型信息
    pub async fn update_dict_type(
        &self,
        req: UpdateDictTypeRequest,
        updater: Option<String>,
    ) -> AppResult<DictTypeResponse> {
        let dt = self
            .dict_type_service
            .update_dict_type(req.id, req.name, req.dict_type, req.remark, updater)
            .await?;
        Ok(dict_type_to_response(dt))
    }

    /// 删除字典类型
    pub async fn delete_dict_type(&self, id: u64, updater: Option<String>) -> AppResult<()> {
        self.dict_type_service.delete_dict_type(id, updater).await
    }

    /// 分页查询字典类型列表
    pub async fn get_dict_type_page(
        &self,
        req: ListDictTypesRequest,
    ) -> AppResult<Page<DictTypeResponse>> {
        let query = DictTypeQuery {
            name: req.name,
            dict_type: req.dict_type,
            status: req.status,
        };
        let page = Page::request(req.page, req.page_size);
        let result = self.dict_type_service.get_dict_type_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(dict_type_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 获取所有字典类型列表
    pub async fn get_all_dict_types(&self) -> AppResult<Vec<DictTypeResponse>> {
        let dts = self.dict_type_service.get_all_dict_types(&DictTypeQuery::default()).await?;
        Ok(dts.into_iter().map(dict_type_to_response).collect())
    }
}

#[derive(Component)]
pub struct DictDataAppService {
    dict_data_service: Arc<DictDataService>,
}

impl DictDataAppService {
    /// 创建字典数据应用服务实例
    pub fn new(dict_data_service: Arc<DictDataService>) -> Self {
        Self { dict_data_service }
    }

    /// 创建新字典数据
    pub async fn create_dict_data(
        &self,
        req: CreateDictDataRequest,
        creator: Option<String>,
    ) -> AppResult<DictDataResponse> {
        let dd = self
            .dict_data_service
            .create_dict_data(req.sort, req.label, req.value, req.dict_type, creator)
            .await?;
        Ok(dict_data_to_response(dd))
    }

    /// 更新字典数据信息
    pub async fn update_dict_data(
        &self,
        req: UpdateDictDataRequest,
        updater: Option<String>,
    ) -> AppResult<DictDataResponse> {
        let dd = self
            .dict_data_service
            .update_dict_data(
                req.id,
                req.sort,
                req.label,
                req.value,
                req.dict_type,
                req.color_type,
                req.css_class,
                req.remark,
                updater,
            )
            .await?;
        Ok(dict_data_to_response(dd))
    }

    /// 删除字典数据
    pub async fn delete_dict_data(&self, id: u64, updater: Option<String>) -> AppResult<()> {
        self.dict_data_service.delete_dict_data(id, updater).await
    }

    /// 分页查询字典数据列表
    pub async fn get_dict_data_page(
        &self,
        req: ListDictDataRequest,
    ) -> AppResult<Page<DictDataResponse>> {
        let query = DictDataQuery {
            dict_type: req.dict_type,
            label: req.label,
            status: req.status,
        };
        let page = Page::request(req.page, req.page_size);
        let result = self.dict_data_service.get_dict_data_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(dict_data_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 根据字典类型编码获取字典数据列表
    pub async fn get_by_dict_type(&self, dict_type: &str) -> AppResult<Vec<DictDataResponse>> {
        let data = self.dict_data_service.get_by_dict_type(dict_type).await?;
        Ok(data.into_iter().map(dict_data_to_response).collect())
    }

    /// 批量根据字典类型编码获取字典数据
    pub async fn get_by_dict_types(&self, dict_types: Vec<String>) -> AppResult<HashMap<String, Vec<DictDataResponse>>> {
        let map = self.dict_data_service.get_by_dict_types(&dict_types).await?;
        Ok(map.into_iter()
            .map(|(k, v)| (k, v.into_iter().map(dict_data_to_response).collect()))
            .collect())
    }
}
