use std::sync::Arc;
use tx_common::id;
use tx_di_core::{Component, DepsTuple};
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::department::model::aggregate::Department;
use crate::shared::model::value_object::DeletedStatus;
use crate::department::model::value_object::{DeptQuery, DeptTreeNode};
use crate::department::repository::DepartmentRepository;

#[derive(Component)]
pub struct DepartmentService {
    dept_repo: Arc<dyn DepartmentRepository>,
}

impl DepartmentService {
    /// 创建部门服务实例
    ///
    /// # 参数
    /// * `dept_repo` - 部门仓储的 Arc 智能指针，用于数据持久化操作
    pub fn new(dept_repo: Arc<dyn DepartmentRepository>) -> Self {
        Self { dept_repo }
    }

    /// 创建新部门
    ///
    /// # 参数
    /// * `name` - 部门名称
    /// * `parent_id` - 父部门 ID，顶级部门传 0
    /// * `sort` - 排序号，数值越小越靠前
    /// * `creator` - 创建人标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 调用 `id::next_id()` 生成全局唯一部门 ID
    /// 2. 通过聚合根 `Department::create` 构造部门实体
    /// 3. 调用仓储的 `insert` 方法将部门持久化到数据库
    ///
    /// # 返回
    /// 成功返回新创建的 `Department` 聚合根实体
    ///
    /// # 错误
    /// - 数据库插入操作失败时返回仓储层错误
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

    /// 更新部门信息
    ///
    /// # 参数
    /// * `dept_id` - 要更新的部门 ID
    /// * `name` - 部门名称
    /// * `parent_id` - 父部门 ID
    /// * `sort` - 排序号
    /// * `leader_user_id` - 部门负责人用户 ID（可选）
    /// * `phone` - 联系电话（可选）
    /// * `email` - 联系邮箱（可选）
    /// * `updater` - 更新人标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 根据 `dept_id` 从仓储查询部门，不存在则抛出 `NotFoundDept` 错误
    /// 2. 校验 `parent_id` 不能等于 `dept_id`（不允许将自身设为父级）
    /// 3. 调用聚合根 `update_info` 方法更新部门属性
    /// 4. 调用仓储的 `update` 方法持久化变更
    ///
    /// # 返回
    /// 成功返回更新后的 `Department` 聚合根实体
    ///
    /// # 错误
    /// - `NotFoundDept` - 指定部门 ID 不存在
    /// - `ValidationDeptSelfParent` - 尝试将部门的父级设为自身
    /// - 数据库更新操作失败时返回仓储层错误
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
            .ok_or_else(|| RepositoryError::NotFoundDept)?;

        if parent_id == dept_id {
            return Err(RepositoryError::ValidationDeptSelfParent)?;
        }

        dept.update_info(name, parent_id, sort, leader_user_id, phone, email, updater);
        self.dept_repo.update(&dept).await?;
        Ok(dept)
    }

    /// 删除部门（软删除）
    ///
    /// # 参数
    /// * `dept_id` - 要删除的部门 ID
    /// * `updater` - 操作人标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 检查该部门是否存在子部门，若存在则拒绝删除
    /// 2. 检查该部门下是否存在用户，若存在则拒绝删除
    /// 3. 根据 `dept_id` 查询部门实体，不存在则抛出 `NotFoundDept` 错误
    /// 4. 调用聚合根 `soft_delete` 方法标记为已删除
    /// 5. 调用仓储的 `update` 方法持久化删除状态
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `ValidationDeptHasChildren` - 该部门下存在子部门，不允许删除
    /// - `ValidationDeptHasUsers` - 该部门下存在用户，不允许删除
    /// - `NotFoundDept` - 指定部门 ID 不存在
    /// - 数据库更新操作失败时返回仓储层错误
    pub async fn delete_dept(
        &self,
        dept_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        if self.dept_repo.has_children(dept_id).await? {
            return Err(RepositoryError::ValidationDeptHasChildren)?;
        }
        if self.dept_repo.has_users(dept_id).await? {
            return Err(RepositoryError::ValidationDeptHasUsers)?;
        }

        let mut dept = self
            .dept_repo
            .find_by_id(dept_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundDept)?;

        dept.soft_delete(updater);
        self.dept_repo.update(&dept).await?;
        Ok(())
    }

    /// 获取部门树形结构
    ///
    /// # 参数
    /// * `query` - 部门查询条件，用于筛选参与构建树的部门数据
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_all` 方法获取满足条件的全部部门列表
    /// 2. 调用 `build_tree` 递归方法，以 `parent_id = 0` 为根节点构建树形结构
    ///
    /// # 返回
    /// 成功返回 `DeptTreeNode` 树形结构列表，每个节点包含其子节点
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_dept_tree(&self, query: &DeptQuery) -> AppResult<Vec<DeptTreeNode>> {
        let depts = self.dept_repo.find_all(query).await?;
        Ok(Self::build_tree(&depts, 0))
    }

    /// 根据查询条件获取所有部门列表
    ///
    /// # 参数
    /// * `query` - 部门查询条件，包含筛选和排序参数
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_all` 方法，根据查询条件检索部门列表
    ///
    /// # 返回
    /// 成功返回匹配条件的 `Department` 实体列表
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_all_depts(&self, query: &DeptQuery) -> AppResult<Vec<Department>> {
        self.dept_repo.find_all(query).await
    }

    /// 根据 ID 获取单个部门详情
    ///
    /// # 参数
    /// * `dept_id` - 部门 ID
    ///
    /// # 执行逻辑
    /// 1. 根据 `dept_id` 从仓储查询部门实体
    /// 2. 若部门不存在则抛出 `NotFoundDept` 错误
    ///
    /// # 返回
    /// 成功返回对应的 `Department` 聚合根实体
    ///
    /// # 错误
    /// - `NotFoundDept` - 指定部门 ID 不存在
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_dept(&self, dept_id: u64) -> AppResult<Department> {
        Ok(self.dept_repo
            .find_by_id(dept_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundDept)?)
    }

    /// 递归构建部门树（内部方法）
    ///
    /// 筛选未删除且 `parent_id` 匹配的部门，递归组装子节点形成树形结构。
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
