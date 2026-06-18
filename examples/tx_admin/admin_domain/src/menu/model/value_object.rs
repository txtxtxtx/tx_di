use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Menu query filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// 菜单查询结构体，用于封装菜单查询相关的参数
///
/// 该结构体包含三个可选字段，用于灵活构建查询条件
pub struct MenuQuery {
    /// 菜单名称，可选参数。用于按名称筛选菜单
    pub name: Option<String>,
    /// 菜单状态，可选参数。用于按状态筛选菜单，通常表示菜单的启用/禁用状态
    pub status: Option<i32>,
    /// 菜单类型，可选参数。用于按类型筛选菜单，如目录、菜单、按钮等
    pub types: Option<i32>,
}

/// Menu tree node for display
/// 菜单树节点结构体，用于表示系统中的菜单节点
///
/// 该结构体实现了 Debug、Clone、Serialize 和 Deserialize 特性，
/// 支持调试输出、克隆、序列化和反序列化操作
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MenuTreeNode {
    /// 菜单节点的唯一标识符
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub id: u64,
    /// 菜单节点的名称
    pub name: String,
    /// 访问该菜单所需的权限标识
    pub permission: String,
    /// 菜单类型，通常用于区分不同的菜单类别
    pub types: i32,
    /// 菜单的排序顺序，数值越小排序越靠前
    pub sort: i32,
    /// 父级菜单节点的ID，根节点的parent_id通常为0
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub parent_id: u64,
    /// 菜单的路由路径，前端路由使用
    pub path: Option<String>,
    /// 菜单图标，可选字段
    pub icon: Option<String>,
    /// 前端组件路径，可选字段
    pub component: Option<String>,
    /// 前端组件名称，可选字段
    pub component_name: Option<String>,
    /// 菜单状态，通常表示启用(1)或禁用(0)
    pub status: i32,
    /// 是否可见，通常表示显示(1)或隐藏(0)
    pub visible: i32,
    /// 是否保持缓存，通常表示是(1)或否(0)
    pub keep_alive: i32,
    /// 子菜单节点列表，用于构建树形结构
    pub children: Vec<MenuTreeNode>,
}
