use std::sync::Arc;

use crate::log::dto::*;
use admin_domain::log::model::value_object::{LoginLogQuery, OperateLogQuery};
use admin_domain::log::service::{LoginLogService, OperateLogService};
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

#[tx_comp]
pub struct OperateLogAppService {
    log_service: Arc<OperateLogService>,
}

impl OperateLogAppService {
    /// 创建操作日志应用服务实例
    ///
    /// # 参数
    /// * `log_service` - 操作日志领域服务，用于执行操作日志相关的业务逻辑
    pub fn new(log_service: Arc<OperateLogService>) -> Self {
        Self { log_service }
    }

    /// 创建操作日志记录
    ///
    /// # 参数
    /// * `cmd` - 创建操作日志命令，包含链路追踪ID、用户ID、用户类型、日志类型、子类型、业务ID、操作动作、是否成功、额外信息
    ///
    /// # 执行逻辑
    /// 委托给操作日志领域服务执行创建操作，逻辑详见 `OperateLogService::create_log`
    ///
    /// # 返回
    /// 成功返回 `OperateLogResponse`，包含操作日志完整信息
    ///
    /// # 错误
    /// - 数据库写入异常
    pub async fn create_log(
        &self,
        cmd: CreateOperateLogCommand,
    ) -> AppResult<OperateLogResponse> {
        let log = self
            .log_service
            .create_log(
                cmd.trace_id,
                cmd.user_id,
                cmd.user_type,
                cmd.log_type,
                cmd.sub_type,
                cmd.biz_id,
                cmd.action,
                cmd.success,
                cmd.extra,
            )
            .await?;
        Ok(OperateLogResponse::from(log))
    }

    /// 分页查询操作日志列表
    ///
    /// # 参数
    /// * `request` - 分页查询请求，包含用户ID、日志类型、子类型、是否成功、开始时间、结束时间等筛选条件，以及页码和每页大小
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `OperateLogQuery`
    /// 2. 构建分页参数 `Page`
    /// 3. 委托给操作日志领域服务执行分页查询
    /// 4. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Page<OperateLogResponse>`，包含操作日志列表、页码、每页大小和总记录数
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_log_page(
        &self,
        request: OperateLogQueryRequest,
    ) -> AppResult<Page<OperateLogResponse>> {
        let query = OperateLogQuery {
            user_id: request.user_id,
            log_type: request.log_type,
            sub_type: request.sub_type,
            success: request.success,
            begin_time: request.begin_time,
            end_time: request.end_time,
        };
        let page = Page::request(request.page, request.size);
        let result = self.log_service.get_log_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(OperateLogResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 批量删除操作日志
    ///
    /// # 参数
    /// * `ids` - 要删除的日志ID列表（引用切片）
    ///
    /// # 执行逻辑
    /// 委托给操作日志领域服务执行批量删除，逻辑详见 `OperateLogService::delete_logs`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - 数据库删除异常
    pub async fn delete_logs(&self, ids: &[u64]) -> AppResult<()> {
        self.log_service.delete_logs(ids).await
    }

    /// 清空所有操作日志
    ///
    /// # 执行逻辑
    /// 委托给操作日志领域服务清空全部日志记录，逻辑详见 `OperateLogService::clean_logs`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - 数据库删除异常
    pub async fn clean_logs(&self) -> AppResult<()> {
        self.log_service.clean_logs().await
    }
}

#[tx_comp]
pub struct LoginLogAppService {
    log_service: Arc<LoginLogService>,
}

impl LoginLogAppService {
    /// 创建登录日志应用服务实例
    ///
    /// # 参数
    /// * `log_service` - 登录日志领域服务，用于执行登录日志相关的业务逻辑
    pub fn new(log_service: Arc<LoginLogService>) -> Self {
        Self { log_service }
    }

    /// 创建登录日志记录
    ///
    /// # 参数
    /// * `cmd` - 创建登录日志命令，包含用户ID、用户类型、用户名、登录IP、登录类型、登录结果
    ///
    /// # 执行逻辑
    /// 委托给登录日志领域服务执行创建操作，逻辑详见 `LoginLogService::create_log`
    ///
    /// # 返回
    /// 成功返回 `LoginLogResponse`，包含登录日志完整信息
    ///
    /// # 错误
    /// - 数据库写入异常
    pub async fn create_log(
        &self,
        cmd: CreateLoginLogCommand,
    ) -> AppResult<LoginLogResponse> {
        let log = self
            .log_service
            .create_log(
                cmd.user_id,
                cmd.user_type,
                cmd.username,
                cmd.login_ip,
                cmd.login_type,
                cmd.result,
            )
            .await?;
        Ok(LoginLogResponse::from(log))
    }

    /// 分页查询登录日志列表
    ///
    /// # 参数
    /// * `request` - 分页查询请求，包含用户ID、用户名、登录IP、登录类型、登录结果、开始时间、结束时间等筛选条件，以及页码和每页大小
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `LoginLogQuery`
    /// 2. 构建分页参数 `Page`
    /// 3. 委托给登录日志领域服务执行分页查询
    /// 4. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Page<LoginLogResponse>`，包含登录日志列表、页码、每页大小和总记录数
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_log_page(
        &self,
        request: LoginLogQueryRequest,
    ) -> AppResult<Page<LoginLogResponse>> {
        let query = LoginLogQuery {
            user_id: request.user_id,
            username: request.username,
            login_ip: request.login_ip,
            login_type: request.login_type,
            result: request.result,
            begin_time: request.begin_time,
            end_time: request.end_time,
        };
        let page = Page::request(request.page, request.size);
        let result = self.log_service.get_log_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(LoginLogResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 批量删除登录日志
    ///
    /// # 参数
    /// * `ids` - 要删除的日志ID列表（引用切片）
    ///
    /// # 执行逻辑
    /// 委托给登录日志领域服务执行批量删除，逻辑详见 `LoginLogService::delete_logs`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - 数据库删除异常
    pub async fn delete_logs(&self, ids: &[u64]) -> AppResult<()> {
        self.log_service.delete_logs(ids).await
    }

    /// 清空所有登录日志
    ///
    /// # 执行逻辑
    /// 委托给登录日志领域服务清空全部日志记录，逻辑详见 `LoginLogService::clean_logs`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - 数据库删除异常
    pub async fn clean_logs(&self) -> AppResult<()> {
        self.log_service.clean_logs().await
    }
}
