use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeptQuery {
    pub name: Option<String>,
    pub status: Option<i32>,
}

/// 部门树节点
///
/// u64 字段（id/parent_id/leader_user_id）通过 `serde_as` 序列化为字符串，
/// 避免前端 JavaScript 数值精度丢失（雪花 ID 超过 2^53）。
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeptTreeNode {
    /// 部门 ID
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub id: u64,
    /// 部门名称
    pub name: String,
    /// 上级部门 ID，0 表示顶级部门
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub parent_id: u64,
    /// 排序号（越小越靠前）
    pub sort: i32,
    /// 负责人用户 ID
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    pub leader_user_id: Option<u64>,
    /// 状态：0=停用, 1=正常
    pub status: i32,
    /// 子部门列表
    pub children: Vec<DeptTreeNode>,
}
