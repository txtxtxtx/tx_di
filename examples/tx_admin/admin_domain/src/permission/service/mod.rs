use std::collections::HashSet;
use std::sync::Arc;
use tx_common::id;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use crate::permission::model::aggregate::Permission;
use crate::permission::model::value_object::{PermissionCheck, PermissionType};
use crate::permission::repository::PermissionRepository;
use crate::shared::repository::RepositoryError;

/// Permission domain service
#[tx_comp]
pub struct PermissionService {
    permission_repo: Arc<dyn PermissionRepository>,
}

impl PermissionService {
    /// 创建权限服务实例
    ///
    /// # 参数
    /// * `permission_repo` - 权限仓储的 Arc 智能指针，用于数据持久化操作
    pub fn new(permission_repo: Arc<dyn PermissionRepository>) -> Self {
        Self { permission_repo }
    }

    // === 原有查询方法 ===

    /// 获取指定用户的所有权限标识集合
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_by_user_id` 方法，查询该用户关联的所有权限标识
    ///
    /// # 返回
    /// 成功返回权限标识（permission code）的 `HashSet` 集合
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_user_permissions(
        &self,
        user_id: u64,
    ) -> AppResult<HashSet<String>> {
        self.permission_repo.find_by_user_id(user_id).await
    }

    /// 检查用户是否拥有指定权限
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    /// * `code` - 要检查的权限标识
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_by_user_id` 方法获取该用户的全部权限集合
    /// 2. 遍历权限集合，判断是否包含目标 `code`
    ///
    /// # 返回
    /// 成功返回 `true` 表示拥有该权限，`false` 表示不拥有
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn check_permission(
        &self,
        user_id: u64,
        code: &str,
    ) -> AppResult<bool> {
        let permissions = self.permission_repo.find_by_user_id(user_id).await?;
        Ok(permissions.iter().any(|p| p == code))
    }

    /// 获取指定角色集合的权限标识集合
    ///
    /// # 参数
    /// * `role_ids` - 角色 ID 切片，支持批量查询多个角色的权限
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_by_role_ids` 方法，查询这些角色关联的所有权限标识（自动去重）
    ///
    /// # 返回
    /// 成功返回权限标识（permission code）的 `HashSet` 集合
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_role_permissions(
        &self,
        role_ids: &[u64],
    ) -> AppResult<HashSet<String>> {
        self.permission_repo.find_by_role_ids(role_ids).await
    }

    /// 获取所有可用权限的轻量级列表
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_all` 方法，获取全部可用权限的精简信息
    ///
    /// # 返回
    /// 成功返回 `PermissionCheck` 轻量级权限对象的 `HashSet` 集合
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_all_permissions(
        &self,
    ) -> AppResult<HashSet<PermissionCheck>> {
        self.permission_repo.find_all().await
    }

    // === 新增 CRUD 方法 ===

    /// 创建新权限
    ///
    /// # 参数
    /// * `name` - 权限名称
    /// * `permission_code` - 权限标识码，全局唯一
    /// * `permission_type` - 权限类型（如菜单权限、按钮权限等）
    /// * `parent_id` - 父权限 ID，顶级权限传 0
    /// * `sort` - 排序号，数值越小越靠前
    /// * `description` - 权限描述（可选）
    /// * `creator` - 创建人标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 检查 `permission_code` 是否已存在，若存在则抛出 `DuplicatePermCode` 错误
    /// 2. 调用 `id::next_id()` 生成全局唯一权限 ID
    /// 3. 通过聚合根 `Permission::create` 构造权限实体
    /// 4. 调用仓储的 `insert` 方法将权限持久化到数据库
    ///
    /// # 返回
    /// 成功返回新创建的 `Permission` 聚合根实体
    ///
    /// # 错误
    /// - `DuplicatePermCode` - 权限标识码已存在，不允许重复创建
    /// - 数据库操作失败时返回仓储层错误
    pub async fn create_permission(
        &self,
        name: String,
        permission_code: String,
        permission_type: PermissionType,
        parent_id: u64,
        sort: i32,
        description: Option<String>,
        creator: Option<String>,
    ) -> AppResult<Permission> {
        if self.permission_repo.exists_by_code(&permission_code).await? {
            return Err(RepositoryError::DuplicatePermCode)?;
        }

        let id = id::next_id();
        let permission = Permission::create(
            id,
            name,
            permission_code,
            permission_type,
            parent_id,
            sort,
            description,
            creator,
        );
        self.permission_repo.insert(&permission).await?;
        Ok(permission)
    }

