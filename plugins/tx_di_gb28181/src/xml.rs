//! GB28181 XML 工具函数
//!
//! 构建和解析 MANSCDP XML 消息（不引入重型 XML 库，直接字符串操作）

// ── 解析 ─────────────────────────────────────────────────────────────────────

/// 从 GB28181 XML 中提取指定字段值
///
/// # 示例
/// ```
/// let xml = "<Notify><CmdType>Keepalive</CmdType><SN>1</SN></Notify>";
/// assert_eq!(parse_xml_field(xml, "CmdType"), Some("Keepalive".to_string()));
/// ```
pub fn parse_xml_field(xml: &str, field: &str) -> Option<String> {
    let open = format!("<{}>", field);
    let close = format!("</{}>", field);
    let start = xml.find(&open)? + open.len();
    let end = xml.find(&close)?;
    if start <= end {
        Some(xml[start..end].trim().to_string())
    } else {
        None
    }
}

/// 从 XML 中解析 SN（消息序号）
pub fn parse_sn(xml: &str) -> u32 {
    parse_xml_field(xml, "SN")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

// ── 构建查询 XML ─────────────────────────────────────────────────────────────

/// 构建目录查询 MESSAGE body（平台 → 设备）
pub fn build_catalog_query_xml(platform_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
         <Query>\r\n\
         <CmdType>Catalog</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{platform_id}</DeviceID>\r\n\
         </Query>",
        sn = sn,
        platform_id = platform_id
    )
}

/// 构建设备信息查询 MESSAGE body
pub fn build_device_info_query_xml(_platform_id: &str, device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
         <Query>\r\n\
         <CmdType>DeviceInfo</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id
    )
}

/// 构建心跳 Keepalive XML（设备 → 平台）
pub fn build_keepalive_xml(device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
         <Notify>\r\n\
         <CmdType>Keepalive</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <Status>OK</Status>\r\n\
         </Notify>",
        sn = sn,
        device_id = device_id
    )
}

/// 构建设备目录响应 XML（设备 → 平台）
///
/// 用于设备响应平台的 Catalog 查询
pub fn build_catalog_response_xml(
    device_id: &str,
    sn: u32,
    channels: &[(String, String)], // (channel_id, name)
) -> String {
    let channel_count = channels.len();
    let items: String = channels
        .iter()
        .map(|(ch_id, name)| {
            format!(
                "<Item>\r\n\
                 <DeviceID>{ch_id}</DeviceID>\r\n\
                 <Name>{name}</Name>\r\n\
                 <Manufacturer>Simulator</Manufacturer>\r\n\
                 <Model>IPC-V1</Model>\r\n\
                 <Status>ON</Status>\r\n\
                 <Parental>0</Parental>\r\n\
                 <ParentID>{device_id}</ParentID>\r\n\
                 <SafetyWay>0</SafetyWay>\r\n\
                 <RegisterWay>1</RegisterWay>\r\n\
                 <Secrecy>0</Secrecy>\r\n\
                 </Item>",
                ch_id = ch_id,
                name = name,
                device_id = device_id
            )
        })
        .collect::<Vec<_>>()
        .join("\r\n");

    format!(
        "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
         <Response>\r\n\
         <CmdType>Catalog</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <SumNum>{channel_count}</SumNum>\r\n\
         <DeviceList Num=\"{channel_count}\">\r\n\
         {items}\r\n\
         </DeviceList>\r\n\
         </Response>",
        sn = sn,
        device_id = device_id,
        channel_count = channel_count,
        items = items
    )
}

/// 解析目录响应中的通道列表
///
/// 返回 `Vec<(channel_id, name, status)>`
pub fn parse_catalog_items(xml: &str) -> Vec<(String, String, String)> {
    let mut result = Vec::new();
    // 简单提取 <Item>...</Item> 块
    let mut rest = xml;
    while let Some(start) = rest.find("<Item>") {
        let after = &rest[start + 6..];
        if let Some(end) = after.find("</Item>") {
            let item_xml = &after[..end];
            let ch_id = parse_xml_field(item_xml, "DeviceID").unwrap_or_default();
            let name = parse_xml_field(item_xml, "Name").unwrap_or_default();
            let status = parse_xml_field(item_xml, "Status").unwrap_or_else(|| "Unknown".into());
            if !ch_id.is_empty() {
                result.push((ch_id, name, status));
            }
            rest = &after[end + 7..];
        } else {
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xml_field() {
        let xml = "<Notify><CmdType>Keepalive</CmdType><SN>42</SN></Notify>";
        assert_eq!(parse_xml_field(xml, "CmdType"), Some("Keepalive".to_string()));
        assert_eq!(parse_xml_field(xml, "SN"), Some("42".to_string()));
        assert_eq!(parse_xml_field(xml, "Missing"), None);
    }

    #[test]
    fn test_parse_sn() {
        let xml = "<Query><SN>99</SN></Query>";
        assert_eq!(parse_sn(xml), 99);
    }

    #[test]
    fn test_parse_catalog_items() {
        let xml = r#"<DeviceList>
<Item><DeviceID>ch01</DeviceID><Name>Camera1</Name><Status>ON</Status></Item>
<Item><DeviceID>ch02</DeviceID><Name>Camera2</Name><Status>OFF</Status></Item>
</DeviceList>"#;
        let items = parse_catalog_items(xml);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, "ch01");
        assert_eq!(items[1].2, "OFF");
    }
}
