use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::log::model::aggregate::{LoginLog, OperateLog};
use admin_domain::log::model::value_object::{LoginLogQuery, OperateLogQuery};
use admin_domain::log::repository::{LoginLogRepository, OperateLogRepository};
use admin_domain::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

pub struct MockOperateLogRepository {
    logs: RwLock<HashMap<u64, OperateLog>>,
}

impl MockOperateLogRepository {
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MockOperateLogRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OperateLogRepository for MockOperateLogRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<OperateLog>, RepositoryError> {
        let logs = self.logs.read().unwrap();
        Ok(logs.get(&id).cloned())
    }

    async fn find_page(
        &self,
        _query: &OperateLogQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<OperateLog>, RepositoryError> {
        let logs = self.logs.read().unwrap();
        let filtered: Vec<OperateLog> = logs.values().cloned().collect();
        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let list = filtered
            .into_iter()
            .skip(offset)
            .take(page.page_size as usize)
            .collect();
        Ok(PageResponse::new(list, total, page.page, page.page_size))
    }

    async fn insert(&self, log: &OperateLog) -> Result<(), RepositoryError> {
        let mut logs = self.logs.write().unwrap();
        logs.insert(log.id, log.clone());
        Ok(())
    }

    async fn delete_by_ids(&self, ids: &[u64]) -> Result<(), RepositoryError> {
        let mut logs = self.logs.write().unwrap();
        for id in ids {
            logs.remove(id);
        }
        Ok(())
    }

    async fn clean_all(&self) -> Result<(), RepositoryError> {
        let mut logs = self.logs.write().unwrap();
        logs.clear();
        Ok(())
    }
}

pub struct MockLoginLogRepository {
    logs: RwLock<HashMap<u64, LoginLog>>,
}

impl MockLoginLogRepository {
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MockLoginLogRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LoginLogRepository for MockLoginLogRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<LoginLog>, RepositoryError> {
        let logs = self.logs.read().unwrap();
        Ok(logs.get(&id).cloned())
    }

    async fn find_page(
        &self,
        _query: &LoginLogQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<LoginLog>, RepositoryError> {
        let logs = self.logs.read().unwrap();
        let filtered: Vec<LoginLog> = logs.values().cloned().collect();
        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let list = filtered
            .into_iter()
            .skip(offset)
            .take(page.page_size as usize)
            .collect();
        Ok(PageResponse::new(list, total, page.page, page.page_size))
    }

    async fn insert(&self, log: &LoginLog) -> Result<(), RepositoryError> {
        let mut logs = self.logs.write().unwrap();
        logs.insert(log.id, log.clone());
        Ok(())
    }

    async fn delete_by_ids(&self, ids: &[u64]) -> Result<(), RepositoryError> {
        let mut logs = self.logs.write().unwrap();
        for id in ids {
            logs.remove(id);
        }
        Ok(())
    }

    async fn clean_all(&self) -> Result<(), RepositoryError> {
        let mut logs = self.logs.write().unwrap();
        logs.clear();
        Ok(())
    }
}
