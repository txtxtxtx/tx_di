use std::collections::HashMap;
use std::sync::Arc;

use crate::dictionary::dto::*;
use admin_domain::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use admin_domain::dictionary::service::{DictDataService, DictTypeService};
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

#[tx_comp]
pub struct DictTypeAppService {
    dict_type_service: Arc<DictTypeService>,
}

impl DictTypeAppService {
    /// 创建字典类型应用服务实例
    ///
    /// # 参数
    /// * `dict_type_service` - 字典类型领域服务，用于执行字典类型相关的业务逻辑
    pub fn new(dict_type_service: Arc<DictTypeService>) -> Self {
        Self { dict_type_service }
    }

    /// 创建新字典类型
    ///
    /// # 参数
    /// * `cmd` - 创建字典类型命令，包含字典名称、字典类型编码
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给字典类型领域服务执行创建操作，逻辑详见 `DictTypeService::create_dict_type`
    ///
    /// # 返回
    /// 成功返回 `DictTypeResponse`，包含字典类型完整信息
    ///
    /// # 错误
    /// - `DuplicateDictType` - 字典类型编码已存在
    /// - 数据库写入异常
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

    /// 更新字典类型信息
    ///
    /// # 参数
    /// * `cmd` - 更新字典类型命令，包含字典类型ID、名称、字典类型编码、备注
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给字典类型领域服务执行更新操作，逻辑详见 `DictTypeService::update_dict_type`
    ///
    /// # 返回
    /// 成功返回更新后的 `DictTypeResponse`
    ///
    /// # 错误
    /// - `NotFoundDictType` - 字典类型ID对应的记录不存在
    /// - `DuplicateDictType` - 字典类型编码与其他记录冲突
    /// - 数据库更新异常
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

