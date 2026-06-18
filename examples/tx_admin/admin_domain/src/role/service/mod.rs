use std::sync::Arc;
use tx_common::id;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::role::model::aggregate::Role;
use crate::role::model::value_object::RoleQuery;
use crate::role::repository::RoleRepository;
use crate::user::repository::UserRepository;
use crate::user::model::value_object::UserStatus;

/// Role domain service
#[tx_comp]
pub struct RoleService {
    role_repo: Arc<dyn RoleRepository>,
    user_repo: Arc<dyn UserRepository>,
}

impl RoleService {
    /// 创建 RoleService 的新实例
    ///
    /// # 参数
    /// * `role_repo` - 角色仓库，用于角色相关的数据库操作
    /// * `user_repo` - 用户仓库，用于校验用户状态
    pub fn new(role_repo: Arc<dyn RoleRepository>, user_repo: Arc<dyn UserRepository>) -> Self {
        Self { role_repo, user_repo }
    }

    /// 创建新角色
    ///
    /// # 参数
    /// * `name` - 角色名称
    /// * `code` - 角色编码，必须全局唯一
    /// * `sort` - 排序值，用于角色列表的显示顺序
    /// * `creator` - 创建者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 检查角色编码是否已存在，若存在则返回重复错误
    /// 2. 生成全局唯一角色 ID
    /// 3. 构建角色聚合根并持久化到数据库
    ///
    /// # 返回
    /// 成功返回新创建的 `Role` 聚合根
    ///
    /// # 错误
    /// - `DuplicateRoleCode` - 角色编码已被占用
    /// - 数据库插入失败时返回错误
    pub async fn create_role(
        &self,
        name: String,
        code: String,
        sort: i32,
        creator: Option<String>,
    ) -> AppResult<Role> {
        if self.role_repo.exists_by_code(&code).await? {
            return Err(RepositoryError::DuplicateRoleCode)?;
        }

        let role_id = id::next_id();
        let role = Role::create(role_id, name, code, sort, creator);
        self.role_repo.insert(&role).await?;
        Ok(role)
    }

    /// 更新角色信息
    ///
    /// # 参数
    /// * `role_id` - 角色 ID
    /// * `name` - 新角色名称
    /// * `code` - 新角色编码
    /// * `sort` - 新排序值
    /// * `data_scope` - 数据权限范围
    /// * `remark` - 备注，可选
    /// * `updater` - 更新者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据角色 ID 查询角色，若不存在则返回未找到错误
    /// 2. 检查新角色编码是否已被其他角色使用，若冲突则返回重复错误
    /// 3. 调用角色聚合根的 `update_info` 方法更新信息
    /// 4. 将更新后的角色持久化到数据库
    ///
    /// # 返回
    /// 成功返回更新后的 `Role` 聚合根
    ///
    /// # 错误
    /// - `NotFoundRole` - 指定角色不存在
    /// - `DuplicateRoleCode` - 角色编码已被其他角色使用
    /// - 数据库更新失败时返回错误
    pub async fn update_role(
        &self,
        role_id: u64,
        name: String,
        code: String,
        sort: i32,
        data_scope: i32,
        remark: Option<String>,
        updater: Option<String>,
    ) -> AppResult<Role> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?;

        // Check if code is taken by another role
        if let Some(existing) = self.role_repo.find_by_code(&code).await? {
            if existing.id != role_id {
                return Err(RepositoryError::DuplicateRoleCode)?;
            }
        }

