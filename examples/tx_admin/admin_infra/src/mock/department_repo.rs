use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::department::model::aggregate::Department;
use admin_domain::department::model::value_object::DeptQuery;
use admin_domain::department::repository::DepartmentRepository;
use admin_domain::shared::repository::RepositoryError;
use admin_domain::shared::model::value_object::DeletedStatus;
use tx_di_core::{tx_comp, tx_cst};
use tx_error::AppResult;

#[tx_comp(as_trait = dyn DepartmentRepository)]
pub struct MockDepartmentRepository {
    #[tx_cst(RwLock::new(HashMap::new()))]
    depts: RwLock<HashMap<u64, Department>>,
    #[tx_cst(RwLock::new(HashMap::new()))]
    dept_users: RwLock<HashMap<u64, Vec<u64>>>,
}

impl MockDepartmentRepository {
    pub fn new() -> Self {
        Self {
            depts: RwLock::new(HashMap::new()),
            dept_users: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_dept(self, dept: Department) -> Self {
        {
            let mut depts = self.depts.write().unwrap();
            depts.insert(dept.id, dept);
        }
        self
    }
}

impl Default for MockDepartmentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DepartmentRepository for MockDepartmentRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Department>> {
        let depts = self.depts.read().unwrap();
        Ok(depts.get(&id).filter(|d| d.audit.deleted == DeletedStatus::Normal).cloned())
    }

    async fn find_all(&self, query: &DeptQuery) -> AppResult<Vec<Department>> {
        let depts = self.depts.read().unwrap();
        Ok(depts
            .values()
            .filter(|d| d.audit.deleted == DeletedStatus::Normal)
            .filter(|d| {
                if let Some(ref name) = query.name {
                    if !d.name.contains(name.as_str()) {
                        return false;
                    }
                }
                if let Some(status) = query.status {
                    if d.status != status {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect())
    }

    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Department>> {
        let depts = self.depts.read().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| depts.get(id))
            .filter(|d| d.audit.deleted == DeletedStatus::Normal)
            .cloned()
            .collect())
    }

    async fn find_by_parent_id(&self, parent_id: u64) -> AppResult<Vec<Department>> {
        let depts = self.depts.read().unwrap();
        Ok(depts
            .values()
            .filter(|d| d.parent_id == parent_id && d.audit.deleted == DeletedStatus::Normal)
            .cloned()
            .collect())
    }

    async fn insert(&self, dept: &Department) -> AppResult<()> {
        let mut depts = self.depts.write().unwrap();
        depts.insert(dept.id, dept.clone());
        Ok(())
    }

    async fn update(&self, dept: &Department) -> AppResult<()> {
        let mut depts = self.depts.write().unwrap();
        depts.insert(dept.id, dept.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut depts = self.depts.write().unwrap();
        if let Some(dept) = depts.get_mut(&id) {
            dept.audit.deleted = DeletedStatus::Deleted;
            Ok(())
        } else {
            Err(RepositoryError::NotFound)?
        }
    }

    async fn has_children(&self, parent_id: u64) -> AppResult<bool> {
        let depts = self.depts.read().unwrap();
        Ok(depts
            .values()
            .any(|d| d.parent_id == parent_id && d.audit.deleted == DeletedStatus::Normal))
    }

    async fn has_users(&self, dept_id: u64) -> AppResult<bool> {
        let dept_users = self.dept_users.read().unwrap();
        Ok(dept_users
            .get(&dept_id)
            .map(|users| !users.is_empty())
            .unwrap_or(false))
    }
}
