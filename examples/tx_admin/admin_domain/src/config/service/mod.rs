use std::collections::HashMap;
use std::sync::Arc;

use crate::config::model::aggregate::Config;
use crate::config::model::value_object::ConfigQuery;
use crate::config::repository::ConfigRepository;
use crate::shared::repository::RepositoryError;
use tx_common::page::Page;
use tx_di_core::{Component, DepsTuple};
use tx_error::AppResult;
use tx_common::id;

#[derive(Component)]
pub struct ConfigService {
    config_repo: Arc<dyn ConfigRepository>,
}

impl ConfigService {
    /// 构造函数，创建配置服务实例
    ///
    /// # 参数
    /// * `config_repo` - 配置仓储的 Arc 智能指针，用于数据持久化操作
    pub fn new(config_repo: Arc<dyn ConfigRepository>) -> Self {
        Self { config_repo }
    }

    /// 创建新的系统配置项
    ///
    /// # 参数
    /// * `category` - 配置分类，用于对配置进行逻辑分组
    /// * `config_type` - 配置类型标识（整数），区分不同种类的配置
    /// * `name` - 配置名称，用于展示和识别
    /// * `config_key` - 配置键名，系统中唯一标识一个配置项
    /// * `value` - 配置值
    /// * `creator` - 创建者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 检查 config_key 是否已存在，若存在则返回重复键错误
    /// 2. 生成唯一配置 ID
    /// 3. 调用 Config 聚合根的 create 方法构造配置实体
    /// 4. 将配置持久化到仓储
    ///
    /// # 返回
    /// 成功返回新创建的 Config 聚合根
    ///
    /// # 错误
    /// - `DuplicateConfigKey` - 当 config_key 已被其他配置占用时
    /// - 数据库操作错误 - 仓储插入失败时
    pub async fn create_config(
        &self,
        category: String,
        config_type: i32,
        name: String,
        config_key: String,
        value: String,
        creator: Option<String>,
    ) -> AppResult<Config> {
        if self.config_repo.exists_by_key(&config_key).await? {
            return Err(RepositoryError::DuplicateConfigKey)?;
        }

        let config_id = id::next_id();
        let config = Config::create(config_id, category, config_type, name, config_key, value, creator);
        self.config_repo.insert(&config).await?;
        Ok(config)
    }

    /// 更新已有配置项的信息
    ///
    /// # 参数
    /// * `config_id` - 要更新的配置 ID
    /// * `category` - 新的配置分类
    /// * `config_type` - 新的配置类型标识
    /// * `name` - 新的配置名称
    /// * `config_key` - 新的配置键名
    /// * `value` - 新的配置值
    /// * `visible` - 可见性标识
    /// * `remark` - 备注信息，可选
    /// * `updater` - 更新者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据 config_id 查找配置，若不存在则返回未找到错误
    /// 2. 检查新的 config_key 是否被其他配置占用（排除自身），若占用则返回重复键错误
    /// 3. 调用聚合根的 update_info 方法更新配置信息
    /// 4. 将更新后的配置持久化到仓储
    ///
    /// # 返回
    /// 成功返回更新后的 Config 聚合根
    ///
    /// # 错误
    /// - `NotFoundConfig` - 当指定 config_id 的配置不存在时
    /// - `DuplicateConfigKey` - 当新的 config_key 已被其他配置占用时
    /// - 数据库操作错误 - 仓储查询或更新失败时
    pub async fn update_config(
        &self,
        config_id: u64,
        category: String,
        config_type: i32,
        name: String,
        config_key: String,
        value: String,
        visible: i32,
        remark: Option<String>,
        updater: Option<String>,
    ) -> AppResult<Config> {
        let mut config = self
            .config_repo
            .find_by_id(config_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundConfig)?;

        // 检查 config_key 是否被其他配置占用
        if let Some(existing) = self.config_repo.find_by_key(&config_key).await? {
            if existing.id != config_id {
                return Err(RepositoryError::DuplicateConfigKey)?;
            }
        }

        config.update_info(category, config_type, name, config_key, value, visible, remark, updater);
        self.config_repo.update(&config).await?;
        Ok(config)
    }

