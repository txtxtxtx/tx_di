use std::sync::Arc;

use admin_proto::{CreateOperateLogRequest, ListOperateLogsRequest, OperateLogResponse,
                  CreateLoginLogRequest, ListLoginLogsRequest, LoginLogResponse};
use admin_domain::log::model::value_object::{LoginLogQuery, OperateLogQuery};
use admin_domain::log::service::{LoginLogService, OperateLogService};
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

use crate::log::dto::{operate_log_to_response, login_log_to_response};

#[tx_comp]
pub struct OperateLogAppService {
    log_service: Arc<OperateLogService>,
}

impl OperateLogAppService {
    pub fn new(log_service: Arc<OperateLogService>) -> Self {
        Self { log_service }
    }

    /// 创建操作日志记录
    pub async fn create_log(
        &self,
        req: CreateOperateLogRequest,
    ) -> AppResult<OperateLogResponse> {
        let log = self
            .log_service
            .create_log(
                req.trace_id,
                req.user_id,
                req.user_type,
                req.log_type,
                req.sub_type,
                req.biz_id,
                req.action,
                req.success,
                req.extra,
            )
            .await?;
        Ok(operate_log_to_response(log))
    }

    /// 分页查询操作日志列表
    pub async fn get_log_page(
        &self,
        req: ListOperateLogsRequest,
    ) -> AppResult<Page<OperateLogResponse>> {
        let query = OperateLogQuery {
            user_id: req.user_id,
            log_type: req.log_type,
            sub_type: req.sub_type,
            success: req.success,
            begin_time: req.begin_time,
            end_time: req.end_time,
        };
        let page = Page::request(req.page, req.page_size);
        let result = self.log_service.get_log_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(operate_log_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 批量删除操作日志
    pub async fn delete_logs(&self, ids: &[u64]) -> AppResult<()> {
        self.log_service.delete_logs(ids).await
    }

    /// 清空所有操作日志
    pub async fn clean_logs(&self) -> AppResult<()> {
        self.log_service.clean_logs().await
    }
}

#[tx_comp]
pub struct LoginLogAppService {
    log_service: Arc<LoginLogService>,
}

impl LoginLogAppService {
    pub fn new(log_service: Arc<LoginLogService>) -> Self {
        Self { log_service }
    }

    /// 创建登录日志记录
    pub async fn create_log(
        &self,
        req: CreateLoginLogRequest,
    ) -> AppResult<LoginLogResponse> {
        let log = self
            .log_service
            .create_log(
                req.user_id,
                req.user_type,
                req.username,
                req.login_ip,
                req.login_type,
                req.result,
            )
            .await?;
        Ok(login_log_to_response(log))
    }

    /// 分页查询登录日志列表
    pub async fn get_log_page(
        &self,
        req: ListLoginLogsRequest,
    ) -> AppResult<Page<LoginLogResponse>> {
        let query = LoginLogQuery {
            user_id: req.user_id,
            username: req.username,
            login_ip: req.login_ip,
            login_type: req.login_type,
            result: req.result,
            begin_time: req.begin_time,
            end_time: req.end_time,
        };
        let page = Page::request(req.page, req.page_size);
        let result = self.log_service.get_log_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(login_log_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 批量删除登录日志
    pub async fn delete_logs(&self, ids: &[u64]) -> AppResult<()> {
        self.log_service.delete_logs(ids).await
    }

    /// 清空所有登录日志
    pub async fn clean_logs(&self) -> AppResult<()> {
        self.log_service.clean_logs().await
    }
}
