//! 设备注册审核模型
//!
//! 新设备首次 REGISTER 时，若开启了「注册审核」模式，
//! 则不会立即接受注册，而是写入待审核记录，
//! 由管理员批准/拒绝后，设备才能上线。

use toasty::Model;
use serde::{Serialize, Deserialize};

/// 注册审核状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditStatus {
    /// 待审核
    Pending,
    /// 已批准
    Approved,
    /// 已拒绝
    Rejected,
}

impl ToString for AuditStatus {
    fn to_string(&self) -> String {
        match self {
            AuditStatus::Pending => "pending".to_string(),
            AuditStatus::Approved => "approved".to_string(),
            AuditStatus::Rejected => "rejected".to_string(),
        }
    }
}

impl From<String> for AuditStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "approved" => AuditStatus::Approved,
            "rejected" => AuditStatus::Rejected,
            _ => AuditStatus::Pending,
        }
    }
}

/// 设备注册审核记录
#[derive(Debug, Clone, Model)]
#[table = "gb_register_audit"]
pub struct GbRegisterAudit {
    #[key]
    #[auto]
    pub id: u64,

    /// 设备国标编码（20位）
    #[index]
    pub device_id: String,

    /// 设备上报的 SIP 联系人地址
    #[default("".to_string())]
    pub contact: String,

    /// 设备远端 IP
    #[default("".to_string())]
    pub remote_ip: String,

    /// 设备厂商（从 REGISTER 消息提取）
    #[default("".to_string())]
    pub manufacturer: String,

    /// 设备型号
    #[default("".to_string())]
    pub model: String,

    /// 设备固件版本
    #[default("".to_string())]
    pub firmware: String,

    /// 审核状态：pending / approved / rejected
    #[index]
    #[default("pending".to_string())]
    pub status: String,

    /// 审核人
    #[default("".to_string())]
    pub auditor: String,

    /// 审核备注
    #[default("".to_string())]
    pub audit_remark: String,

    /// 审核时间
    pub audited_at: Option<jiff::Timestamp>,

    /// 申请说明（设备端可附带）
    #[default("".to_string())]
    pub apply_remark: String,

    /// 创建时间（申请时间）
    #[auto]
    pub created_at: jiff::Timestamp,

    /// 更新时间
    #[update(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
}
