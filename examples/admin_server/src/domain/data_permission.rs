//! 数据权限领域模型

use std::collections::HashSet;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum DataScope {
    #[column(variant = 0)] Self_,
    #[column(variant = 1)] Custom,
    #[column(variant = 2)] Dept,
    #[column(variant = 3)] DeptAndChild,
    #[column(variant = 4)] All,
}
impl Display for DataScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { DataScope::All => write!(f, "all"), DataScope::Custom => write!(f, "custom"), DataScope::Dept => write!(f, "dept"), DataScope::DeptAndChild => write!(f, "dept_and_child"), DataScope::Self_ => write!(f, "self") }
    }
}

#[derive(Debug, Clone)]
pub struct DataPermissionContext { pub scope: DataScope, pub user_id: u64, pub dept_id: Option<u64>, pub child_dept_ids: HashSet<u64>, pub custom_dept_ids: HashSet<u64> }
impl DataPermissionContext {
    pub fn new(scope: DataScope, user_id: u64, dept_id: Option<u64>) -> Self { Self { scope, user_id, dept_id, child_dept_ids: HashSet::new(), custom_dept_ids: HashSet::new() } }
    pub fn can_access_all(&self) -> bool { self.scope == DataScope::All }
    pub fn can_access_dept(&self, dept_id: u64) -> bool { match self.scope { DataScope::All => true, DataScope::DeptAndChild => self.dept_id.map_or(false, |id| id == dept_id) || self.child_dept_ids.contains(&dept_id), DataScope::Dept => self.dept_id.map_or(false, |id| id == dept_id), DataScope::Custom => self.custom_dept_ids.contains(&dept_id), DataScope::Self_ => false } }
}

pub struct DataPermissionService;
impl DataPermissionService {
    pub fn compute_context(user_id: u64, roles: &[super::role::Role], dept_id: Option<u64>) -> DataPermissionContext {
        let scope = Self::compute_max_scope(roles);
        let mut ctx = DataPermissionContext::new(scope, user_id, dept_id);
        for role in roles { if role.data_scope == DataScope::Custom { for &dept in &role.data_scope_dept_ids { ctx.custom_dept_ids.insert(dept); } } }
        ctx
    }
    fn compute_max_scope(roles: &[super::role::Role]) -> DataScope { let mut max = DataScope::Self_; for role in roles { max = Self::max_scope(max, role.data_scope); if max == DataScope::All { return max; } } max }
    fn max_scope(a: DataScope, b: DataScope) -> DataScope { fn v(s: DataScope) -> u8 { match s { DataScope::All => 5, DataScope::DeptAndChild => 4, DataScope::Dept => 3, DataScope::Custom => 2, DataScope::Self_ => 1 } } if v(a) >= v(b) { a } else { b } }
}
