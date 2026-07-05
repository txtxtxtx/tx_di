use std::collections::HashMap;
use std::sync::Arc;

use crate::dictionary::model::aggregate::{DictData, DictType};
use crate::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use crate::dictionary::repository::{DictDataRepository, DictTypeRepository};
use crate::shared::repository::RepositoryError;
use tx_common::page::Page;
use tx_di_core::{Component, DepsTuple};
use tx_error::AppResult;
use tx_common::id;

#[derive(Component)]
pub struct DictTypeService {
    dict_type_repo: Arc<dyn DictTypeRepository>,
}

impl DictTypeService {
    /// 构造函数，创建字典类型服务实例
    ///
    /// # 参数
    /// * `dict_type_repo` - 字典类型仓储的 Arc 智能指针，用于数据持久化操作
    pub fn new(dict_type_repo: Arc<dyn DictTypeRepository>) -> Self {
        Self { dict_type_repo }
    }

    /// 创建新的字典类型
    ///
    /// # 参数
    /// * `name` - 字典类型名称，用于展示和识别
    /// * `dict_type` - 字典类型编码，系统中唯一标识一个字典类型
    /// * `creator` - 创建者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 检查 dict_type 编码是否已存在，若存在则返回重复错误
    /// 2. 生成唯一 ID
    /// 3. 调用 DictType 聚合根的 create 方法构造字典类型实体
    /// 4. 将字典类型持久化到仓储
    ///
    /// # 返回
    /// 成功返回新创建的 DictType 聚合根
    ///
    /// # 错误
    /// - `DuplicateDictType` - 当 dict_type 编码已存在时
    /// - 数据库操作错误 - 仓储插入失败时
    pub async fn create_dict_type(
        &self,
        name: String,
        dict_type: String,
        creator: Option<String>,
    ) -> AppResult<DictType> {
        if self.dict_type_repo.exists_by_type(&dict_type).await? {
            return Err(RepositoryError::DuplicateDictType)?;
        }
        let id = id::next_id();
        let dt = DictType::create(id, name, dict_type, creator);
        self.dict_type_repo.insert(&dt).await?;
        Ok(dt)
    }

    /// 更新已有字典类型的信息
    ///
    /// # 参数
    /// * `id` - 要更新的字典类型 ID
    /// * `name` - 新的字典类型名称
    /// * `dict_type` - 新的字典类型编码
    /// * `remark` - 备注信息，可选
    /// * `updater` - 更新者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据 id 查找字典类型，若不存在则返回未找到错误
    /// 2. 调用聚合根的 update_info 方法更新字典类型信息
    /// 3. 将更新后的字典类型持久化到仓储
    ///
    /// # 返回
    /// 成功返回更新后的 DictType 聚合根
    ///
    /// # 错误
    /// - `NotFoundDict` - 当指定 id 的字典类型不存在时
    /// - 数据库操作错误 - 仓储查询或更新失败时
    pub async fn update_dict_type(
        &self,
        id: u64,
        name: String,
        dict_type: String,
        remark: Option<String>,
        updater: Option<String>,
    ) -> AppResult<DictType> {
        let mut dt = self
            .dict_type_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundDict)?;
        dt.update_info(name, dict_type, remark, updater);
        self.dict_type_repo.update(&dt).await?;
        Ok(dt)
    }

    /// 软删除字典类型（逻辑删除）
    ///
    /// # 参数
    /// * `id` - 要删除的字典类型 ID
    /// * `updater` - 操作者标识，可选，用于记录删除操作人
    ///
    /// # 执行逻辑
    /// 1. 根据 id 查找字典类型，若不存在则返回未找到错误
    /// 2. 调用聚合根的 soft_delete 方法标记为已删除
    /// 3. 将状态变更持久化到仓储
    ///
    /// # 返回
    /// 成功返回 ()
    ///
    /// # 错误
    /// - `NotFoundDict` - 当指定 id 的字典类型不存在时
    /// - 数据库操作错误 - 仓储查询或更新失败时
    pub async fn delete_dict_type(&self, id: u64, updater: Option<String>) -> AppResult<()> {
        let mut dt = self
            .dict_type_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundDict)?;
        dt.soft_delete(updater);
        self.dict_type_repo.update(&dt).await?;
        Ok(())
    }

    /// 分页查询字典类型列表
    ///
    /// # 参数
    /// * `query` - 查询条件对象，包含名称、编码等筛选字段
    /// * `page` - 分页参数，包含页码、每页条数等信息
    ///
    /// # 执行逻辑
    /// 1. 将查询条件和分页参数传递给仓储层
    /// 2. 仓储层执行分页查询并返回结果
    ///
    /// # 返回
    /// 成功返回包含字典类型列表的分页对象 Page<DictType>
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_dict_type_page(
        &self,
        query: &DictTypeQuery,
        page: Page<DictType>,
    ) -> AppResult<Page<DictType>> {
        self.dict_type_repo.find_page(query, page).await
    }

    /// 获取全部字典类型列表（不分页）
    ///
    /// # 参数
    /// * `query` - 查询条件对象，用于筛选字典类型
    ///
    /// # 执行逻辑
    /// 1. 将查询条件传递给仓储层
    /// 2. 仓储层查询所有符合条件的字典类型并返回
    ///
    /// # 返回
    /// 成功返回符合条件的 DictType 列表
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_all_dict_types(
        &self,
        query: &DictTypeQuery,
    ) -> AppResult<Vec<DictType>> {
        self.dict_type_repo.find_all(query).await
    }
}

#[derive(Component)]
pub struct DictDataService {
    dict_data_repo: Arc<dyn DictDataRepository>,
}

