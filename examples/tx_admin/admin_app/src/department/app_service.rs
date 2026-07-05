use std::sync::Arc;

use crate::department::dto::*;
use admin_domain::department::model::value_object::{DeptQuery, DeptTreeNode};
use admin_domain::department::service::DepartmentService;
use tx_di_core::{Component, DepsTuple};
use tx_error::AppResult;

#[derive(Component)]
pub struct DepartmentAppService {
    dept_service: Arc<DepartmentService>,
}

impl DepartmentAppService {
    /// 创建部门应用服务实例
    ///
    /// # 参数
    /// * `dept_service` - 部门领域服务，用于执行部门相关的业务逻辑
    pub fn new(dept_service: Arc<DepartmentService>) -> Self {
        Self { dept_service }
    }

    /// 创建新部门
    ///
    /// # 参数
    /// * `req` - 创建部门请求，包含部门名称、父部门ID、排序号
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给部门领域服务执行创建操作，逻辑详见 `DepartmentService::create_dept`
    ///
    /// # 返回
    /// 成功返回 `DeptResponse`，包含部门完整信息
    ///
    /// # 错误
    /// - `NotFoundDept` - 父部门ID对应的部门不存在
    /// - 数据库写入异常
    pub async fn create_dept(
        &self,
        req: CreateDeptRequest,
        creator: Option<String>,
    ) -> AppResult<DeptResponse> {
        let dept = self
            .dept_service
            .create_dept(req.name, req.parent_id, req.sort, creator)
            .await?;
        Ok(dept_to_response(dept))
    }

    /// 更新部门信息
    ///
    /// # 参数
    /// * `req` - 更新部门请求，包含部门ID、名称、父部门ID、排序号、负责人用户ID、联系电话、邮箱
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给部门领域服务执行更新操作，逻辑详见 `DepartmentService::update_dept`
    ///
    /// # 返回
    /// 成功返回更新后的 `DeptResponse`
    ///
    /// # 错误
    /// - `NotFoundDept` - 部门ID对应的部门不存在
    /// - 数据库更新异常
    pub async fn update_dept(
        &self,
        req: UpdateDeptRequest,
        updater: Option<String>,
    ) -> AppResult<DeptResponse> {
        let dept = self
            .dept_service
            .update_dept(
                req.dept_id,
                req.name,
                req.parent_id,
                req.sort,
                req.leader_user_id,
                req.phone,
                req.email,
                updater,
            )
            .await?;
        Ok(dept_to_response(dept))
    }

    /// 删除部门
    ///
    /// # 参数
    /// * `dept_id` - 要删除的部门ID
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给部门领域服务执行删除操作，逻辑详见 `DepartmentService::delete_dept`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundDept` - 部门ID对应的部门不存在
    /// - 存在子部门时可能拒绝删除
    /// - 数据库删除异常
    pub async fn delete_dept(&self, dept_id: u64, updater: Option<String>) -> AppResult<()> {
        self.dept_service.delete_dept(dept_id, updater).await
    }

    /// 获取部门列表（扁平结构）
    ///
    /// # 参数
    /// * `request` - 查询请求，包含部门名称、状态等筛选条件
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `DeptQuery`
    /// 2. 委托给部门领域服务查询所有符合条件的部门
    /// 3. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Vec<DeptResponse>`，包含所有符合条件的部门列表（扁平结构）
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_dept_list(
        &self,
        request: ListDeptsRequest,
    ) -> AppResult<Vec<DeptResponse>> {
        let query = DeptQuery {
            name: request.name,
            status: request.status,
        };
        let depts = self.dept_service.get_all_depts(&query).await?;
        Ok(depts.into_iter().map(dept_to_response).collect())
    }

    /// 获取部门树结构
    ///
    /// # 参数
    /// * `request` - 查询请求，包含部门名称、状态等筛选条件
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `DeptQuery`
    /// 2. 委托给部门领域服务构建部门树结构
    ///
    /// # 返回
    /// 成功返回 `Vec<DeptTreeNode>`，包含树形结构的部门列表
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_dept_tree(
        &self,
        request: ListDeptsRequest,
    ) -> AppResult<Vec<DeptTreeNode>> {
        let query = DeptQuery {
            name: request.name,
            status: request.status,
        };
        self.dept_service.get_dept_tree(&query).await
    }

    /// 根据ID获取部门信息
    ///
    /// # 参数
    /// * `dept_id` - 部门ID
    ///
    /// # 执行逻辑
    /// 委托给部门领域服务查询部门，逻辑详见 `DepartmentService::get_dept`
    ///
    /// # 返回
    /// 成功返回 `DeptResponse`
    ///
    /// # 错误
    /// - `NotFoundDept` - 部门ID对应的部门不存在
    pub async fn get_dept(&self, dept_id: u64) -> AppResult<DeptResponse> {
        let dept = self.dept_service.get_dept(dept_id).await?;
        Ok(dept_to_response(dept))
    }
}