        role.update_info(name, code, sort, data_scope, remark, updater);
        self.role_repo.update(&role).await?;
        Ok(role)
    }

    /// 软删除角色
    ///
    /// # 参数
    /// * `role_id` - 角色 ID
    /// * `updater` - 操作者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据角色 ID 查询角色，若不存在则返回未找到错误
    /// 2. 调用角色聚合根的 `soft_delete` 方法标记为已删除状态
    /// 3. 将更新后的角色持久化到数据库
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundRole` - 指定角色不存在
    /// - 数据库更新失败时返回错误
    pub async fn delete_role(
        &self,
        role_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?;

        role.soft_delete(updater);
        self.role_repo.update(&role).await?;
        Ok(())
    }

    /// 变更角色状态（启用/禁用）
    ///
    /// # 参数
    /// * `role_id` - 角色 ID
    /// * `status` - 目标状态（0 为启用，其他值为禁用）
    /// * `updater` - 操作者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据角色 ID 查询角色，若不存在则返回未找到错误
    /// 2. 调用角色聚合根的 `change_status` 方法变更状态
    /// 3. 将更新后的角色持久化到数据库
    ///
    /// # 返回
    /// 成功返回更新后的 `Role` 聚合根
    ///
    /// # 错误
    /// - `NotFoundRole` - 指定角色不存在
    /// - 数据库更新失败时返回错误
    pub async fn change_status(
        &self,
        role_id: u64,
        status: i32,
        updater: Option<String>,
    ) -> AppResult<Role> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?;

        role.change_status(status, updater);
        self.role_repo.update(&role).await?;
        Ok(role)
    }

    /// 为角色分配菜单权限
    ///
    /// # 参数
    /// * `role_id` - 角色 ID
    /// * `menu_ids` - 要分配的菜单 ID 列表
    ///
    /// # 执行逻辑
    /// 1. 根据角色 ID 查询角色，若不存在则返回未找到错误
    /// 2. 校验角色状态必须为启用（status == 0），否则返回角色已禁用错误
    /// 3. 更新角色聚合根的菜单 ID 列表
    /// 4. 调用角色仓库绑定角色与菜单的关联关系
    /// 5. 将更新后的角色持久化到数据库
    ///
    /// # 返回
    /// 成功返回更新后的 `Role` 聚合根
    ///
    /// # 错误
    /// - `NotFoundRole` - 指定角色不存在
    /// - `ValidationRoleDisabled` - 角色已禁用，无法分配菜单权限
    /// - 数据库操作失败时返回错误
    pub async fn assign_menus(
        &self,
        role_id: u64,
        menu_ids: Vec<u64>,
    ) -> AppResult<Role> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?;

        // 角色必须为启用状态才能分配菜单
        if role.status != 0 {
            return Err(RepositoryError::ValidationRoleDisabled)?;
        }

        role.set_menus(menu_ids.clone());
        self.role_repo.bind_menus(role_id, &menu_ids).await?;
        self.role_repo.update(&role).await?;
        Ok(role)
    }

    /// 分页查询角色列表
    ///
    /// # 参数
    /// * `query` - 查询条件，包含角色名称、状态等筛选项
    /// * `page` - 分页参数，包含页码和每页数量
    ///
    /// # 执行逻辑
    /// 1. 调用角色仓库的分页查询方法，根据条件筛选并返回分页结果
    ///
    /// # 返回
    /// 成功返回分页结果 `Page<Role>`，包含角色列表和分页元数据
    ///
    /// # 错误
    /// - 数据库查询异常时返回错误
    pub async fn get_role_page(
        &self,
        query: &RoleQuery,
        page: Page<Role>,
    ) -> AppResult<Page<Role>> {
        self.role_repo.find_page(query, page).await
    }

    /// 根据 ID 获取角色详情
    ///
    /// # 参数
    /// * `role_id` - 角色 ID
    ///
    /// # 执行逻辑
    /// 1. 根据角色 ID 查询角色，若不存在则返回未找到错误
    ///
    /// # 返回
    /// 成功返回 `Role` 聚合根
    ///
    /// # 错误
    /// - `NotFoundRole` - 指定角色不存在
    /// - 数据库查询异常时返回错误
    pub async fn get_role(&self, role_id: u64) -> AppResult<Role> {
        Ok(self.role_repo
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundRole)?)
    }

    /// 根据 ID 列表批量获取角色
    ///
    /// # 参数
    /// * `ids` - 角色 ID 列表
    ///
    /// # 执行逻辑
    /// 1. 调用角色仓库根据 ID 列表批量查询角色记录
    ///
    /// # 返回
    /// 成功返回匹配的角色列表 `Vec<Role>`
    ///
    /// # 错误
    /// - 数据库查询异常时返回错误
    pub async fn get_roles_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Role>> {
        self.role_repo.find_by_ids(ids).await
    }

    /// 获取所有角色列表
    ///
    /// # 参数
    /// * `query` - 查询条件，包含角色名称、状态等筛选项
    ///
    /// # 执行逻辑
    /// 1. 调用角色仓库查询所有符合条件的角色记录
    ///
    /// # 返回
    /// 成功返回角色列表 `Vec<Role>`
    ///
    /// # 错误
    /// - 数据库查询异常时返回错误
    pub async fn get_all_roles(&self, query: &RoleQuery) -> AppResult<Vec<Role>> {
        self.role_repo.find_all(query).await
    }

    /// 获取角色关联的用户列表
    ///
    /// # 参数
    /// * `role_id` - 角色 ID
    ///
    /// # 执行逻辑
    /// 1. 根据角色 ID 查询角色，若不存在则返回未找到错误
    /// 2. 查询该角色关联的所有用户列表
    ///
    /// # 返回
    /// 成功返回用户列表 `Vec<User>`
    ///
    /// # 错误
    /// - `NotFoundRole` - 指定角色不存在
    /// - 数据库查询异常时返回错误
    pub async fn get_role_users(&self, role_id: u64) -> AppResult<Vec<crate::user::model::aggregate::User>> {
        // Verify role exists
        let _role = self.role_repo.find_by_id(role_id).await?.ok_or_else(|| RepositoryError::NotFoundRole)?;
        self.role_repo.find_users_by_role_id(role_id).await
    }

    /// 为角色添加用户
    ///
    /// # 参数
    /// * `role_id` - 角色 ID
    /// * `user_ids` - 要添加的用户 ID 列表
    ///
    /// # 执行逻辑
    /// 1. 根据角色 ID 查询角色，若不存在则返回未找到错误
    /// 2. 校验角色状态必须为启用（status == 0），否则返回角色已禁用错误
    /// 3. 遍历用户 ID 列表，逐个校验用户必须存在且状态为 Active
    /// 4. 调用角色仓库绑定角色与用户的关联关系
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundRole` - 指定角色不存在
    /// - `ValidationRoleDisabled` - 角色已禁用，无法添加用户
    /// - `NotFoundUser` - 指定用户不存在
    /// - `ValidationUserStatus` - 用户状态非 Active
    /// - 数据库操作失败时返回错误
    pub async fn add_users_to_role(&self, role_id: u64, user_ids: Vec<u64>) -> AppResult<()> {
        let role = self.role_repo.find_by_id(role_id).await?.ok_or_else(|| RepositoryError::NotFoundRole)?;

        // 角色必须为启用状态才能添加用户
        if role.status != 0 {
            return Err(RepositoryError::ValidationRoleDisabled)?;
        }

        // 校验每个用户存在且为 Active 状态
        for &uid in &user_ids {
            if let Some(user) = self.user_repo.find_by_id(uid).await? {
                if user.status != UserStatus::Active {
                    return Err(RepositoryError::ValidationUserStatus)?;
                }
            } else {
                return Err(RepositoryError::NotFoundUser)?;
            }
        }

        self.role_repo.bind_users(role_id, &user_ids).await
    }

    /// 获取角色关联的菜单 ID 列表
    pub async fn get_menu_ids(&self, role_id: u64) -> AppResult<Vec<u64>> {
        self.role_repo.get_menu_ids(role_id).await
    }

    /// 从角色移除用户
    ///
    /// # 参数
    /// * `role_id` - 角色 ID
    /// * `user_ids` - 要移除的用户 ID 列表
    ///
    /// # 执行逻辑
    /// 1. 根据角色 ID 查询角色，若不存在则返回未找到错误
    /// 2. 调用角色仓库解绑角色与用户的关联关系
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundRole` - 指定角色不存在
    /// - 数据库操作失败时返回错误
    pub async fn remove_users_from_role(&self, role_id: u64, user_ids: Vec<u64>) -> AppResult<()> {
        // Verify role exists
        let _role = self.role_repo.find_by_id(role_id).await?.ok_or_else(|| RepositoryError::NotFoundRole)?;
        self.role_repo.unbind_users(role_id, &user_ids).await
    }
}

#[cfg(test)]
mod tests;