    /// 更新权限信息
    ///
    /// # 参数
    /// * `permission_id` - 要更新的权限 ID
    /// * `name` - 权限名称
    /// * `permission_code` - 权限标识码
    /// * `permission_type` - 权限类型
    /// * `parent_id` - 父权限 ID
    /// * `sort` - 排序号
    /// * `description` - 权限描述（可选）
    /// * `updater` - 更新人标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 根据 `permission_id` 从仓储查询权限，不存在则抛出 `NotFoundPerm` 错误
    /// 2. 检查 `permission_code` 是否已被其他权限使用，若已被占用则抛出 `DuplicatePermCode` 错误
    /// 3. 调用聚合根 `update_info` 方法更新权限属性
    /// 4. 调用仓储的 `update` 方法持久化变更
    ///
    /// # 返回
    /// 成功返回更新后的 `Permission` 聚合根实体
    ///
    /// # 错误
    /// - `NotFoundPerm` - 指定权限 ID 不存在
    /// - `DuplicatePermCode` - 权限标识码已被其他权限占用
    /// - 数据库操作失败时返回仓储层错误
    pub async fn update_permission(
        &self,
        permission_id: u64,
        name: String,
        permission_code: String,
        permission_type: PermissionType,
        parent_id: u64,
        sort: i32,
        description: Option<String>,
        updater: Option<String>,
    ) -> AppResult<Permission> {
        let mut permission = self
            .permission_repo
            .find_by_id(permission_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundPerm)?;

        // Check if code is taken by another permission
        if let Some(existing) = self.permission_repo.find_by_code(&permission_code).await? {
            if existing.id != permission_id {
                return Err(RepositoryError::DuplicatePermCode)?;
            }
        }

        permission.update_info(
            name,
            permission_code,
            permission_type,
            parent_id,
            sort,
            description,
            updater,
        );
        self.permission_repo.update(&permission).await?;
        Ok(permission)
    }

    /// 删除权限（软删除）
    ///
    /// # 参数
    /// * `permission_id` - 要删除的权限 ID
    /// * `updater` - 操作人标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 根据 `permission_id` 从仓储查询权限，不存在则抛出 `NotFoundPerm` 错误
    /// 2. 调用聚合根 `soft_delete` 方法标记为已删除
    /// 3. 调用仓储的 `update` 方法持久化删除状态
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundPerm` - 指定权限 ID 不存在
    /// - 数据库更新操作失败时返回仓储层错误
    pub async fn delete_permission(
        &self,
        permission_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        let mut permission = self
            .permission_repo
            .find_by_id(permission_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundPerm)?;

        permission.soft_delete(updater);
        self.permission_repo.update(&permission).await?;
        Ok(())
    }

    /// 根据 ID 获取单个权限详情
    ///
    /// # 参数
    /// * `permission_id` - 权限 ID
    ///
    /// # 执行逻辑
    /// 1. 根据 `permission_id` 从仓储查询权限实体
    /// 2. 若权限不存在则抛出 `NotFoundPerm` 错误
    ///
    /// # 返回
    /// 成功返回对应的 `Permission` 聚合根实体
    ///
    /// # 错误
    /// - `NotFoundPerm` - 指定权限 ID 不存在
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_permission(&self, permission_id: u64) -> AppResult<Permission> {
        Ok(self.permission_repo
            .find_by_id(permission_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundPerm)?)
    }

    /// 获取所有权限的完整实体列表
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_all_permissions` 方法，获取全部权限的完整实体信息
    ///
    /// # 返回
    /// 成功返回 `Permission` 完整实体的 `Vec` 列表
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_all_permission_details(&self) -> AppResult<Vec<Permission>> {
        self.permission_repo.find_all_permissions().await
    }
}

#[cfg(test)]
mod tests;
