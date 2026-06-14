use std::sync::Arc;
use tx_common::id;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::department::model::aggregate::Department;
use crate::shared::model::value_object::DeletedStatus;
use crate::department::model::value_object::{DeptQuery, DeptTreeNode};
use crate::department::repository::DepartmentRepository;
use crate::shared::repository::RepositoryError::NotFound;

#[tx_comp]
pub struct DepartmentService {
    dept_repo: Arc<dyn DepartmentRepository>,
}

impl DepartmentService {
    pub fn new(dept_repo: Arc<dyn DepartmentRepository>) -> Self {
        Self { dept_repo }
    }

    pub async fn create_dept(
        &self,
        name: String,
        parent_id: u64,
        sort: i32,
        creator: Option<String>,
    ) -> AppResult<Department> {
        let dept_id = id::next_id();
        let dept = Department::create(dept_id, name, parent_id, sort, creator);
        self.dept_repo.insert(&dept).await?;
        Ok(dept)
    }

    pub async fn update_dept(
        &self,
        dept_id: u64,
        name: String,
        parent_id: u64,
        sort: i32,
        leader_user_id: Option<u64>,
        phone: Option<String>,
        email: Option<String>,
        updater: Option<String>,
    ) -> AppResult<Department> {
        let mut dept = self
            .dept_repo
            .find_by_id(dept_id)
            .await?
            .ok_or_else(|| NotFound)?;

        if parent_id == dept_id {
            return Err(RepositoryError::Validation)?;
        }

        dept.update_info(name, parent_id, sort, leader_user_id, phone, email, updater);
        self.dept_repo.update(&dept).await?;
        Ok(dept)
    }

    pub async fn delete_dept(
        &self,
        dept_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        if self.dept_repo.has_children(dept_id).await? {
            return Err(RepositoryError::Validation)?;
        }
        if self.dept_repo.has_users(dept_id).await? {
            return Err(RepositoryError::Validation)?;
        }

        let mut dept = self
            .dept_repo
            .find_by_id(dept_id)
            .await?
            .ok_or_else(|| NotFound)?;

        dept.soft_delete(updater);
        self.dept_repo.update(&dept).await?;
        Ok(())
    }

    pub async fn get_dept_tree(&self, query: &DeptQuery) -> AppResult<Vec<DeptTreeNode>> {
        let depts = self.dept_repo.find_all(query).await?;
        Ok(Self::build_tree(&depts, 0))
    }

    pub async fn get_all_depts(&self, query: &DeptQuery) -> AppResult<Vec<Department>> {
        self.dept_repo.find_all(query).await
    }

    pub async fn get_dept(&self, dept_id: u64) -> AppResult<Department> {
        Ok(self.dept_repo
            .find_by_id(dept_id)
            .await?
            .ok_or_else(|| NotFound)?)
    }

    fn build_tree(depts: &[Department], parent_id: u64) -> Vec<DeptTreeNode> {
        depts
            .iter()
            .filter(|d| d.parent_id == parent_id && d.audit.deleted == DeletedStatus::Normal)
            .map(|d| DeptTreeNode {
                id: d.id,
                name: d.name.clone(),
                parent_id: d.parent_id,
                sort: d.sort,
                leader_user_id: d.leader_user_id,
                status: d.status,
                children: Self::build_tree(depts, d.id),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
