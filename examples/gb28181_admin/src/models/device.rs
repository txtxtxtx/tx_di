//! 设备模型 — GB28181 设备持久化

use toasty::Model;

/// GB28181 设备注册记录
#[derive(Debug, Clone, Model)]
#[table = "gb_device"]
pub struct GbDeviceRecord {
    /// 主键 ID（自增）
    #[key]
    #[auto]
    pub id: u64,

    /// 设备国标编码（20位）
    #[unique]
    pub device_id: String,

    /// SIP 联系人地址
    #[default("".to_string())]
    pub contact: String,

    /// 远端 IP:Port
    #[default("".to_string())]
    pub remote_addr: String,

    /// 是否在线
    #[default(false)]
    pub online: bool,

    /// 厂商
    #[default("".to_string())]
    pub manufacturer: String,

    /// 型号
    #[default("".to_string())]
    pub model: String,

    /// 固件版本
    #[default("".to_string())]
    pub firmware: String,

    /// 通道数量
    #[default(0)]
    pub channel_count: i32,

    /// SIP 注册有效期（秒）
    #[default(3600)]
    pub expires: i64,

    /// 认证用户名
    #[default("".to_string())]
    pub auth_username: String,

    /// 注册时间
    #[default(jiff::Timestamp::now())]
    pub registered_at: jiff::Timestamp,

    /// 最后心跳时间
    #[update(jiff::Timestamp::now())]
    pub last_heartbeat_at: jiff::Timestamp,

    /// 更新时间
    #[update(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,

    /// 所属分组 ID（0 = 未分组）
    #[default(0)]
    pub group_id: u64,
}
