use std::sync::Arc;

use crate::log::model::aggregate::{LoginLog, OperateLog};
use crate::log::model::value_object::{LoginLogQuery, OperateLogQuery};
use crate::log::repository::{LoginLogRepository, OperateLogRepository};
use crate::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};
use admin_common::id;

pub struct OperateLogService {
    log_repo: Arc<dyn OperateLogRepository>,
}

impl OperateLogService {
    pub fn new(log_repo: Arc<dyn OperateLogRepository>) -> Self {
        Self { log_repo }
    }

    pub async fn create_log(
        &self,
        trace_id: String,
        user_id: u64,
        user_type: i32,
        log_type: String,
        sub_type: String,
        biz_id: u64,
        action: String,
        success: i32,
        extra: String,
    ) -> Result<OperateLog, RepositoryError> {
        let log_id = id::next_id();
        let log = OperateLog::create(
            log_id, trace_id, user_id, user_type, log_type, sub_type, biz_id, action, success, extra,
        );
        self.log_repo.insert(&log).await?;
        Ok(log)
    }

    pub async fn get_log_page(
        &self,
        query: &OperateLogQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<OperateLog>, RepositoryError> {
        self.log_repo.find_page(query, page).await
    }

    pub async fn delete_logs(&self, ids: &[u64]) -> Result<(), RepositoryError> {
        self.log_repo.delete_by_ids(ids).await
    }

    pub async fn clean_logs(&self) -> Result<(), RepositoryError> {
        self.log_repo.clean_all().await
    }
}

pub struct LoginLogService {
    log_repo: Arc<dyn LoginLogRepository>,
}

impl LoginLogService {
    pub fn new(log_repo: Arc<dyn LoginLogRepository>) -> Self {
        Self { log_repo }
    }

    pub async fn create_log(
        &self,
        user_id: u64,
        user_type: i32,
        username: String,
        login_ip: String,
        login_type: String,
        result: i32,
    ) -> Result<LoginLog, RepositoryError> {
        let log_id = id::next_id();
        let log = LoginLog::create(log_id, user_id, user_type, username, login_ip, login_type, result);
        self.log_repo.insert(&log).await?;
        Ok(log)
    }

    pub async fn get_log_page(
        &self,
        query: &LoginLogQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<LoginLog>, RepositoryError> {
        self.log_repo.find_page(query, page).await
    }

    pub async fn delete_logs(&self, ids: &[u64]) -> Result<(), RepositoryError> {
        self.log_repo.delete_by_ids(ids).await
    }

    pub async fn clean_logs(&self) -> Result<(), RepositoryError> {
        self.log_repo.clean_all().await
    }
}
