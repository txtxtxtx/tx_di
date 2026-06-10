use std::sync::Arc;

use crate::department::dto::*;
use admin_domain::department::model::value_object::{DeptQuery, DeptTreeNode};
use admin_domain::department::service::DepartmentService;
use admin_domain::shared::repository::RepositoryError;

pub struct DepartmentAppService {
    dept_service: Arc<DepartmentService>,
}

impl DepartmentAppService {
    pub fn new(dept_service: Arc<DepartmentService>) -> Self {
        Self { dept_service }
    }

    pub async fn create_dept(
        &self,
        cmd: CreateDeptCommand,
        creator: Option<String>,
    ) -> Result<DeptResponse, RepositoryError> {
        let dept = self
            .dept_service
            .create_dept(cmd.name, cmd.parent_id, cmd.sort, creator)
            .await?;
        Ok(DeptResponse::from(dept))
    }

    pub async fn update_dept(
        &self,
        cmd: UpdateDeptCommand,
        updater: Option<String>,
    ) -> Result<DeptResponse, RepositoryError> {
        let dept = self
            .dept_service
            .update_dept(
                cmd.dept_id,
                cmd.name,
                cmd.parent_id,
                cmd.sort,
                cmd.leader_user_id,
                cmd.phone,
                cmd.email,
                updater,
            )
            .await?;
        Ok(DeptResponse::from(dept))
    }

    pub async fn delete_dept(&self, dept_id: u64, updater: Option<String>) -> Result<(), RepositoryError> {
        self.dept_service.delete_dept(dept_id, updater).await
    }

    pub async fn get_dept_list(
        &self,
        request: DeptQueryRequest,
    ) -> Result<Vec<DeptResponse>, RepositoryError> {
        let query = DeptQuery {
            name: request.name,
            status: request.status,
        };
        let depts = self.dept_service.get_all_depts(&query).await?;
        Ok(depts.into_iter().map(DeptResponse::from).collect())
    }

    pub async fn get_dept_tree(
        &self,
        request: DeptQueryRequest,
    ) -> Result<Vec<DeptTreeNode>, RepositoryError> {
        let query = DeptQuery {
            name: request.name,
            status: request.status,
        };
        self.dept_service.get_dept_tree(&query).await
    }

    pub async fn get_dept(&self, dept_id: u64) -> Result<DeptResponse, RepositoryError> {
        let dept = self.dept_service.get_dept(dept_id).await?;
        Ok(DeptResponse::from(dept))
    }
}
