use crate::error::{AppError, AppResult};
use crate::model::gqb200a7u::GQB200A7UModel;
use crate::protocol::base::BaseMessage;
use crate::util::{convert, ieee754};

/// GQB200A7U协议解析器
pub struct GQB200A7UParser;

impl GQB200A7UParser {
    /// 解析GQB200A7U消息
    pub fn parse(message: &str) -> AppResult<GQB200A7UModel> {
        // 解析基础消息
        let base = BaseMessage::from_hex(message)?;

        // 解析数据部分
        let data = &base.data;
        if data.len() < 52 {
            return Err(AppError::Protocol("GQB200A7U数据长度不足".to_string()));
        }

        // 解析传感器数据 (每路2字节，共8字节)
        let sensor1_hex = &data[0..4];
        let sensor2_hex = &data[4..8];
        let sensor3_hex = &data[8..12];
        let sensor4_hex = &data[12..16];

        // 解析GPS数据 (8字节)
        let gps_hex = &data[16..32];

        // 解析高度数据 (2字节)
        let _altitude_hex = &data[32..36];

        // 解析气压数据 (2字节)
        let _pressure_hex = &data[36..40];

        // 解析温度数据 (4字节)
        let _temperature_hex = &data[40..48];

        // 解析报警数据 (2字节)
        let alarm_hex = &data[48..52];

        // 转换传感器值
        let sensor1_val = convert::get_gas_decimal(sensor1_hex);
        let sensor2_val = convert::get_gas_decimal(sensor2_hex);
        let sensor3_val = convert::get_gas_decimal(sensor3_hex);
        let sensor4_val = convert::get_gas_decimal(sensor4_hex);

        // 转换GPS坐标
        let (longitude, latitude) = ieee754::extract_gps_coordinates(gps_hex);

        // 转换RSSI
        let rssi_description = base.get_rssi_description();

        // 解析报警数据
        let alarm = convert::hex_to_alarm_level_array(&alarm_hex[0..2]);
        let alarm_sp = convert::hex_to_alarm_sp_array(&alarm_hex[2..4]);

        // 生成报警描述
        let alarm_level_descriptions = generate_alarm_level_descriptions(&alarm);
        let alarm_sp_descriptions = generate_alarm_sp_descriptions(&alarm_sp);

        // 处理小数位情况
        let sensor_dot_arr = [0, 0, 0, 1]; // 第四路传感器有1位小数
        let sensor1_decimal = convert::format_decimal_value(sensor1_val, Some(sensor_dot_arr[0]));
        let sensor2_decimal = convert::format_decimal_value(sensor2_val, Some(sensor_dot_arr[1]));
        let sensor3_decimal = convert::format_decimal_value(sensor3_val, Some(sensor_dot_arr[2]));
        let sensor4_decimal = convert::format_decimal_value(sensor4_val, Some(sensor_dot_arr[3]));

        Ok(GQB200A7UModel {
            device_model: "GQB200A7U".to_string(),
            device_code: base.get_uuid_string(),
            rssi: rssi_description,
            sensor1: sensor1_decimal,
            sensor2: sensor2_decimal,
            sensor3: sensor3_decimal,
            sensor4: sensor4_decimal,
            lng: longitude.to_string(),
            lat: latitude.to_string(),
            alarm,
            level: alarm_level_descriptions,
            alarm_sp: alarm_sp_descriptions,
        })
    }
}

/// 生成报警级别描述
fn generate_alarm_level_descriptions(alarm: &[u8]) -> Vec<String> {
    let channel_names = ["通道1", "通道2", "通道3", "通道4"];
    let level_names = ["", "一级报警", "二级报警", "三级报警"];

    let mut descriptions = Vec::new();
    let mut channel_index = 0;

    for &level in alarm {
        if level > 0 && channel_index < channel_names.len() && (level as usize) < level_names.len() {
            let channel = channel_names[channel_index];
            let level_desc = level_names[level as usize];
            descriptions.push(format!("{}{}", channel, level_desc));
        }
        channel_index += 1;
    }

    descriptions
}

/// 生成特殊报警描述
fn generate_alarm_sp_descriptions(alarm_sp: &[usize]) -> Vec<String> {
    let sp_names = [
        "",
        "",
        "",
        "",
        "",
        "",
        "气泵二级报警",
        "气泵一级报警",
    ];

    alarm_sp
        .iter()
        .filter(|&&index| index < sp_names.len() && !sp_names[index].is_empty())
        .map(|&index| sp_names[index].to_string())
        .collect()
}

/// 解析GQB200A7U消息的便捷函数
pub fn parse(message: &str) -> AppResult<GQB200A7UModel> {
    GQB200A7UParser::parse(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gqb200a7u() {
        // 测试报文 (需要根据实际数据调整)
        // let hex = "...";
        // let result = GQB200A7UParser::parse(hex);
        // assert!(result.is_ok());
    }

    #[test]
    fn test_generate_alarm_level_descriptions() {
        let alarm = vec![1, 0, 2, 0];
        let descriptions = generate_alarm_level_descriptions(&alarm);
        assert_eq!(descriptions, vec!["通道1一级报警", "通道3二级报警"]);
    }

    #[test]
    fn test_generate_alarm_sp_descriptions() {
        let alarm_sp = vec![6, 7];
        let descriptions = generate_alarm_sp_descriptions(&alarm_sp);
        assert_eq!(descriptions, vec!["气泵二级报警", "气泵一级报警"]);
    }
}