pub mod base;
pub mod gqb200a7u;
pub mod nano4sp;

use crate::model::gqb200a7u::GQB200A7UModel;
use crate::model::nano4sp::Nano4SPModel;
use crate::util::convert;
use tx_di_core::tx_comp;

/// 设备类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    Nano4SP,
    GQB200A7U,
    Unknown(String),
}

impl DeviceType {
    /// 从模板ID解析设备类型
    pub fn from_template_id(template_id: &str) -> Self {
        match template_id {
            "04_23_01" => DeviceType::Nano4SP,
            "07_1D_00" => DeviceType::GQB200A7U,
            other => DeviceType::Unknown(other.to_string()),
        }
    }
}

/// 解析后的设备数据
#[derive(Debug, Clone)]
pub enum DeviceData {
    Nano4SP(Nano4SPModel),
    GQB200A7U(GQB200A7UModel),
}

/// 协议解析器（无状态组件）
///
/// 负责解析设备上报的私有协议数据。
/// 支持 Nano4SP 和 GQB200A7U 两种设备类型。
#[tx_comp]
pub struct ProtocolParser;

impl ProtocolParser {
    /// 解析消息
    ///
    /// # Arguments
    /// * `message` - 十六进制格式的消息字符串
    ///
    /// # Returns
    /// 解析后的设备数据
    pub fn parse(message: &str) -> anyhow::Result<DeviceData> {
        // 验证消息
        if !convert::verify_message(message) {
            return Err(anyhow::anyhow!("消息验证失败"));
        }

        // 获取设备类型
        let template_id = convert::get_template_id(message)?;
        let device_type = DeviceType::from_template_id(&template_id);

        match device_type {
            DeviceType::Nano4SP => {
                let model = nano4sp::parse(message)?;
                Ok(DeviceData::Nano4SP(model))
            }
            DeviceType::GQB200A7U => {
                let model = gqb200a7u::parse(message)?;
                Ok(DeviceData::GQB200A7U(model))
            }
            DeviceType::Unknown(id) => Err(anyhow::anyhow!("未知设备类型: {}", id)),
        }
    }
}
