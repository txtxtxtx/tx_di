//! 数据权限领域模型
//!
//! 参考 RuoYi-Vue-Pro 的数据权限设计，提供五种数据范围。
//!
//! # 数据范围说明
//!
//! | 范围 | variant | 说明 |
//! |------|---------|------|
//! | `All` | 1 | 全部数据权限 |
//! | `Custom` | 2 | 自定数据权限（指定部门） |
//! | `Dept` | 3 | 本部门数据权限 |
//! | `DeptAndChild` | 4 | 本部门及以下数据权限 |
//! | `Self_` | 5 | 仅本人数据权限 |

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Display;

/// 数据权限范围（toasty 枚举，存储为整数判别值）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum DataScope {
    /// 全部数据权限
    #[column(variant = 1)]
    All,
    /// 自定数据权限（指定部门）
    #[column(variant = 2)]
    Custom,
    /// 本部门数据权限
    #[column(variant = 3)]
    Dept,
    /// 本部门及以下数据权限
    #[column(variant = 4)]
    DeptAndChild,
    /// 仅本人数据权限
    #[column(variant = 5)]
    Self_,
}

impl Display for DataScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataScope::All => write!(f, "all"),
            DataScope::Custom => write!(f, "custom"),
            DataScope::Dept => write!(f, "dept"),
            DataScope::DeptAndChild => write!(f, "dept_and_child"),
            DataScope::Self_ => write!(f, "self"),
        }
    }
}

// ─── 数据权限上下文（值对象）───────────────────────────────

/// 数据权限上下文
///
/// 承载当前用户的数据权限信息，用于在查询时动态过滤数据。
/// 这是一个值对象，不可变，由领域服务计算生成。
#[derive(Debug, Clone)]
pub struct DataPermissionContext {
    /// 数据范围
    pub scope: DataScope,
    /// 当前用户 ID
    pub user_id: u64,
    /// 当前用户所属部门 ID（可选）
    pub dept_id: Option<u64>,
    /// 所有子部门 ID 集合（scope 为 DeptAndChild 时使用）
    pub child_dept_ids: HashSet<u64>,
    /// 自定义部门 ID 集合（scope 为 Custom 时使用）
    pub custom_dept_ids: HashSet<u64>,
}

impl DataPermissionContext {
    /// 创建新的数据权限上下文
    pub fn new(scope: DataScope, user_id: u64, dept_id: Option<u64>) -> Self {
        Self {
            scope,
            user_id,
            dept_id,
            child_dept_ids: HashSet::new(),
            custom_dept_ids: HashSet::new(),
        }
    }

    /// 判断是否拥有全部数据权限（超级管理员）
    pub fn can_access_all(&self) -> bool {
        self.scope == DataScope::All
    }

    /// 判断是否可以访问指定用户的数据
    pub fn can_access_user(&self, target_user_id: u64) -> bool {
        self.scope == DataScope::All || self.user_id == target_user_id
    }

    /// 判断是否可以访问指定部门的数据
    pub fn can_access_dept(&self, dept_id: u64) -> bool {
        match self.scope {
            DataScope::All => true,
            DataScope::DeptAndChild => {
                self.dept_id.map_or(false, |id| id == dept_id)
                    || self.child_dept_ids.contains(&dept_id)
            }
            DataScope::Dept => self.dept_id.map_or(false, |id| id == dept_id),
            DataScope::Custom => self.custom_dept_ids.contains(&dept_id),
            DataScope::Self_ => false,
        }
    }
}

// ─── 数据权限领域服务 ──────────────────────────────────────

/// 数据权限领域服务
///
/// 职责：根据用户角色计算最终的数据权限上下文。
/// 合并用户所有角色的数据权限范围，取最宽松的。
pub struct DataPermissionService;

impl DataPermissionService {
    /// 计算用户的数据权限上下文
    pub fn compute_context(
        user_id: u64,
        roles: &[super::role::Role],
        dept_id: Option<u64>,
    ) -> DataPermissionContext {
        let scope = Self::compute_max_scope(roles);
        let mut ctx = DataPermissionContext::new(scope, user_id, dept_id);

        for role in roles {
            if role.data_scope == DataScope::Custom {
                for &dept in &role.data_scope_dept_ids {
                    ctx.custom_dept_ids.insert(dept);
                }
            }
            if role.data_scope == DataScope::DeptAndChild {
                // 子部门 ID 应从部门服务获取，这里先留空
            }
        }

        ctx
    }

    /// 取最宽松的数据范围
    ///
    /// All > DeptAndChild > Dept > Custom > Self
    fn compute_max_scope(roles: &[super::role::Role]) -> DataScope {
        let mut max_scope = DataScope::Self_;
        for role in roles {
            max_scope = Self::max_scope(max_scope, role.data_scope);
            if max_scope == DataScope::All {
                return max_scope;
            }
        }
        max_scope
    }

    fn max_scope(a: DataScope, b: DataScope) -> DataScope {
        fn scope_value(s: DataScope) -> u8 {
            match s {
                DataScope::All => 5,
                DataScope::DeptAndChild => 4,
                DataScope::Dept => 3,
                DataScope::Custom => 2,
                DataScope::Self_ => 1,
            }
        }
        if scope_value(a) >= scope_value(b) {
            a
        } else {
            b
        }
    }
}
