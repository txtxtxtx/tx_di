//! 登录日志仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{LoginLog, LoginLogType, LoginResult, LoginLogRepository};

#[derive(Debug, Clone, Model)]
#[table = "system_login_log"]
pub struct LoginLogModel {
    #[key] #[auto] pub id: u64, pub log_type: LoginLogType, #[default("".to_string())] pub trace_id: String,
    #[default(0i64)] #[index] pub user_id: i64, #[default(0u8)] pub user_type: u8, #[default("".to_string())] pub username: String,
    pub result: LoginResult, #[default("".to_string())] pub user_ip: String, #[default("".to_string())] pub user_agent: String,
    #[index] pub tenant_id: i64, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<LoginLogModel> for LoginLog {
    fn from(m: LoginLogModel) -> Self { Self { id: m.id, log_type: m.log_type, trace_id: if m.trace_id.is_empty() { None } else { Some(m.trace_id) }, user_id: if m.user_id == 0 { None } else { Some(m.user_id as u64) }, user_type: m.user_type, username: if m.username.is_empty() { None } else { Some(m.username) }, result: m.result, user_ip: if m.user_ip.is_empty() { None } else { Some(m.user_ip) }, user_agent: if m.user_agent.is_empty() { None } else { Some(m.user_agent) }, tenant_id: m.tenant_id as u64, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } }
}

#[derive(Debug)] #[tx_comp]
pub struct ToastyLoginLogRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl LoginLogRepository for ToastyLoginLogRepository {
    async fn save(&self, log: &LoginLog) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone(); toasty::create!(LoginLogModel { log_type: log.log_type, trace_id: log.trace_id.clone().unwrap_or_default(), user_id: log.user_id.map(|v| v as i64).unwrap_or_default(), user_type: log.user_type, username: log.username.clone().unwrap_or_default(), result: log.result, user_ip: log.user_ip.clone().unwrap_or_default(), user_agent: log.user_agent.clone().unwrap_or_default(), tenant_id: log.tenant_id as i64, creator: log.creator.clone().unwrap_or_default(), updater: log.updater.clone().unwrap_or_default() }).exec(&mut db).await?; Ok(())
    }
    async fn find_page(&self, tenant_id: u64, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<LoginLog>, u64), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        let offset = (page - 1) * page_size;
        let total = LoginLogModel::filter_by_tenant_id(tenant_id as i64).count().exec(&mut db).await? as u64;
        let models = LoginLogModel::filter_by_tenant_id(tenant_id as i64)
            .offset(offset as usize)
            .limit(page_size as usize)
            .exec(&mut db)
            .await?;
        Ok((models.into_iter().filter(|m| m.deleted == 0).map(LoginLog::from).collect(), total))
    }
}
