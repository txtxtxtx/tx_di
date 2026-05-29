//! 日志仓储 — toasty 0.6 实现（登录日志 + 操作日志 + API 日志）

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::login_log::LoginLog;
use crate::domain::operate_log::OperateLog;
use crate::domain::api_log::{ApiAccessLog, ApiErrorLog};

/// 日志仓储 trait
#[async_trait]
pub trait LogRepository: Send + Sync {
    // LoginLog
    async fn save_login_log(&self, log: &LoginLog) -> Result<(), anyhow::Error>;
    async fn find_login_log_page(&self, tenant_id: i64, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<LoginLog>, u64), anyhow::Error>;
    // OperateLog
    async fn save_operate_log(&self, log: &OperateLog) -> Result<(), anyhow::Error>;
    async fn find_operate_log_page(&self, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<OperateLog>, u64), anyhow::Error>;
    // ApiAccessLog
    async fn save_api_access_log(&self, log: &ApiAccessLog) -> Result<(), anyhow::Error>;
    async fn find_api_access_log_page(&self, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<ApiAccessLog>, u64), anyhow::Error>;
    // ApiErrorLog
    async fn save_api_error_log(&self, log: &ApiErrorLog) -> Result<(), anyhow::Error>;
    async fn find_api_error_log_page(&self, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<ApiErrorLog>, u64), anyhow::Error>;
}

/// 日志仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyLogRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl LogRepository for ToastyLogRepository {
    async fn save_login_log(&self, log: &LoginLog) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        log.clone().create(db).await?;
        Ok(())
    }

    async fn find_login_log_page(&self, tenant_id: i64, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<LoginLog>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let mut stmt = LoginLog::filter(LoginLog::tenant_id.eq(tenant_id).and(LoginLog::deleted.eq(0i16)));
        if let Some(kw) = keyword {
            stmt = stmt.filter(LoginLog::username.like(format!("%{}%", kw)));
        }
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let logs = stmt.order(LoginLog::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((logs, total))
    }

    async fn save_operate_log(&self, log: &OperateLog) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        log.clone().create(db).await?;
        Ok(())
    }

    async fn find_operate_log_page(&self, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<OperateLog>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let stmt = OperateLog::filter(OperateLog::tenant_id.eq(tenant_id).and(OperateLog::deleted.eq(0i16)));
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let logs = stmt.order(OperateLog::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((logs, total))
    }

    async fn save_api_access_log(&self, log: &ApiAccessLog) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        log.clone().create(db).await?;
        Ok(())
    }

    async fn find_api_access_log_page(&self, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<ApiAccessLog>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let stmt = ApiAccessLog::filter(ApiAccessLog::tenant_id.eq(tenant_id).and(ApiAccessLog::deleted.eq(0i16)));
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let logs = stmt.order(ApiAccessLog::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((logs, total))
    }

    async fn save_api_error_log(&self, log: &ApiErrorLog) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        log.clone().create(db).await?;
        Ok(())
    }

    async fn find_api_error_log_page(&self, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<ApiErrorLog>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let stmt = ApiErrorLog::filter(ApiErrorLog::tenant_id.eq(tenant_id).and(ApiErrorLog::deleted.eq(0i16)));
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let logs = stmt.order(ApiErrorLog::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((logs, total))
    }
}
