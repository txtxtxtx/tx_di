//! 角色应用服务

use std::sync::Arc;
use tx_di_core::tx_comp;

use crate::domain::data_permission::DataScope;
use crate::domain::dept::DeptId;
use crate::domain::role::{Role, RoleId, RoleStatus};
use crate::domain::tenant::TenantId;
use crate::infrastructure::persistence::{InMemoryRoleRepository, InMemoryPermissionRepository};

/// 创建角色请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub code: String,
    pub remark: Option<String>,
    pub sort: Option<i32>,
    pub permission_ids: Option<Vec<String>>,
    pub data_scope: Option<DataScope>,
    pub data_scope_dept_ids: Option<Vec<DeptId>>,
}

/// 更新角色请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub remark: Option<String>,
    pub sort: Option<i32>,
    pub status: Option<RoleStatus>,
    pub permission_ids: Option<Vec<String>>,
    pub data_scope: Option<DataScope>,
    pub data_scope_dept_ids: Option<Vec<DeptId>>,
}

/// 角色应用服务
#[derive(Debug)]
#[tx_comp]
pub struct RoleService {
    pub role_repo: Arc<InMemoryRoleRepository>,
    pub perm_repo: Arc<InMemoryPermissionRepository>,
}

impl RoleService {
    /// 查询角色列表
    pub async fn list_roles(
        &self,
        tenant_id: TenantId,
        keyword: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Role>, u64), anyhow::Error> {
        self.role_repo.find_page(&tenant_id, keyword, page, page_size).await
    }

    /// 获取租户所有角色（不分页）
    pub async fn all_roles(&self, tenant_id: TenantId) -> Result<Vec<Role>, anyhow::Error> {
        self.role_repo.find_by_tenant(&tenant_id).await
    }

    /// 创建角色
    pub async fn create_role(
        &self,
        tenant_id: TenantId,
        req: CreateRoleRequest,
    ) -> Result<Role, anyhow::Error> {
        // 检查编码是否已存在
        if self.role_repo.find_by_code(&req.code).await?.is_some() {
            return Err(anyhow::anyhow!("角色编码已存在"));
        }

        // 生成简单递增 ID（内存实现，生产环境应使用数据库自增）
        let role_id = {
            let store = self.role_repo.read_store();
            store.keys().max().map_or(1, |m| m + 1)
        };
        let mut role = Role::new(
            role_id,
            tenant_id,
            req.name,
            req.code,
            req.sort.unwrap_or(0),
        );
        role.remark = req.remark;

        if let Some(perm_ids) = req.permission_ids {
            role.assign_permissions(perm_ids);
        }
        if let Some(scope) = req.data_scope {
            role.set_data_scope(scope, req.data_scope_dept_ids.unwrap_or_default());
        }

        self.role_repo.save(&role).await?;
        tracing::info!(role_id = %role.id, role_name = %role.name, "角色已创建");
        Ok(role)
    }

    /// 更新角色
    pub async fn update_role(
        &self,
        role_id: RoleId,
        req: UpdateRoleRequest,
    ) -> Result<Role, anyhow::Error> {
        let mut role = self
            .role_repo
            .find_by_id(&role_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("角色不存在"))?;

        if let Some(name) = req.name {
            role.name = name;
        }
        if let Some(remark) = req.remark {
            role.remark = Some(remark);
        }
        if let Some(sort) = req.sort {
            role.sort = sort;
        }
        if let Some(status) = req.status {
            role.status = status;
        }
        if let Some(perm_ids) = req.permission_ids {
            role.assign_permissions(perm_ids);
        }
        if let Some(scope) = req.data_scope {
            role.set_data_scope(scope, req.data_scope_dept_ids.unwrap_or_default());
        }

        self.role_repo.save(&role).await?;
        Ok(role)
    }

    /// 删除角色
    pub async fn delete_role(&self, role_id: RoleId) -> Result<(), anyhow::Error> {
        self.role_repo
            .find_by_id(&role_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("角色不存在"))?;

        self.role_repo.delete(&role_id).await?;
        tracing::info!(role_id = %role_id, "角色已删除");
        Ok(())
    }
}
