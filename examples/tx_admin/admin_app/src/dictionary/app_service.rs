use std::sync::Arc;

use crate::dictionary::dto::*;
use admin_domain::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use admin_domain::dictionary::service::{DictDataService, DictTypeService};
use tx_error::AppResult;
use tx_common::page::Page;

pub struct DictTypeAppService {
    dict_type_service: Arc<DictTypeService>,
}

impl DictTypeAppService {
    pub fn new(dict_type_service: Arc<DictTypeService>) -> Self {
        Self { dict_type_service }
    }

    pub async fn create_dict_type(
        &self,
        cmd: CreateDictTypeCommand,
        creator: Option<String>,
    ) -> AppResult<DictTypeResponse> {
        let dt = self
            .dict_type_service
            .create_dict_type(cmd.name, cmd.dict_type, creator)
            .await?;
        Ok(DictTypeResponse::from(dt))
    }

    pub async fn update_dict_type(
        &self,
        cmd: UpdateDictTypeCommand,
        updater: Option<String>,
    ) -> AppResult<DictTypeResponse> {
        let dt = self
            .dict_type_service
            .update_dict_type(cmd.id, cmd.name, cmd.dict_type, cmd.remark, updater)
            .await?;
        Ok(DictTypeResponse::from(dt))
    }

    pub async fn delete_dict_type(&self, id: u64, updater: Option<String>) -> AppResult<()> {
        self.dict_type_service.delete_dict_type(id, updater).await
    }

    pub async fn get_dict_type_page(
        &self,
        request: DictTypeQueryRequest,
    ) -> AppResult<Page<DictTypeResponse>> {
        let query = DictTypeQuery {
            name: request.name,
            dict_type: request.dict_type,
            status: request.status,
        };
        let page = Page::<()>::request(request.page, request.page_size);
        let result = self.dict_type_service.get_dict_type_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(DictTypeResponse::from).collect(),
            result.page,
            result.page_size,
            result.total,
        ))
    }

    pub async fn get_all_dict_types(&self) -> AppResult<Vec<DictTypeResponse>> {
        let dts = self.dict_type_service.get_all_dict_types(&DictTypeQuery::default()).await?;
        Ok(dts.into_iter().map(DictTypeResponse::from).collect())
    }
}

pub struct DictDataAppService {
    dict_data_service: Arc<DictDataService>,
}

impl DictDataAppService {
    pub fn new(dict_data_service: Arc<DictDataService>) -> Self {
        Self { dict_data_service }
    }

    pub async fn create_dict_data(
        &self,
        cmd: CreateDictDataCommand,
        creator: Option<String>,
    ) -> AppResult<DictDataResponse> {
        let dd = self
            .dict_data_service
            .create_dict_data(cmd.sort, cmd.label, cmd.value, cmd.dict_type, creator)
            .await?;
        Ok(DictDataResponse::from(dd))
    }

    pub async fn update_dict_data(
        &self,
        cmd: UpdateDictDataCommand,
        updater: Option<String>,
    ) -> AppResult<DictDataResponse> {
        let dd = self
            .dict_data_service
            .update_dict_data(
                cmd.id,
                cmd.sort,
                cmd.label,
                cmd.value,
                cmd.dict_type,
                cmd.color_type,
                cmd.css_class,
                cmd.remark,
                updater,
            )
            .await?;
        Ok(DictDataResponse::from(dd))
    }

    pub async fn delete_dict_data(&self, id: u64, updater: Option<String>) -> AppResult<()> {
        self.dict_data_service.delete_dict_data(id, updater).await
    }

    pub async fn get_dict_data_page(
        &self,
        request: DictDataQueryRequest,
    ) -> AppResult<Page<DictDataResponse>> {
        let query = DictDataQuery {
            dict_type: request.dict_type,
            label: request.label,
            status: request.status,
        };
        let page = Page::<()>::request(request.page, request.page_size);
        let result = self.dict_data_service.get_dict_data_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(DictDataResponse::from).collect(),
            result.page,
            result.page_size,
            result.total,
        ))
    }

    pub async fn get_by_dict_type(&self, dict_type: &str) -> AppResult<Vec<DictDataResponse>> {
        let data = self.dict_data_service.get_by_dict_type(dict_type).await?;
        Ok(data.into_iter().map(DictDataResponse::from).collect())
    }
}
