//! 权限应用服务

use std::sync::Arc;
use tx_di_core::tx_comp;

use crate::domain::permission::{Permission, PermissionId, PermissionType};
use crate::infrastructure::persistence::InMemoryPermissionRepository;

/// 权限树节点（用于前端菜单渲染）
#[derive(Debug, Clone, serde::Serialize)]
pub struct PermissionTreeNode {
    pub id: PermissionId,
    pub parent_id: Option<PermissionId>,
    pub name: String,
    pub code: String,
    pub perm_type: PermissionType,
    pub path: Option<String>,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub sort: i32,
    pub visible: bool,
    pub children: Vec<PermissionTreeNode>,
}

/// 权限应用服务
#[derive(Debug)]
#[tx_comp]
pub struct PermissionService {
    pub perm_repo: Arc<InMemoryPermissionRepository>,
}

impl PermissionService {
    /// 获取权限树（菜单+目录）
    pub async fn get_menu_tree(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<Vec<PermissionTreeNode>, anyhow::Error> {
        let perms = self.perm_repo.find_menu_tree(tenant_id).await?;
        Ok(Self::build_tree(&perms, None))
    }

    /// 获取所有权限列表（含按钮和 API）
    pub async fn list_all(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<Vec<Permission>, anyhow::Error> {
        self.perm_repo.find_by_tenant(tenant_id).await
    }

    /// 获取角色权限编码列表
    pub async fn get_permission_codes(
        &self,
        role_ids: &[String],
    ) -> Result<Vec<String>, anyhow::Error> {
        let perms = self.perm_repo.find_by_role_ids(role_ids).await?;
        Ok(perms.into_iter().map(|p| p.code).collect())
    }

    /// 递归构建树结构
    fn build_tree(
        all: &[Permission],
        parent_id: Option<&PermissionId>,
    ) -> Vec<PermissionTreeNode> {
        let mut children: Vec<PermissionTreeNode> = all
            .iter()
            .filter(|p| p.parent_id.as_ref() == parent_id)
            .map(|p| {
                let child_nodes = Self::build_tree(all, Some(&p.id));
                PermissionTreeNode {
                    id: p.id.clone(),
                    parent_id: p.parent_id.clone(),
                    name: p.name.clone(),
                    code: p.code.clone(),
                    perm_type: p.perm_type,
                    path: p.path.clone(),
                    component: p.component.clone(),
                    icon: p.icon.clone(),
                    sort: p.sort,
                    visible: p.visible,
                    children: child_nodes,
                }
            })
            .collect();

        children.sort_by_key(|n| n.sort);
        children
    }
}
