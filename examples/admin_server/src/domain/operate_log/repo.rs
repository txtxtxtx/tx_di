//! 操作日志仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{OperateLog, OperateLogRepository};

#[derive(Debug, Clone, Model)]
#[table = "system_operate_log"]
pub struct OperateLogModel {
    #[key] #[auto] pub id: u64, #[default("".to_string())] pub trace_id: String, #[index] pub user_id: i64,
    #[default(0u8)] pub user_type: u8, pub op_type: String, pub sub_type: String, pub biz_id: i64,
    #[default("".to_string())] pub action: String, #[default(true)] pub success: bool, #[default("".to_string())] pub extra: String,
    #[default("".to_string())] pub request_method: String, #[default("".to_string())] pub request_url: String, #[default("".to_string())] pub user_ip: String, #[default("".to_string())] pub user_agent: String,
    #[index] pub tenant_id: i64, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<OperateLogModel> for OperateLog {
    fn from(m: OperateLogModel) -> Self { Self { id: m.id, trace_id: if m.trace_id.is_empty() { None } else { Some(m.trace_id) }, user_id: m.user_id as u64, user_type: m.user_type, op_type: m.op_type, sub_type: m.sub_type, biz_id: m.biz_id as u64, action: m.action, success: m.success, extra: m.extra, request_method: if m.request_method.is_empty() { None } else { Some(m.request_method) }, request_url: if m.request_url.is_empty() { None } else { Some(m.request_url) }, user_ip: if m.user_ip.is_empty() { None } else { Some(m.user_ip) }, user_agent: if m.user_agent.is_empty() { None } else { Some(m.user_agent) }, tenant_id: m.tenant_id as u64, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } }
}

#[derive(Debug)] #[tx_comp]
pub struct ToastyOperateLogRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl OperateLogRepository for ToastyOperateLogRepository {
    async fn save(&self, log: &OperateLog) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone(); toasty::create!(OperateLogModel { trace_id: log.trace_id.clone().unwrap_or_default(), user_id: log.user_id as i64, user_type: log.user_type, op_type: log.op_type.clone(), sub_type: log.sub_type.clone(), biz_id: log.biz_id as i64, action: log.action.clone(), success: log.success, extra: log.extra.clone(), request_method: log.request_method.clone().unwrap_or_default(), request_url: log.request_url.clone().unwrap_or_default(), user_ip: log.user_ip.clone().unwrap_or_default(), user_agent: log.user_agent.clone().unwrap_or_default(), tenant_id: log.tenant_id as i64, creator: log.creator.clone().unwrap_or_default(), updater: log.updater.clone().unwrap_or_default() }).exec(&mut db).await?; Ok(())
    }
    async fn find_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<OperateLog>, u64), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        let offset = (page - 1) * page_size;
        let total = OperateLogModel::filter_by_tenant_id(tenant_id as i64).count().exec(&mut db).await? as u64;
        let models = OperateLogModel::filter_by_tenant_id(tenant_id as i64)
            .offset(offset as usize)
            .limit(page_size as usize)
            .exec(&mut db)
            .await?;
        Ok((models.into_iter().filter(|m| m.deleted == 0).map(OperateLog::from).collect(), total))
    }
}
