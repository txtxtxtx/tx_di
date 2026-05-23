//! 设备分组模型 — 虚拟组织/业务分组
//!
//! 支持树形分组（父子关系），设备通过关联表加入分组。

use toasty::Model;

/// 设备分组
///
/// 树形结构：`parent_id = 0` 表示根分组。
#[derive(Debug, Clone, Model)]
#[table = "gb_device_group"]
pub struct GbDeviceGroup {
    #[key]
    #[auto]
    pub id: i64,

    /// 分组名称
    #[index]
    pub name: String,

    /// 父分组 ID（0 = 根分组）
    #[default(0)]
    #[index]
    pub parent_id: i64,

    /// 分组描述
    #[default("".to_string())]
    pub description: String,

    /// 排序权重（越小越靠前）
    #[default(0)]
    pub sort_order: i32,

    /// 创建人
    #[default("".to_string())]
    pub created_by: String,

    /// 创建时间
    #[auto]
    pub created_at: jiff::Timestamp,

    /// 更新时间
    #[update(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
}
