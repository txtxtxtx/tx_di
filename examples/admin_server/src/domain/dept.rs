//! 部门聚合
//!
//! 部门是组织架构的核心实体，支持树形层级结构。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use toasty::Model;

// ─── 枚举定义 ──────────────────────────────────────────────

/// 通用状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum CommonStatus {
    /// 启用
    #[column(variant = 0)]
    Enable,
    /// 停用
    #[column(variant = 1)]
    Disable,
}

impl CommonStatus {
    pub fn is_enable(&self) -> bool {
        matches!(self, CommonStatus::Enable)
    }
}

// ─── 部门实体 ──────────────────────────────────────────────

/// 部门实体
///
/// 职责：
/// - 定义组织架构树结构
/// - 管理部门负责人
/// - 与数据权限联动
#[derive(Debug, Clone, Model)]
#[table = "system_dept"]
pub struct Dept {
    /// 部门 ID
    #[key]
    #[auto]
    pub id: u64,

    /// 所属租户 ID
    pub tenant_id: u64,

    /// 部门名称
    pub name: String,

    /// 父部门 ID（0 表示根部门）
    #[default(0u64)]
    pub parent_id: u64,

    /// 显示顺序
    #[default(0i32)]
    pub sort: i32,

    /// 负责人用户 ID
    pub leader_user_id: Option<u64>,

    /// 联系电话
    pub phone: Option<String>,

    /// 邮箱
    pub email: Option<String>,

    /// 部门状态
    pub status: CommonStatus,

    /// 创建者
    pub creator: Option<String>,

    /// 更新者
    pub updater: Option<String>,

    /// 创建时间
    #[auto]
    pub created_at: jiff::Timestamp,

    /// 更新时间
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,

    /// 软删除标记
    #[default(0u8)]
    pub deleted: u8,
}

// ─── 领域行为 ──────────────────────────────────────────────

impl Dept {
    /// 创建新部门
    pub fn new(tenant_id: u64, name: String, parent_id: u64) -> Self {
        Self {
            id: 0,
            tenant_id,
            name,
            parent_id,
            sort: 0,
            leader_user_id: None,
            phone: None,
            email: None,
            status: CommonStatus::Enable,
            creator: None,
            updater: None,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }

    /// 判断是否为根部门
    pub fn is_root(&self) -> bool {
        self.parent_id == 0
    }

    /// 是否可用
    pub fn is_active(&self) -> bool {
        self.status.is_enable() && self.deleted == 0
    }

    /// 更新部门信息
    pub fn update_info(
        &mut self,
        name: Option<String>,
        sort: Option<i32>,
        leader_user_id: Option<u64>,
        phone: Option<String>,
        email: Option<String>,
    ) {
        if let Some(n) = name {
            self.name = n;
        }
        if let Some(s) = sort {
            self.sort = s;
        }
        self.leader_user_id = leader_user_id;
        if let Some(p) = phone {
            self.phone = Some(p);
        }
        if let Some(e) = email {
            self.email = Some(e);
        }
    }

    /// 移动部门到新的父部门
    pub fn move_to(&mut self, new_parent_id: u64) {
        self.parent_id = new_parent_id;
    }

    /// 软删除
    pub fn mark_deleted(&mut self) {
        self.deleted = 1;
    }
}

// ─── 仓储 trait ──────────────────────────────────────────────

/// 部门仓储 trait
#[async_trait]
pub trait DeptRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Dept>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<Dept>, anyhow::Error>;
    async fn find_by_parent_id(&self, parent_id: u64, tenant_id: u64) -> Result<Vec<Dept>, anyhow::Error>;
    async fn save(&self, dept: &Dept) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