impl DictDataService {
    /// 构造函数，创建字典数据服务实例
    ///
    /// # 参数
    /// * `dict_data_repo` - 字典数据仓储的 Arc 智能指针，用于数据持久化操作
    pub fn new(dict_data_repo: Arc<dyn DictDataRepository>) -> Self {
        Self { dict_data_repo }
    }

    /// 创建新的字典数据项
    ///
    /// # 参数
    /// * `sort` - 排序号，用于控制字典数据的显示顺序
    /// * `label` - 字典数据标签，用于前端展示
    /// * `value` - 字典数据值，用于业务逻辑匹配
    /// * `dict_type` - 所属字典类型编码，关联到对应的字典类型
    /// * `creator` - 创建者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 生成唯一 ID
    /// 2. 调用 DictData 聚合根的 create 方法构造字典数据实体
    /// 3. 将字典数据持久化到仓储
    ///
    /// # 返回
    /// 成功返回新创建的 DictData 聚合根
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储插入失败时
    pub async fn create_dict_data(
        &self,
        sort: i32,
        label: String,
        value: String,
        dict_type: String,
        creator: Option<String>,
    ) -> AppResult<DictData> {
        let id = id::next_id();
        let dd = DictData::create(id, sort, label, value, dict_type, creator);
        self.dict_data_repo.insert(&dd).await?;
        Ok(dd)
    }

    /// 更新已有字典数据项的信息
    ///
    /// # 参数
    /// * `id` - 要更新的字典数据 ID
    /// * `sort` - 新的排序号
    /// * `label` - 新的标签
    /// * `value` - 新的数据值
    /// * `dict_type` - 新的所属字典类型编码
    /// * `color_type` - 颜色类型，可选，用于前端样式展示
    /// * `css_class` - CSS 类名，可选，用于前端样式定制
    /// * `remark` - 备注信息，可选
    /// * `updater` - 更新者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据 id 查找字典数据，若不存在则返回未找到错误
    /// 2. 调用聚合根的 update_info 方法更新字典数据信息
    /// 3. 将更新后的字典数据持久化到仓储
    ///
    /// # 返回
    /// 成功返回更新后的 DictData 聚合根
    ///
    /// # 错误
    /// - `NotFoundDict` - 当指定 id 的字典数据不存在时
    /// - 数据库操作错误 - 仓储查询或更新失败时
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
    ) -> AppResult<DictData> {
        let mut dd = self
            .dict_data_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundDict)?;
        dd.update_info(sort, label, value, dict_type, color_type, css_class, remark, updater);
        self.dict_data_repo.update(&dd).await?;
        Ok(dd)
    }

    /// 软删除字典数据项（逻辑删除）
    ///
    /// # 参数
    /// * `id` - 要删除的字典数据 ID
    /// * `updater` - 操作者标识，可选，用于记录删除操作人
    ///
    /// # 执行逻辑
    /// 1. 根据 id 查找字典数据，若不存在则返回未找到错误
    /// 2. 调用聚合根的 soft_delete 方法标记为已删除
    /// 3. 将状态变更持久化到仓储
    ///
    /// # 返回
    /// 成功返回 ()
    ///
    /// # 错误
    /// - `NotFoundDict` - 当指定 id 的字典数据不存在时
    /// - 数据库操作错误 - 仓储查询或更新失败时
    pub async fn delete_dict_data(&self, id: u64, updater: Option<String>) -> AppResult<()> {
        let mut dd = self
            .dict_data_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundDict)?;
        dd.soft_delete(updater);
        self.dict_data_repo.update(&dd).await?;
        Ok(())
    }

    /// 分页查询字典数据列表
    ///
    /// # 参数
    /// * `query` - 查询条件对象，包含字典类型、标签、值等筛选字段
    /// * `page` - 分页参数，包含页码、每页条数等信息
    ///
    /// # 执行逻辑
    /// 1. 将查询条件和分页参数传递给仓储层
    /// 2. 仓储层执行分页查询并返回结果
    ///
    /// # 返回
    /// 成功返回包含字典数据列表的分页对象 Page<DictData>
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_dict_data_page(
        &self,
        query: &DictDataQuery,
        page: Page<DictData>,
    ) -> AppResult<Page<DictData>> {
        self.dict_data_repo.find_page(query, page).await
    }

    /// 根据字典类型编码获取该类型下的所有字典数据
    ///
    /// # 参数
    /// * `dict_type` - 字典类型编码
    ///
    /// # 执行逻辑
    /// 1. 根据 dict_type 编码从仓储中查找该类型下的所有字典数据
    /// 2. 返回查询结果列表
    ///
    /// # 返回
    /// 成功返回该字典类型下的 DictData 列表
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_by_dict_type(&self, dict_type: &str) -> AppResult<Vec<DictData>> {
        self.dict_data_repo.find_by_type(dict_type).await
    }

    /// 批量根据字典类型编码获取字典数据，按类型分组返回
    ///
    /// # 参数
    /// * `dict_types` - 字典类型编码列表
    ///
    /// # 执行逻辑
    /// 1. 根据 dict_types 列表从仓储中批量查找所有字典数据
    /// 2. 将查询结果按 dict_type 字段进行分组
    /// 3. 构建 dict_type -> Vec<DictData> 的映射关系
    ///
    /// # 返回
    /// 成功返回 dict_type -> Vec<DictData> 的 HashMap 映射，未找到的类型对应空 Vec
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_by_dict_types(&self, dict_types: &[String]) -> AppResult<HashMap<String, Vec<DictData>>> {
        let all_data = self.dict_data_repo.find_by_types(dict_types).await?;
        let mut map: HashMap<String, Vec<DictData>> = HashMap::new();
        for data in all_data {
            map.entry(data.dict_type.clone()).or_default().push(data);
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests;
