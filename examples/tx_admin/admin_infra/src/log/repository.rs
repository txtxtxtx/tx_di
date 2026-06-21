use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::log::model::aggregate::{LoginLog, OperateLog};
use admin_domain::log::model::value_object::{LoginLogQuery, OperateLogQuery};
use admin_domain::log::repository::{LoginLogRepository, OperateLogRepository};
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::repository::{RepositoryError, db_err};
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::{SysLoginLog, SysOperateLog};
use crate::common::Deleted;

/// Toasty 实现的 OperateLogRepository
#[tx_comp(as_trait = dyn OperateLogRepository)]
pub struct ToastyOperateLogRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyOperateLogRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(l: &SysOperateLog) -> OperateLog {
        OperateLog::restore(
            l.id as u64,
            l.trace_id.clone(),
            l.user_id as u64,
            l.user_type,
            l.log_type.clone(),
            l.sub_type.clone(),
            l.biz_id as u64,
            l.action.clone(),
            l.success,
            l.extra.clone(),
            if l.request_method.is_empty() { None } else { Some(l.request_method.clone()) },
            if l.request_url.is_empty() { None } else { Some(l.request_url.clone()) },
            if l.user_ip.is_empty() { None } else { Some(l.user_ip.clone()) },
            if l.user_agent.is_empty() { None } else { Some(l.user_agent.clone()) },
            l.tenant_id,
            AuditFields {
                creator: if l.creator.is_empty() { None } else { Some(l.creator.clone()) },
                create_time: l.created_at,
                updater: if l.updater.is_empty() { None } else { Some(l.updater.clone()) },
                update_time: l.updated_at,
                deleted: if l.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl OperateLogRepository for ToastyOperateLogRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<OperateLog>> {
        let mut db = self.plugin.db().clone();
        match SysOperateLog::get_by_id(&mut db, id as i64).await {
            Ok(l) if l.deleted == Deleted::No => Ok(Some(Self::to_domain(&l))),
            _ => Ok(None),
        }
    }

    async fn find_page(&self, query: &OperateLogQuery, page: Page<OperateLog>) -> AppResult<Page<OperateLog>> {
        let mut db = self.plugin.db().clone();
        let all = SysOperateLog::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;

        let filtered: Vec<&SysOperateLog> = all
            .iter()
            .filter(|l| l.deleted == Deleted::No)
            .filter(|l| {
                if let Some(user_id) = query.user_id {
                    if l.user_id != user_id as i64 { return false; }
                }
                if let Some(ref log_type) = query.log_type {
                    if l.log_type != *log_type { return false; }
                }
                if let Some(ref sub_type) = query.sub_type {
                    if l.sub_type != *sub_type { return false; }
                }
                if let Some(success) = query.success {
                    if l.success != success { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let list: Vec<OperateLog> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(Self::to_domain)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn insert(&self, log: &OperateLog) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        SysOperateLog::create()
            .id(log.id as i64)
            .trace_id(log.trace_id.clone())
            .user_id(log.user_id as i64)
            .user_type(log.user_type)
            .log_type(log.log_type.clone())
            .sub_type(log.sub_type.clone())
            .biz_id(log.biz_id as i64)
            .action(log.action.clone())
            .success(log.success)
            .extra(log.extra.clone())
            .request_method(log.request_method.clone().unwrap_or_default())
            .request_url(log.request_url.clone().unwrap_or_default())
            .user_ip(log.user_ip.clone().unwrap_or_default())
            .user_agent(log.user_agent.clone().unwrap_or_default())
            .tenant_id(log.tenant_id)
            .creator(log.audit.creator.clone().unwrap_or_default())
            .updater(log.audit.updater.clone().unwrap_or_default())
            .deleted(Deleted::from(log.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;
        Ok(())
    }

    async fn delete_by_ids(&self, ids: &[u64]) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        for &id in ids {
            if let Ok(log) = SysOperateLog::get_by_id(&mut db, id as i64).await {
                log.delete().exec(&mut db)
                    .await
                    .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;
            }
        }
        Ok(())
    }

    async fn clean_all(&self) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let all = SysOperateLog::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;

        for log in all {
            log.delete().exec(&mut db)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;
        }
        Ok(())
    }
}

/// Toasty 实现的 LoginLogRepository
#[tx_comp(as_trait = dyn LoginLogRepository)]
pub struct ToastyLoginLogRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyLoginLogRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(l: &SysLoginLog) -> LoginLog {
        LoginLog::restore(
            l.id as u64,
            l.user_id as u64,
            l.user_type,
            l.username.clone(),
            l.login_ip.clone(),
            if l.login_location.is_empty() { None } else { Some(l.login_location.clone()) },
            if l.browser.is_empty() { None } else { Some(l.browser.clone()) },
            if l.os.is_empty() { None } else { Some(l.os.clone()) },
            l.login_type.clone(),
            l.result,
            if l.msg.is_empty() { None } else { Some(l.msg.clone()) },
            l.login_time,
            l.tenant_id,
            AuditFields {
                creator: if l.creator.is_empty() { None } else { Some(l.creator.clone()) },
                create_time: l.created_at,
                updater: if l.updater.is_empty() { None } else { Some(l.updater.clone()) },
                update_time: l.updated_at,
                deleted: if l.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl LoginLogRepository for ToastyLoginLogRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<LoginLog>> {
        let mut db = self.plugin.db().clone();
        match SysLoginLog::get_by_id(&mut db, id as i64).await {
            Ok(l) if l.deleted == Deleted::No => Ok(Some(Self::to_domain(&l))),
            _ => Ok(None),
        }
    }

    async fn find_page(&self, query: &LoginLogQuery, page: Page<LoginLog>) -> AppResult<Page<LoginLog>> {
        let mut db = self.plugin.db().clone();
        let all = SysLoginLog::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;

        let filtered: Vec<&SysLoginLog> = all
            .iter()
            .filter(|l| l.deleted == Deleted::No)
            .filter(|l| {
                if let Some(user_id) = query.user_id {
                    if l.user_id != user_id as i64 { return false; }
                }
                if let Some(ref username) = query.username {
                    if !l.username.contains(username.as_str()) { return false; }
                }
                if let Some(ref login_ip) = query.login_ip {
                    if !l.login_ip.contains(login_ip.as_str()) { return false; }
                }
                if let Some(ref login_type) = query.login_type {
                    if l.login_type != *login_type { return false; }
                }
                if let Some(result) = query.result {
                    if l.result != result { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let list: Vec<LoginLog> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(Self::to_domain)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn insert(&self, log: &LoginLog) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        SysLoginLog::create()
            .id(log.id as i64)
            .user_id(log.user_id as i64)
            .user_type(log.user_type)
            .username(log.username.clone())
            .login_ip(log.login_ip.clone())
            .login_location(log.login_location.clone().unwrap_or_default())
            .browser(log.browser.clone().unwrap_or_default())
            .os(log.os.clone().unwrap_or_default())
            .login_type(log.login_type.clone())
            .result(log.result)
            .msg(log.msg.clone().unwrap_or_default())
            .login_time(jiff::Timestamp::now())
            .tenant_id(log.tenant_id)
            .creator(log.audit.creator.clone().unwrap_or_default())
            .updater(log.audit.updater.clone().unwrap_or_default())
            .deleted(Deleted::from(log.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;
        Ok(())
    }

    async fn delete_by_ids(&self, ids: &[u64]) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        for &id in ids {
            if let Ok(log) = SysLoginLog::get_by_id(&mut db, id as i64).await {
                log.delete().exec(&mut db)
                    .await
                    .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;
            }
        }
        Ok(())
    }

    async fn clean_all(&self) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let all = SysLoginLog::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;

        for log in all {
            log.delete().exec(&mut db)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseLog))?;
        }
        Ok(())
    }
}