    /// 删除字典类型
    ///
    /// # 参数
    /// * `id` - 要删除的字典类型ID
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给字典类型领域服务执行删除操作，逻辑详见 `DictTypeService::delete_dict_type`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundDictType` - 字典类型ID对应的记录不存在
    /// - 数据库删除异常
    pub async fn delete_dict_type(&self, id: u64, updater: Option<String>) -> AppResult<()> {
        self.dict_type_service.delete_dict_type(id, updater).await
    }

    /// 分页查询字典类型列表
    ///
    /// # 参数
    /// * `request` - 分页查询请求，包含字典名称、字典类型编码、状态等筛选条件，以及页码和每页大小
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `DictTypeQuery`
    /// 2. 构建分页参数 `Page`
    /// 3. 委托给字典类型领域服务执行分页查询
    /// 4. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Page<DictTypeResponse>`，包含字典类型列表、页码、每页大小和总记录数
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_dict_type_page(
        &self,
        request: DictTypeQueryRequest,
    ) -> AppResult<Page<DictTypeResponse>> {
        let query = DictTypeQuery {
            name: request.name,
            dict_type: request.dict_type,
            status: request.status,
        };
        let page = Page::request(request.page, request.size);
        let result = self.dict_type_service.get_dict_type_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(DictTypeResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 获取所有字典类型列表
    ///
    /// # 执行逻辑
    /// 使用默认查询条件（无筛选）调用字典类型领域服务获取全部字典类型
    ///
    /// # 返回
    /// 成功返回 `Vec<DictTypeResponse>`，包含所有字典类型列表
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_all_dict_types(&self) -> AppResult<Vec<DictTypeResponse>> {
        let dts = self.dict_type_service.get_all_dict_types(&DictTypeQuery::default()).await?;
        Ok(dts.into_iter().map(DictTypeResponse::from).collect())
    }
}

#[tx_comp]
pub struct DictDataAppService {
    dict_data_service: Arc<DictDataService>,
}

impl DictDataAppService {
    /// 创建字典数据应用服务实例
    ///
    /// # 参数
    /// * `dict_data_service` - 字典数据领域服务，用于执行字典数据相关的业务逻辑
    pub fn new(dict_data_service: Arc<DictDataService>) -> Self {
        Self { dict_data_service }
    }

    /// 创建新字典数据
    ///
    /// # 参数
    /// * `cmd` - 创建字典数据命令，包含排序号、标签、值、所属字典类型
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给字典数据领域服务执行创建操作，逻辑详见 `DictDataService::create_dict_data`
    ///
    /// # 返回
    /// 成功返回 `DictDataResponse`，包含字典数据完整信息
    ///
    /// # 错误
    /// - `NotFoundDictType` - 所属字典类型不存在
    /// - 数据库写入异常
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

    /// 更新字典数据信息
    ///
    /// # 参数
    /// * `cmd` - 更新字典数据命令，包含字典数据ID、排序号、标签、值、所属字典类型、颜色类型、CSS类名、备注
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给字典数据领域服务执行更新操作，逻辑详见 `DictDataService::update_dict_data`
    ///
    /// # 返回
    /// 成功返回更新后的 `DictDataResponse`
    ///
    /// # 错误
    /// - `NotFoundDictData` - 字典数据ID对应的记录不存在
    /// - 数据库更新异常
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

    /// 删除字典数据
    ///
    /// # 参数
    /// * `id` - 要删除的字典数据ID
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给字典数据领域服务执行删除操作，逻辑详见 `DictDataService::delete_dict_data`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundDictData` - 字典数据ID对应的记录不存在
    /// - 数据库删除异常
    pub async fn delete_dict_data(&self, id: u64, updater: Option<String>) -> AppResult<()> {
        self.dict_data_service.delete_dict_data(id, updater).await
    }

    /// 分页查询字典数据列表
    ///
    /// # 参数
    /// * `request` - 分页查询请求，包含所属字典类型、标签、状态等筛选条件，以及页码和每页大小
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `DictDataQuery`
    /// 2. 构建分页参数 `Page`
    /// 3. 委托给字典数据领域服务执行分页查询
    /// 4. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Page<DictDataResponse>`，包含字典数据列表、页码、每页大小和总记录数
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_dict_data_page(
        &self,
        request: DictDataQueryRequest,
    ) -> AppResult<Page<DictDataResponse>> {
        let query = DictDataQuery {
            dict_type: request.dict_type,
            label: request.label,
            status: request.status,
        };
        let page = Page::request(request.page, request.size);
        let result = self.dict_data_service.get_dict_data_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(DictDataResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 根据字典类型编码获取字典数据列表
    ///
    /// # 参数
    /// * `dict_type` - 字典类型编码
    ///
    /// # 执行逻辑
    /// 委托给字典数据领域服务根据字典类型查询数据列表，逻辑详见 `DictDataService::get_by_dict_type`
    ///
    /// # 返回
    /// 成功返回 `Vec<DictDataResponse>`，包含该字典类型下的所有数据项
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_by_dict_type(&self, dict_type: &str) -> AppResult<Vec<DictDataResponse>> {
        let data = self.dict_data_service.get_by_dict_type(dict_type).await?;
        Ok(data.into_iter().map(DictDataResponse::from).collect())
    }

    /// 批量根据字典类型编码获取字典数据
    ///
    /// # 参数
    /// * `dict_types` - 字典类型编码列表
    ///
    /// # 执行逻辑
    /// 委托给字典数据领域服务批量查询字典数据，返回按字典类型分组的数据映射，逻辑详见 `DictDataService::get_by_dict_types`
    ///
    /// # 返回
    /// 成功返回 `HashMap<String, Vec<DictDataResponse>>`，键为字典类型编码，值为对应的数据项列表
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_by_dict_types(&self, dict_types: Vec<String>) -> AppResult<HashMap<String, Vec<DictDataResponse>>> {
        let map = self.dict_data_service.get_by_dict_types(&dict_types).await?;
        Ok(map.into_iter()
            .map(|(k, v)| (k, v.into_iter().map(DictDataResponse::from).collect()))
            .collect())
    }
}
