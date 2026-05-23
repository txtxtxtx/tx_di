//! 设备分组成员模型（多对多关联表）

use toasty::Model;

/// 设备分组成员
///
/// 关联 `gb_device_group` 和 `gb_device`，表示某设备属于某分组。
/// 一个设备可属于多个分组，一个分组可包含多个设备。
#[derive(Debug, Clone, Model)]
#[table = "gb_device_group_member"]
pub struct GbDeviceGroupMember {
    #[key]
    #[auto]
    pub id: i64,

    /// 分组 ID
    #[index]
    pub group_id: i64,

    /// 设备国标编码（关联 gb_device.device_id）
    #[index]
    pub device_id: String,

    /// 加入时间（注意：非 created_at/updated_at 字段名不能用 #[auto]）
    #[default(jiff::Timestamp::now())]
    pub joined_at: jiff::Timestamp,
}
