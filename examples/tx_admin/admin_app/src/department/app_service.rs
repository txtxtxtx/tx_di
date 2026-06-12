use std::sync::Arc;

use crate::department::dto::*;
use admin_domain::department::model::value_object::{DeptQuery, DeptTreeNode};
use admin_domain::department::service::DepartmentService;
use tx_di_core::tx_comp;
use tx_error::AppResult;

#[tx_comp]
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
    ) -> AppResult<DeptResponse> {
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
    ) -> AppResult<DeptResponse> {
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

    pub async fn delete_dept(&self, dept_id: u64, updater: Option<String>) -> AppResult<()> {
        self.dept_service.delete_dept(dept_id, updater).await
    }

    pub async fn get_dept_list(
        &self,
        request: DeptQueryRequest,
    ) -> AppResult<Vec<DeptResponse>> {
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
    ) -> AppResult<Vec<DeptTreeNode>> {
        let query = DeptQuery {
            name: request.name,
            status: request.status,
        };
        self.dept_service.get_dept_tree(&query).await
    }

    pub async fn get_dept(&self, dept_id: u64) -> AppResult<DeptResponse> {
        let dept = self.dept_service.get_dept(dept_id).await?;
        Ok(DeptResponse::from(dept))
    }
}
