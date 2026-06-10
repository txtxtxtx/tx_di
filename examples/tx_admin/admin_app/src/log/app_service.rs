use std::sync::Arc;

use crate::log::dto::*;
use admin_domain::log::model::value_object::{LoginLogQuery, OperateLogQuery};
use admin_domain::log::service::{LoginLogService, OperateLogService};
use tx_error::AppResult;
use tx_common::page::Page;

pub struct OperateLogAppService {
    log_service: Arc<OperateLogService>,
}

impl OperateLogAppService {
    pub fn new(log_service: Arc<OperateLogService>) -> Self {
        Self { log_service }
    }

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
        let page = Page::request(request.page, request.page_size);
        let result = self.log_service.get_log_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(OperateLogResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    pub async fn delete_logs(&self, ids: &[u64]) -> AppResult<()> {
        self.log_service.delete_logs(ids).await
    }

    pub async fn clean_logs(&self) -> AppResult<()> {
        self.log_service.clean_logs().await
    }
}

pub struct LoginLogAppService {
    log_service: Arc<LoginLogService>,
}

impl LoginLogAppService {
    pub fn new(log_service: Arc<LoginLogService>) -> Self {
        Self { log_service }
    }

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
        let page = Page::request(request.page, request.page_size);
        let result = self.log_service.get_log_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(LoginLogResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    pub async fn delete_logs(&self, ids: &[u64]) -> AppResult<()> {
        self.log_service.delete_logs(ids).await
    }

    pub async fn clean_logs(&self) -> AppResult<()> {
        self.log_service.clean_logs().await
    }
}