    /// 软删除配置项（逻辑删除）
    ///
    /// # 参数
    /// * `config_id` - 要删除的配置 ID
    /// * `updater` - 操作者标识，可选，用于记录删除操作人
    ///
    /// # 执行逻辑
    /// 1. 根据 config_id 查找配置，若不存在则返回未找到错误
    /// 2. 调用聚合根的 soft_delete 方法标记为已删除
    /// 3. 将状态变更持久化到仓储
    ///
    /// # 返回
    /// 成功返回 ()
    ///
    /// # 错误
    /// - `NotFoundConfig` - 当指定 config_id 的配置不存在时
    /// - 数据库操作错误 - 仓储查询或更新失败时
    pub async fn delete_config(&self, config_id: u64, updater: Option<String>) -> AppResult<()> {
        let mut config = self
            .config_repo
            .find_by_id(config_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundConfig)?;

        config.soft_delete(updater);
        self.config_repo.update(&config).await?;
        Ok(())
    }

    /// 分页查询配置列表
    ///
    /// # 参数
    /// * `query` - 查询条件对象，包含分类、类型、名称等筛选字段
    /// * `page` - 分页参数，包含页码、每页条数等信息
    ///
    /// # 执行逻辑
    /// 1. 将查询条件和分页参数传递给仓储层
    /// 2. 仓储层执行分页查询并返回结果
    ///
    /// # 返回
    /// 成功返回包含配置列表的分页对象 Page<Config>
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_config_page(
        &self,
        query: &ConfigQuery,
        page: Page<Config>,
    ) -> AppResult<Page<Config>> {
        self.config_repo.find_page(query, page).await
    }

    /// 根据 ID 获取单个配置详情
    ///
    /// # 参数
    /// * `config_id` - 配置 ID
    ///
    /// # 执行逻辑
    /// 1. 根据 config_id 从仓储中查找配置
    /// 2. 若找到则返回，若未找到则返回未找到错误
    ///
    /// # 返回
    /// 成功返回对应的 Config 聚合根
    ///
    /// # 错误
    /// - `NotFoundConfig` - 当指定 config_id 的配置不存在时
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_config(&self, config_id: u64) -> AppResult<Config> {
        Ok(self.config_repo
            .find_by_id(config_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundConfig)?)
    }

    /// 根据配置键名获取单个配置
    ///
    /// # 参数
    /// * `key` - 配置键名
    ///
    /// # 执行逻辑
    /// 1. 根据 key 从仓储中查找配置
    /// 2. 若找到则返回，若未找到则返回未找到错误
    ///
    /// # 返回
    /// 成功返回对应的 Config 聚合根
    ///
    /// # 错误
    /// - `NotFoundConfig` - 当指定 key 的配置不存在时
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_by_key(&self, key: &str) -> AppResult<Config> {
        Ok(self.config_repo
            .find_by_key(key)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundConfig)?)
    }

    /// 批量根据配置键名获取配置值映射
    ///
    /// # 参数
    /// * `keys` - 配置键名列表
    ///
    /// # 执行逻辑
    /// 1. 根据 keys 列表从仓储中批量查找配置
    /// 2. 将查询结果转换为 HashMap，键为 config_key，值为对应的 value
    ///
    /// # 返回
    /// 成功返回 config_key -> value 的 HashMap 映射，未找到的键不会出现在结果中
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_by_keys(&self, keys: &[String]) -> AppResult<HashMap<String, String>> {
        let configs = self.config_repo.find_by_keys(keys).await?;
        let map = configs.into_iter()
            .map(|c| (c.config_key, c.value))
            .collect();
        Ok(map)
    }
}

#[cfg(test)]
mod tests;
