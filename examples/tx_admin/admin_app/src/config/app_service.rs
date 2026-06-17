use std::collections::HashMap;
use std::sync::Arc;
use crate::config::dto::*;
use admin_domain::config::model::value_object::ConfigQuery;
use admin_domain::config::service::ConfigService;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

#[tx_comp]
pub struct ConfigAppService {
    config_service: Arc<ConfigService>,
}

impl ConfigAppService {
    /// 创建配置应用服务实例
    ///
    /// # 参数
    /// * `config_service` - 配置领域服务，用于执行系统配置相关的业务逻辑
    pub fn new(config_service: Arc<ConfigService>) -> Self {
        Self { config_service }
    }

    /// 创建新系统配置
    ///
    /// # 参数
    /// * `cmd` - 创建配置命令，包含分类、配置类型、名称、配置键、配置值
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给配置领域服务执行创建操作，逻辑详见 `ConfigService::create_config`
    ///
    /// # 返回
    /// 成功返回 `ConfigResponse`，包含配置完整信息
    ///
    /// # 错误
    /// - `DuplicateConfigKey` - 配置键已存在
    /// - 数据库写入异常
    pub async fn create_config(
        &self,
        cmd: CreateConfigCommand,
        creator: Option<String>,
    ) -> AppResult<ConfigResponse> {
        let config = self
            .config_service
            .create_config(cmd.category, cmd.config_type, cmd.name, cmd.config_key, cmd.value, creator)
            .await?;
        Ok(ConfigResponse::from(config))
    }

    /// 更新系统配置
    ///
    /// # 参数
    /// * `cmd` - 更新配置命令，包含配置ID、分类、配置类型、名称、配置键、配置值、是否可见、备注
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给配置领域服务执行更新操作，逻辑详见 `ConfigService::update_config`
    ///
    /// # 返回
    /// 成功返回更新后的 `ConfigResponse`
    ///
    /// # 错误
    /// - `NotFoundConfig` - 配置ID对应的配置不存在
    /// - `DuplicateConfigKey` - 配置键与其他配置冲突
    /// - 数据库更新异常
    pub async fn update_config(
        &self,
        cmd: UpdateConfigCommand,
        updater: Option<String>,
    ) -> AppResult<ConfigResponse> {
        let config = self
            .config_service
            .update_config(
                cmd.config_id,
                cmd.category,
                cmd.config_type,
                cmd.name,
                cmd.config_key,
                cmd.value,
                cmd.visible,
                cmd.remark,
                updater,
            )
            .await?;
        Ok(ConfigResponse::from(config))
    }

    /// 删除系统配置
    ///
    /// # 参数
    /// * `config_id` - 要删除的配置ID
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给配置领域服务执行删除操作，逻辑详见 `ConfigService::delete_config`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundConfig` - 配置ID对应的配置不存在
    /// - 数据库删除异常
    pub async fn delete_config(&self, config_id: u64, updater: Option<String>) -> AppResult<()> {
        self.config_service.delete_config(config_id, updater).await
    }

    /// 分页查询系统配置列表
    ///
    /// # 参数
    /// * `request` - 分页查询请求，包含名称、分类、配置键、配置类型等筛选条件，以及页码和每页大小
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `ConfigQuery`
    /// 2. 构建分页参数 `Page`
    /// 3. 委托给配置领域服务执行分页查询
    /// 4. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Page<ConfigResponse>`，包含配置列表、页码、每页大小和总记录数
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_config_page(
        &self,
        request: ConfigQueryRequest,
    ) -> AppResult<Page<ConfigResponse>> {
        let query = ConfigQuery {
            name: request.name,
            category: request.category,
            config_key: request.config_key,
            config_type: request.config_type,
        };
        let page = Page::request(request.page, request.size);
        let result = self.config_service.get_config_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(ConfigResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 根据ID获取配置信息
    ///
    /// # 参数
    /// * `config_id` - 配置ID
    ///
    /// # 执行逻辑
    /// 委托给配置领域服务查询配置，逻辑详见 `ConfigService::get_config`
    ///
    /// # 返回
    /// 成功返回 `ConfigResponse`
    ///
    /// # 错误
    /// - `NotFoundConfig` - 配置ID对应的配置不存在
    pub async fn get_config(&self, config_id: u64) -> AppResult<ConfigResponse> {
        let config = self.config_service.get_config(config_id).await?;
        Ok(ConfigResponse::from(config))
    }

    /// 根据配置键获取配置信息
    ///
    /// # 参数
    /// * `key` - 配置键名称
    ///
    /// # 执行逻辑
    /// 委托给配置领域服务根据键名查询配置，逻辑详见 `ConfigService::get_by_key`
    ///
    /// # 返回
    /// 成功返回 `ConfigResponse`
    ///
    /// # 错误
    /// - `NotFoundConfig` - 配置键对应的配置不存在
    pub async fn get_by_key(&self, key: &str) -> AppResult<ConfigResponse> {
        let config = self.config_service.get_by_key(key).await?;
        Ok(ConfigResponse::from(config))
    }

    /// 批量根据配置键获取配置值
    ///
    /// # 参数
    /// * `keys` - 配置键名称列表
    ///
    /// # 执行逻辑
    /// 委托给配置领域服务批量查询配置，返回键值对映射，逻辑详见 `ConfigService::get_by_keys`
    ///
    /// # 返回
    /// 成功返回 `HashMap<String, String>`，键为配置键名，值为对应的配置值
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_by_keys(&self, keys: Vec<String>) -> AppResult<HashMap<String, String>> {
        self.config_service.get_by_keys(&keys).await
    }
}
