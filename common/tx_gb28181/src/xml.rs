//! GB28181 XML 工具函数
//!
//! 构建和解析 MANSCDP XML 消息
//!
//! ## 性能优化
//! 使用 `quick-xml` 流式解析器，避免重复字符串分配
//!
//! ## 支持的命令类型（GB28181-2022） 2022 版本字符集换了 GB2312 -> GB18030
//! - Keepalive       — 心跳
//! - Catalog         — 目录查询/响应
//! - DeviceInfo      — 设备信息查询/响应
//! - DeviceStatus    — 设备状态查询/响应
//! - DeviceControl   — 设备控制（PTZ/巡航/看守位等）
//! - RecordInfo      — 录像查询/响应
//! - Alarm           — 报警通知/确认
//! - MediaStatus     — 媒体状态通知
//! - Broadcast       — 广播通知
//! - ConfigDownload  — 配置下载

use quick_xml::Reader;
use quick_xml::events::Event;
use crate::ChannelStatus;
// ── 解析工具 ──────────────────────────────────────────────────────────────────

/// 从 GB28181 XML 中提取指定字段值（使用 quick-xml）
///
/// # 示例
/// ```
/// # use tx_gb28181::xml::parse_xml_field;
/// let xml = "<Notify><CmdType>Keepalive</CmdType><SN>1</SN></Notify>";
/// assert_eq!(parse_xml_field(xml, "CmdType"), Some("Keepalive".to_string()));
/// ```
pub fn parse_xml_field(xml: &str, field: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let field_bytes = field.as_bytes();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == field_bytes => {
                // 找到目标标签，读取文本内容
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Text(t)) => {
                        if let Ok(s) = t.xml_content() {
                            return Some(s.trim().to_string());
                        }
                    }
                    Ok(Event::Empty(e)) if e.name().as_ref() == field_bytes => {
                        // 自闭合标签如 <Field/>
                        return Some(String::new());
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// 从 XML 中解析 SN（消息序号）
pub fn parse_sn(xml: &str) -> u32 {
    parse_xml_field(xml, "SN")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

// ── 心跳 ─────────────────────────────────────────────────────────────────────

/// 构建心跳 Keepalive XML（设备 → 平台）
pub fn build_keepalive_xml(device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
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

// ── 目录查询 ─────────────────────────────────────────────────────────────────

/// 构建目录查询 MESSAGE body（平台 → 设备）
pub fn build_catalog_query_xml(platform_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>Catalog</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{platform_id}</DeviceID>\r\n\
         </Query>",
        sn = sn,
        platform_id = platform_id
    )
}

/// 解析目录响应中的通道列表（使用 quick-xml 流式解析）
///
/// 返回 `Vec<CatalogItem>`，包含通道完整信息
pub fn parse_catalog_items(xml: &str) -> Vec<CatalogItem> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut result = Vec::new();
    let mut buf = Vec::new();
    let mut in_item = false;

    // 当前 Item 的字段
    // 国标id
    let mut cur_device_id = String::new();
    // 名称
    let mut cur_name = String::new();
    // 生产厂商
    let mut cur_manufacturer = String::new();
    // 型号
    let mut cur_model = String::new();
    // 状态
    let mut cur_status = String::new();
    // 地址
    let mut cur_address = String::new();
    // 父级国标id
    let mut cur_parent_id = String::new();
    // 父级级联
    let mut cur_parental: u8 = 0;
    // 注册方式
    let mut cur_register_way: u8 = 0;
    // 保密级别
    let mut cur_secrecy: u8 = 0;
    // IP地址
    let mut cur_ip_address = String::new();
    // 端口
    let mut cur_port: u16 = 0;
    // 经度
    let mut cur_longitude: Option<f64> = None;
    // 纬度
    let mut cur_latitude: Option<f64> = None;
    // 警区
    let mut cur_block = String::new();
    // 警区编号
    let mut cur_civil_code = String::new();
    // 通道数
    let mut cur_channel_num: u32 = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"Item" => {
                        in_item = true;
                        // 重置当前 Item 字段
                        cur_device_id.clear();
                        cur_name.clear();
                        cur_manufacturer.clear();
                        cur_model.clear();
                        cur_status.clear();
                        cur_address.clear();
                        cur_parent_id.clear();
                        cur_parental = 0;
                        cur_register_way = 0;
                        cur_secrecy = 0;
                        cur_ip_address.clear();
                        cur_port = 0;
                        cur_longitude = None;
                        cur_latitude = None;
                        cur_block.clear();
                        cur_civil_code.clear();
                        cur_channel_num = 0;
                    }
                    b"DeviceID" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_device_id = s.trim().to_string();
                        }
                    }
                    b"Name" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_name = s.trim().to_string();
                        }
                    }
                    b"Manufacturer" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_manufacturer = s.trim().to_string();
                        }
                    }
                    b"Model" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_model = s.trim().to_string();
                        }
                    }
                    b"Status" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_status = s.trim().to_string();
                        }
                    }
                    b"Address" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_address = s.trim().to_string();
                        }
                    }
                    b"ParentID" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_parent_id = s.trim().to_string();
                        }
                    }
                    b"Parental" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_parental = s.trim().parse().unwrap_or(0);
                        }
                    }
                    b"RegisterWay" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_register_way = s.trim().parse().unwrap_or(0);
                        }
                    }
                    b"Secrecy" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_secrecy = s.trim().parse().unwrap_or(0);
                        }
                    }
                    b"IPAddress" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_ip_address = s.trim().to_string();
                        }
                    }
                    b"Port" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_port = s.trim().parse().unwrap_or(0);
                        }
                    }
                    b"Longitude" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_longitude = s.trim().parse().ok();
                        }
                    }
                    b"Latitude" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_latitude = s.trim().parse().ok();
                        }
                    }
                    b"Block" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_block = s.trim().to_string();
                        }
                    }
                    b"CivilCode" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_civil_code = s.trim().to_string();
                        }
                    }
                    b"Num" => {
                        if in_item
                            && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                            && let Ok(s) = t.xml_content()
                        {
                            cur_channel_num = s.trim().parse().unwrap_or(0);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"Item" => {
                if !cur_device_id.is_empty() {
                    if cur_status.is_empty() {
                        cur_status = "Unknown".to_string();
                    }
                    result.push(CatalogItem {
                        device_id: std::mem::take(&mut cur_device_id),
                        name: std::mem::take(&mut cur_name),
                        manufacturer: std::mem::take(&mut cur_manufacturer),
                        model: std::mem::take(&mut cur_model),
                        status: std::mem::take(&mut cur_status),
                        address: std::mem::take(&mut cur_address),
                        parent_id: std::mem::take(&mut cur_parent_id),
                        parental: cur_parental,
                        register_way: cur_register_way,
                        security_level_code: None,
                        secrecy: cur_secrecy,
                        ip_address: std::mem::take(&mut cur_ip_address),
                        port: cur_port,
                        longitude: cur_longitude,
                        latitude: cur_latitude,
                        block: std::mem::take(&mut cur_block),
                        civil_code: std::mem::take(&mut cur_civil_code),
                        channel_num: cur_channel_num,
                        password: None,
                    });
                }
                in_item = false;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    result
}

/// 目录条目信息
#[derive(Debug, Clone)]
pub struct CatalogItem {
    /// - 行政区划分时可为 2、4、6、8位
    /// - 其他情况为20位
    pub device_id: String,
    /// 设备名称
    pub name: String,
    /// 制造商名称
    pub manufacturer: String,
    /// 设备型号
    pub model: String,
    /// 行政区划代码 可选 2，4，6，8位
    pub civil_code: String,
    /// 警区
    pub block: Option<String>,
    /// 设备地址信息
    pub address: String,
    /// 是否有子设备（0=无，1=有）
    pub parental: u8,
    /// 父节点ID，可多值 / 分割
    pub parent_id: String,
    /// 注册方式（1=标准，2=基于口令的双向认证注册模式，3=基于数字证书的双向认证模式（高安全级别），4=基于数字证书的单向认证模式（高安全级别））
    ///
    /// 默认 1
    pub register_way: u8,
    /// 摄像机安全能力等级代码
    /// - A-GB 35114 前端设备安全能力A级
    /// - B-GB 35114 前端设备安全能力B级
    /// - C-GB 35114 前端设备安全能力C级
    pub security_level_code: Option<String>,
    /// 保密属性（0=非保密，1=保密）
    ///
    /// 默认 0
    pub secrecy: u8,
    /// 设备/系统 ipv4/ipv6 地址
    pub ip_address: Option<String>,
    /// 设备/系统 端口
    pub port: Option<u16>,
    /// 设备口令
    pub password: Option<String>,
    /// 设备状态（ON/OFF/Unknown）
    pub status: ChannelStatus,
    /// 经度坐标，一二类监控点位必选
    pub longitude: Option<f64>,
    /// 纬度坐标，一二类监控点位必选
    pub latitude: Option<f64>,
    pub

    /// 通道数量
    pub channel_num: u32,
}

/// 构建设备目录响应 XML（设备 → 平台）
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
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
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

// ── 设备信息 ──────────────────────────────────────────────────────────────────

/// 构建设备信息查询 MESSAGE body（平台 → 设备）
pub fn build_device_info_query_xml(_platform_id: &str, device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>DeviceInfo</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id
    )
}

// ── 设备状态 ──────────────────────────────────────────────────────────────────

/// 构建设备状态查询 XML（平台 → 设备）
///
/// GB28181-2022 第 7 章：设备状态查询
pub fn build_device_status_query_xml(device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>DeviceStatus</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id
    )
}

/// 设备状态信息（解析自 DeviceStatus 响应）
#[derive(Debug, Clone, Default)]
pub struct DeviceStatus {
    pub device_id: String,
    pub result: String,
    pub on_line: String,
    pub status: String,
    pub encode: String,
    pub record: String,
    pub device_time: String,
    pub alarmstatus: Option<AlarmStatus>,
}

/// 报警状态
#[derive(Debug, Clone, Default)]
pub struct AlarmStatus {
    /// 胁迫报警
    pub duress_alarm: u8,
    /// 遮挡报警
    pub enclosure_alarm: u8,
    /// 视频丢失报警
    pub video_lost: u8,
    /// 视频移动报警
    pub video_motion: u8,
    /// 存储设备故障报警
    pub storage_fault: u8,
    /// 存储设备满报警
    pub storage_full: u8,
}

/// 解析设备状态响应（使用 quick-xml）
pub fn parse_device_status(xml: &str) -> DeviceStatus {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut s = DeviceStatus::default();
    let mut in_alarmstatus = false;
    let mut cur_duress_alarm: u8 = 0;
    let mut cur_enclosure_alarm: u8 = 0;
    let mut cur_video_lost: u8 = 0;
    let mut cur_video_motion: u8 = 0;
    let mut cur_storage_fault: u8 = 0;
    let mut cur_storage_full: u8 = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"DeviceID" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            s.device_id = txt.trim().to_string();
                        }
                    }
                }
                b"Result" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            s.result = txt.trim().to_string();
                        }
                    }
                }
                b"Online" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            s.on_line = txt.trim().to_string();
                        }
                    }
                }
                b"Status" => {
                    if !in_alarmstatus {
                        if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                            if let Ok(txt) = t.xml_content() {
                                s.status = txt.trim().to_string();
                            }
                        }
                    }
                }
                b"Encode" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            s.encode = txt.trim().to_string();
                        }
                    }
                }
                b"Record" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            s.record = txt.trim().to_string();
                        }
                    }
                }
                b"DeviceTime" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            s.device_time = txt.trim().to_string();
                        }
                    }
                }
                b"Alarmstatus" => {
                    in_alarmstatus = true;
                }
                b"DuressAlarm" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            cur_duress_alarm = txt.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"EnclosureAlarm" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            cur_enclosure_alarm = txt.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"VideoLost" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            cur_video_lost = txt.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"VideoMotion" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            cur_video_motion = txt.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"StorageFault" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            cur_storage_fault = txt.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"StorageFull" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(txt) = t.xml_content() {
                            cur_storage_full = txt.trim().parse().unwrap_or(0);
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"Alarmstatus" => {
                s.alarmstatus = Some(AlarmStatus {
                    duress_alarm: cur_duress_alarm,
                    enclosure_alarm: cur_enclosure_alarm,
                    video_lost: cur_video_lost,
                    video_motion: cur_video_motion,
                    storage_fault: cur_storage_fault,
                    storage_full: cur_storage_full,
                });
                in_alarmstatus = false;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    s
}

// ── PTZ 云台控制 ──────────────────────────────────────────────────────────────

/// PTZ 速度（0~255）
#[derive(Debug, Clone, Copy)]
pub struct PtzSpeed {
    pub pan: u8,  // 水平速度
    pub tilt: u8, // 垂直速度
    pub zoom: u8, // 变倍速度
}

impl Default for PtzSpeed {
    fn default() -> Self {
        Self {
            pan: 64,
            tilt: 64,
            zoom: 32,
        }
    }
}

/// PTZ 控制命令
#[derive(Debug, Clone)]
pub enum PtzCommand {
    /// 停止
    Stop,
    /// 向右
    Right(PtzSpeed),
    /// 向左
    Left(PtzSpeed),
    /// 向上
    Up(PtzSpeed),
    /// 向下
    Down(PtzSpeed),
    /// 右上
    RightUp(PtzSpeed),
    /// 右下
    RightDown(PtzSpeed),
    /// 左上
    LeftUp(PtzSpeed),
    /// 左下
    LeftDown(PtzSpeed),
    /// 放大
    ZoomIn(PtzSpeed),
    /// 缩小
    ZoomOut(PtzSpeed),
    /// 聚焦近
    FocusNear,
    /// 聚焦远
    FocusFar,
    /// 光圈开
    IrisOpen,
    /// 光圈关
    IrisClose,
}

/// 将 PTZ 命令转换为 GB28181 规范的 8 字节 PTZ 指令字符串
///
/// 格式：0xFF 0x01 HH WW V1 V2 V3 SS
/// - HH: 高速位（bit7=上 bit6=下 bit5=左 bit4=右 bit3=变倍 bit2=对焦 bit1=光圈）
/// - WW: 保留
/// - V1: 水平速度（0~255）
/// - V2: 垂直速度（0~255）
/// - V3: 变倍速度（0~255，高4位=变倍，低4位=聚焦/光圈）
/// - SS: 校验和
pub fn encode_ptz_cmd(cmd: &PtzCommand) -> String {
    let (hh, v1, v2, v3): (u8, u8, u8, u8) = match cmd {
        PtzCommand::Stop => (0x00, 0x00, 0x00, 0x00),
        PtzCommand::Right(s) => (0x01, s.pan, 0x00, 0x00),
        PtzCommand::Left(s) => (0x02, s.pan, 0x00, 0x00),
        PtzCommand::Down(s) => (0x04, 0x00, s.tilt, 0x00),
        PtzCommand::Up(s) => (0x08, 0x00, s.tilt, 0x00),
        PtzCommand::RightDown(s) => (0x05, s.pan, s.tilt, 0x00),
        PtzCommand::RightUp(s) => (0x09, s.pan, s.tilt, 0x00),
        PtzCommand::LeftDown(s) => (0x06, s.pan, s.tilt, 0x00),
        PtzCommand::LeftUp(s) => (0x0A, s.pan, s.tilt, 0x00),
        PtzCommand::ZoomIn(s) => (0x10, 0x00, 0x00, (s.zoom & 0x0F) << 4),
        PtzCommand::ZoomOut(s) => (0x20, 0x00, 0x00, (s.zoom & 0x0F) << 4),
        PtzCommand::FocusFar => (0x40, 0x00, 0x00, 0x01),
        PtzCommand::FocusNear => (0x40, 0x00, 0x00, 0x02),
        PtzCommand::IrisOpen => (0x40, 0x00, 0x00, 0x04),
        PtzCommand::IrisClose => (0x40, 0x00, 0x00, 0x08),
    };

    let sum: u8 =
        (0xFFu16 + 0x01u16 + hh as u16 + 0x00u16 + v1 as u16 + v2 as u16 + v3 as u16) as u8;
    format!(
        "A50F01{:02X}00{:02X}{:02X}{:02X}{:02X}",
        hh, v1, v2, v3, sum
    )
}

/// 构建 PTZ 控制 XML（平台 → 设备）
///
/// GB28181-2022 §8.4：DeviceControl/PTZCmd
pub fn build_ptz_control_xml(
    _device_id: &str,
    channel_id: &str,
    sn: u32,
    cmd: &PtzCommand,
) -> String {
    let ptz_cmd = encode_ptz_cmd(cmd);
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <PTZCmd>{ptz_cmd}</PTZCmd>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        ptz_cmd = ptz_cmd
    )
}

/// 构建看守位控制 XML（设置/调用预置位）
///
/// 注意：此函数已被 `build_preset_set_xml` 和 `build_preset_goto_xml` 取代
#[allow(dead_code)]
pub fn build_preset_control_xml(
    _device_id: &str,
    channel_id: &str,
    sn: u32,
    preset_index: u8,
    set: bool,
) -> String {
    // set=true 设置预置位，set=false 调用预置位
    let _cmd_type = if set { "SetPreset" } else { "GotoPreset" };
    let ptz_cmd = if set {
        format!("8F0{:02X}0000000{:02X}", preset_index, preset_index)
    } else {
        format!("8F0{:02X}0000000{:02X}", preset_index + 3, preset_index)
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <PTZCmd>{ptz_cmd}</PTZCmd>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        ptz_cmd = ptz_cmd,
    )
}

/// 构建设备控制（录像控制）XML
pub fn build_record_control_xml(
    _device_id: &str,
    channel_id: &str,
    sn: u32,
    start: bool,
) -> String {
    let record_cmd = if start { "Record" } else { "StopRecord" };
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <RecordCmd>{record_cmd}</RecordCmd>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        record_cmd = record_cmd
    )
}

/// 构建设备控制（主动告警控制）XML
pub fn build_guard_control_xml(_device_id: &str, channel_id: &str, sn: u32, guard: bool) -> String {
    let guard_cmd = if guard { "SetGuard" } else { "ResetGuard" };
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <GuardCmd>{guard_cmd}</GuardCmd>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        guard_cmd = guard_cmd
    )
}

// ── 录像查询 ──────────────────────────────────────────────────────────────────

/// 构建录像查询 XML（平台 → 设备）
///
/// GB28181-2022 §8.5 录像文件查询
///
/// # 参数
/// - `start_time` / `end_time`：格式 `"2024-01-01T00:00:00"` （ISO8601）
/// - `record_type`：0=全部 1=定时录像 2=报警录像 3=手动录像
pub fn build_record_info_query_xml(
    _device_id: &str,
    channel_id: &str,
    sn: u32,
    start_time: &str,
    end_time: &str,
    record_type: u8,
    file_path: &str,
) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>RecordInfo</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <StartTime>{start_time}</StartTime>\r\n\
         <EndTime>{end_time}</EndTime>\r\n\
         <FilePath>{file_path}</FilePath>\r\n\
         <Address></Address>\r\n\
         <Secrecy>0</Secrecy>\r\n\
         <Type>{record_type}</Type>\r\n\
         </Query>",
        sn = sn,
        channel_id = channel_id,
        start_time = start_time,
        end_time = end_time,
        file_path = file_path,
        record_type = record_type
    )
}

/// 录像文件条目
#[derive(Debug, Clone)]
pub struct RecordItem {
    pub device_id: String,
    pub name: String,
    pub file_path: String,
    pub address: String,
    pub start_time: String,
    pub end_time: String,
    pub secrecy: u8,
    pub record_type: String,
    pub file_size: Option<u64>,
}

/// 解析录像查询响应（使用 quick-xml）
pub fn parse_record_items(xml: &str) -> Vec<RecordItem> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut result = Vec::new();
    let mut buf = Vec::new();
    let mut in_item = false;

    let mut cur_device_id = String::new();
    let mut cur_name = String::new();
    let mut cur_file_path = String::new();
    let mut cur_address = String::new();
    let mut cur_start_time = String::new();
    let mut cur_end_time = String::new();
    let mut cur_secrecy: u8 = 0;
    let mut cur_record_type = String::new();
    let mut cur_file_size: Option<u64> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"Item" => {
                    in_item = true;
                    cur_device_id.clear();
                    cur_name.clear();
                    cur_file_path.clear();
                    cur_address.clear();
                    cur_start_time.clear();
                    cur_end_time.clear();
                    cur_secrecy = 0;
                    cur_record_type.clear();
                    cur_file_size = None;
                }
                b"DeviceID" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_device_id = s.trim().to_string();
                    }
                }
                b"Name" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_name = s.trim().to_string();
                    }
                }
                b"FilePath" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_file_path = s.trim().to_string();
                    }
                }
                b"Address" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_address = s.trim().to_string();
                    }
                }
                b"StartTime" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_start_time = s.trim().to_string();
                    }
                }
                b"EndTime" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_end_time = s.trim().to_string();
                    }
                }
                b"Secrecy" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_secrecy = s.trim().parse().unwrap_or(0);
                    }
                }
                b"Type" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_record_type = s.trim().to_string();
                    }
                }
                b"FileSize" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_file_size = s.trim().parse().ok();
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"Item" => {
                if !cur_device_id.is_empty() {
                    result.push(RecordItem {
                        device_id: std::mem::take(&mut cur_device_id),
                        name: std::mem::take(&mut cur_name),
                        file_path: std::mem::take(&mut cur_file_path),
                        address: std::mem::take(&mut cur_address),
                        start_time: std::mem::take(&mut cur_start_time),
                        end_time: std::mem::take(&mut cur_end_time),
                        secrecy: cur_secrecy,
                        record_type: std::mem::take(&mut cur_record_type),
                        file_size: cur_file_size,
                    });
                }
                in_item = false;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    result
}

// ── 历史回放控制 ──────────────────────────────────────────────────────────────

/// 历史回放控制命令
#[derive(Debug, Clone)]
pub enum PlaybackControl {
    /// 暂停
    Pause,
    /// 继续
    Resume,
    /// 快放（倍速：1/2/4/8）
    FastForward(u8),
    /// 慢放（倍速：1/2/4/8）
    SlowForward(u8),
    /// 拖动到指定时间
    Seek(String),
    /// 停止
    Stop,
}

/// 构建历史回放控制 XML（平台 → 设备）
///
/// GB28181-2022 §9.2：回放控制
pub fn build_playback_control_xml(device_id: &str, sn: u32, ctrl: &PlaybackControl) -> String {
    let (cmd, scale, range) = match ctrl {
        PlaybackControl::Pause => ("Pause".to_string(), String::new(), String::new()),
        PlaybackControl::Resume => ("Resume".to_string(), String::new(), String::new()),
        PlaybackControl::Stop => ("TearDown".to_string(), String::new(), String::new()),
        PlaybackControl::FastForward(n) => (
            "SpeedChange".to_string(),
            format!("<Scale>{}</Scale>\r\n", n),
            String::new(),
        ),
        PlaybackControl::SlowForward(n) => (
            "SpeedChange".to_string(),
            format!("<Scale>1/{}</Scale>\r\n", n),
            String::new(),
        ),
        PlaybackControl::Seek(time) => (
            "PlayPositionChange".to_string(),
            String::new(),
            format!("<Range>\r\n<Start>{}</Start>\r\n</Range>\r\n", time),
        ),
    };

    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>MediaControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <MediaControl>\r\n\
         <PlayCmd>{cmd}</PlayCmd>\r\n\
         {scale}\
         {range}\
         </MediaControl>\r\n\
         </Control>",
        sn = sn,
        device_id = device_id,
        cmd = cmd,
        scale = scale,
        range = range
    )
}

// ── 报警 ─────────────────────────────────────────────────────────────────────

/// 报警信息
#[derive(Debug, Clone)]
pub struct AlarmInfo {
    pub device_id: String,
    pub start_alarm_time: String,
    pub end_alarm_time: String,
    pub alarm_priority: u8,
    pub alarm_method: u8,
    pub alarm_type: String,
    pub alarm_description: String,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

/// 解析报警通知 XML（使用 quick-xml）
pub fn parse_alarm_notify(xml: &str) -> Option<AlarmInfo> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut device_id = None;
    let mut start_alarm_time = String::new();
    let mut end_alarm_time = String::new();
    let mut alarm_priority: u8 = 0;
    let mut alarm_method: u8 = 0;
    let mut alarm_type = String::new();
    let mut alarm_description = String::new();
    let mut longitude: Option<f64> = None;
    let mut latitude: Option<f64> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"DeviceID" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            device_id = Some(s.trim().to_string());
                        }
                    }
                }
                b"StartAlarmTime" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            start_alarm_time = s.trim().to_string();
                        }
                    }
                }
                b"EndAlarmTime" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            end_alarm_time = s.trim().to_string();
                        }
                    }
                }
                b"AlarmPriority" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            alarm_priority = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"AlarmMethod" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            alarm_method = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"AlarmType" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            alarm_type = s.trim().to_string();
                        }
                    }
                }
                b"AlarmDescription" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            alarm_description = s.trim().to_string();
                        }
                    }
                }
                b"Longitude" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            longitude = s.trim().parse().ok();
                        }
                    }
                }
                b"Latitude" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            latitude = s.trim().parse().ok();
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    device_id.map(|id| AlarmInfo {
        device_id: id,
        start_alarm_time,
        end_alarm_time,
        alarm_priority,
        alarm_method,
        alarm_type,
        alarm_description,
        longitude,
        latitude,
    })
}

/// 构建报警确认响应 XML（平台 → 设备）
pub fn build_alarm_ack_xml(device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Response>\r\n\
         <CmdType>Alarm</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <Result>OK</Result>\r\n\
         </Response>",
        sn = sn,
        device_id = device_id
    )
}

/// 构建报警订阅查询 XML（平台 → 设备，用于 SUBSCRIBE 消息体）
pub fn build_alarm_subscribe_xml(device_id: &str, sn: u32, alarm_type: u8, expire: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>Alarm</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <AlarmType>{alarm_type}</AlarmType>\r\n\
         <Expires>{expire}</Expires>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id,
        alarm_type = alarm_type,
        expire = expire
    )
}

// ── 媒体状态 ──────────────────────────────────────────────────────────────────

/// 解析媒体状态通知（设备推流结束时上报）
pub fn parse_media_status(xml: &str) -> Option<String> {
    // GB28181 §9.3 媒体状态通知
    parse_xml_field(xml, "NotifyType")
}

// ── 广播通知 ──────────────────────────────────────────────────────────────────

/// 构建广播通知 XML（平台 → 设备）
#[allow(dead_code)]
pub fn build_broadcast_xml(device_id: &str, sn: u32, source_id: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Notify>\r\n\
         <CmdType>Broadcast</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <SourceID>{source_id}</SourceID>\r\n\
         <TargetID>{device_id}</TargetID>\r\n\
         </Notify>",
        sn = sn,
        device_id = device_id,
        source_id = source_id
    )
}

// ── 设备配置查询 ───────────────────────────────────────────────────────────────

/// 设备配置类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigType {
    /// 基本参数配置
    Basic,
    /// 网络参数配置
    Network,
    /// 视频参数配置
    Video,
}

impl ConfigType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigType::Basic => "BasicParam",
            ConfigType::Network => "NetworkParam",
            ConfigType::Video => "VideoParam",
        }
    }
}

/// 设备配置条目
#[derive(Debug, Clone)]
pub struct ConfigItem {
    pub name: String,
    pub value: String,
}

/// 构建设备配置查询 XML（平台 → 设备）
///
/// GB28181-2022 A.2.4.7：ConfigDownload
pub fn build_config_download_query_xml(
    device_id: &str,
    sn: u32,
    config_type: ConfigType,
) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>ConfigDownload</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <ConfigType>{config_type}</ConfigType>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id,
        config_type = config_type.as_str()
    )
}

/// 解析设备配置响应（使用 quick-xml）
pub fn parse_config_download_response(xml: &str) -> Vec<ConfigItem> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut result = Vec::new();
    let mut buf = Vec::new();
    let mut in_item = false;
    let mut cur_name = String::new();
    let mut cur_value = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"Item" => {
                    in_item = true;
                    cur_name.clear();
                    cur_value.clear();
                }
                b"Name" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_name = s.trim().to_string();
                    }
                }
                b"Value" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_value = s.trim().to_string();
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"Item" => {
                if !cur_name.is_empty() {
                    result.push(ConfigItem {
                        name: std::mem::take(&mut cur_name),
                        value: std::mem::take(&mut cur_value),
                    });
                }
                in_item = false;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    result
}

// ── 预置位查询 ────────────────────────────────────────────────────────────────

/// 预置位信息
#[derive(Debug, Clone)]
pub struct PresetInfo {
    /// 预置位 ID
    pub preset_id: String,
    /// 预置位名称
    pub name: String,
}

/// 构建预置位列表查询 XML（平台 → 设备）
///
/// GB28181-2022 A.2.4.8：PresetList
pub fn build_preset_list_query_xml(device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>PresetList</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id
    )
}

/// 解析预置位列表响应（使用 quick-xml）
pub fn parse_preset_list(xml: &str) -> Vec<PresetInfo> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut result = Vec::new();
    let mut buf = Vec::new();
    let mut in_item = false;
    let mut cur_preset_id = String::new();
    let mut cur_name = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"Item" => {
                    in_item = true;
                    cur_preset_id.clear();
                    cur_name.clear();
                }
                b"PresetID" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_preset_id = s.trim().to_string();
                    }
                }
                b"PresetName" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_name = s.trim().to_string();
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"Item" => {
                if !cur_preset_id.is_empty() {
                    result.push(PresetInfo {
                        preset_id: std::mem::take(&mut cur_preset_id),
                        name: std::mem::take(&mut cur_name),
                    });
                }
                in_item = false;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    result
}

// ── 巡航轨迹查询 ──────────────────────────────────────────────────────────────

/// 巡航轨迹信息
#[derive(Debug, Clone)]
pub struct CruiseInfo {
    /// 巡航轨迹编号
    pub cruise_id: String,
    /// 巡航名称
    pub name: String,
}

/// 构建巡航轨迹列表查询 XML（平台 → 设备）
///
/// GB28181-2022 A.2.4.11：CruiseList
pub fn build_cruise_list_query_xml(device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>CruiseList</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id
    )
}

/// 解析巡航轨迹列表响应（使用 quick-xml）
pub fn parse_cruise_list(xml: &str) -> Vec<CruiseInfo> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut result = Vec::new();
    let mut buf = Vec::new();
    let mut in_item = false;
    let mut cur_cruise_id = String::new();
    let mut cur_name = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"Item" => {
                    in_item = true;
                    cur_cruise_id.clear();
                    cur_name.clear();
                }
                b"CruiseID" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_cruise_id = s.trim().to_string();
                    }
                }
                b"CruiseName" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_name = s.trim().to_string();
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"Item" => {
                if !cur_cruise_id.is_empty() {
                    result.push(CruiseInfo {
                        cruise_id: std::mem::take(&mut cur_cruise_id),
                        name: std::mem::take(&mut cur_name),
                    });
                }
                in_item = false;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    result
}

// ── 看守位信息查询 ─────────────────────────────────────────────────────────────

/// 看守位信息查询 XML（平台 → 设备）
///
/// GB28181-2022 A.2.4.10（2022 新增）
pub fn build_guard_info_query_xml(device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>GuardInfo</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id
    )
}

/// 看守位信息
#[derive(Debug, Clone)]
pub struct GuardInfo {
    pub guard_id: u8,
    pub preset_index: u8,
}

/// 解析看守位信息响应 XML（使用 quick-xml）
///
/// GB28181-2022 A.2.4.11：看守位信息查询响应
///
/// # 参数
/// - `xml`: MANSCDP XML 响应体
///
/// # 返回
/// - `Some(GuardInfo)`: 解析成功
/// - `None`: 解析失败或 XML 中无有效数据
pub fn parse_guard_info(xml: &str) -> Option<GuardInfo> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut guard_id: u8 = 0;
    let mut preset_index: u8 = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"GuardID" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            guard_id = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"PresetIndex" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            preset_index = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    // 只有当 guard_id > 0 时才返回
    if guard_id > 0 {
        Some(GuardInfo {
            guard_id,
            preset_index,
        })
    } else {
        None
    }
}

// ── 预置位/巡航控制 ────────────────────────────────────────────────────────────

/// 构建调用预置位 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.10：调用预置位
pub fn build_preset_goto_xml(channel_id: &str, sn: u32, preset_index: u8) -> String {
    let ptz_cmd = format!("8F0{:02X}0000000{:02X}", preset_index + 3, preset_index);
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <PTZCmd>{ptz_cmd}</PTZCmd>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        ptz_cmd = ptz_cmd
    )
}

/// 构建设置预置位 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.10：设置预置位
pub fn build_preset_set_xml(channel_id: &str, sn: u32, preset_index: u8) -> String {
    let ptz_cmd = format!("8F0{:02X}0000000{:02X}", preset_index, preset_index);
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <PTZCmd>{ptz_cmd}</PTZCmd>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        ptz_cmd = ptz_cmd
    )
}

/// 构建启动巡航 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.10：巡航控制
pub fn build_cruise_start_xml(channel_id: &str, sn: u32, cruise_no: u8) -> String {
    // 巡航控制码：0x8F 0x09 00 cruise_no 00 preset 00 00 checksum
    let ptz_cmd = format!("8F09{:02X}000000{:02X}", cruise_no, cruise_no);
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <PTZCmd>{ptz_cmd}</PTZCmd>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        ptz_cmd = ptz_cmd
    )
}

/// 构建停止巡航 XML（平台 → 设备）
pub fn build_cruise_stop_xml(channel_id: &str, sn: u32, cruise_no: u8) -> String {
    let ptz_cmd = format!("8F0A{:02X}000000{:02X}", cruise_no, cruise_no);
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <PTZCmd>{ptz_cmd}</PTZCmd>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        ptz_cmd = ptz_cmd
    )
}

// ── 语音广播/对讲 ──────────────────────────────────────────────────────────────

/// 广播会话信息
#[derive(Debug, Clone)]
pub struct BroadcastSession {
    /// 广播源 ID（平台）
    pub source_id: String,
    /// 广播目标 ID（设备）
    pub target_id: String,
}

/// 构建语音广播邀请 XML（平台 → 设备）
///
/// GB28181-2022 §9.12：平台向设备发起语音广播邀请
pub fn build_broadcast_invite_xml(source_id: &str, target_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Notify>\r\n\
         <CmdType>Broadcast</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <SourceID>{source_id}</SourceID>\r\n\
         <TargetID>{target_id}</TargetID>\r\n\
         </Notify>",
        sn = sn,
        source_id = source_id,
        target_id = target_id
    )
}

/// 构建语音广播取消 XML（平台 → 设备）
///
/// GB28181-2022 §9.12：结束语音广播
pub fn build_broadcast_cancel_xml(source_id: &str, target_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Notify>\r\n\
         <CmdType>Broadcast</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <SourceID>{source_id}</SourceID>\r\n\
         <TargetID>{target_id}</TargetID>\r\n\
         <NotifyType>TearDown</NotifyType>\r\n\
         </Notify>",
        sn = sn,
        source_id = source_id,
        target_id = target_id
    )
}

/// 解析语音广播邀请响应
///
/// 返回 `(result, audio_port)` — OK + 音频端口，或错误信息
pub fn parse_broadcast_ack(xml: &str) -> Option<(String, u16)> {
    let result = parse_xml_field(xml, "Result").unwrap_or_else(|| "ERROR".to_string());
    let audio_port = parse_xml_field(xml, "AudioPort")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    Some((result, audio_port))
}

// ── 网络校时 ──────────────────────────────────────────────────────────────────

/// 校时查询/响应类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSyncMode {
    /// 平台向设备校时（Query + Response）
    Query,
    /// 平台向设备下发时间（Response 下发）
    Response,
}

/// 校时信息
#[derive(Debug, Clone)]
pub struct TimeSyncInfo {
    /// 设备时间（ISO8601）
    pub device_time: String,
    /// 时间差（设备时间 - 本地时间，秒）
    pub time_diff_secs: f64,
}

/// 构建设备校时查询 XML（平台 → 设备）
///
/// GB28181-2022 §9.10：向设备查询当前时间
/// 使用 `<TimeRequest>` 标签，请求设备返回校时信息
pub fn build_time_sync_query_xml(_platform_id: &str, device_id: &str, sn: u32) -> String {
    // TimeRequest 使用 ISO8601 格式的时间字符串
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>DeviceStatus</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <TimeRequest>{time_request}</TimeRequest>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id,
        time_request = now
    )
}

/// 构建时间下发响应 XML（平台 → 设备，主动校时）
///
/// GB28181-2022 §9.10：平台向设备下发标准时间
pub fn build_time_sync_response_xml(device_id: &str, sn: u32) -> String {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Response>\r\n\
         <CmdType>DeviceStatus</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <CurrentTime>{current_time}</CurrentTime>\r\n\
         <Result>OK</Result>\r\n\
         </Response>",
        sn = sn,
        device_id = device_id,
        current_time = now
    )
}

/// 解析设备校时响应
///
/// 从 `<Response CmdType="DeviceStatus">` 中提取 `<DeviceTime>`
pub fn parse_time_sync_response(xml: &str) -> Option<TimeSyncInfo> {
    let device_time = parse_xml_field(xml, "DeviceTime")?;
    let local_time = chrono::Utc::now();

    // 解析设备时间
    // 解析设备时间（先 parse，再统一转换为 UTC）
    // 解析设备时间，统一转换为 UTC
    let dev_time_utc = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&device_time) {
        dt.with_timezone(&chrono::Utc)
    } else if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(&device_time, "%Y-%m-%dT%H:%M:%S")
    {
        ndt.and_utc()
    } else {
        return None;
    };

    let diff = (dev_time_utc - local_time).num_milliseconds() as f64 / 1000.0;

    Some(TimeSyncInfo {
        device_time,
        time_diff_secs: diff,
    })
}

// ── 远程启动 ──────────────────────────────────────────────────────────────────

/// 构建远程启动 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.3：远程启动
/// 用于远程唤醒休眠状态的设备
pub fn build_teleboot_xml(device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <TeleBoot>Boot</TeleBoot>\r\n\
         </Control>",
        sn = sn,
        device_id = device_id
    )
}

// ── 报警复位 ──────────────────────────────────────────────────────────────────

/// 构建报警复位 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.6：报警复位
/// 用于复位指定类型的报警状态
pub fn build_alarm_reset_xml(device_id: &str, sn: u32, alarm_type: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <AlarmReset>{alarm_type}</AlarmReset>\r\n\
         </Control>",
        sn = sn,
        device_id = device_id,
        alarm_type = alarm_type
    )
}

// ── 强制关键帧 ────────────────────────────────────────────────────────────────

/// 构建强制关键帧 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.7：强制关键帧
/// 用于请求设备立即生成一个关键帧（I帧），改善视频质量
pub fn build_make_video_record_xml(channel_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <MakeVideoRecord>Send</MakeVideoRecord>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id
    )
}

// ── 拉框缩放控制 ──────────────────────────────────────────────────────────────

/// 拉框缩放信息
#[derive(Debug, Clone)]
pub struct ZoomRect {
    /// 左上角 X 坐标（0-65535，归一化）
    pub x1: u16,
    /// 左上角 Y 坐标（0-65535，归一化）
    pub y1: u16,
    /// 右下角 X 坐标（0-65535，归一化）
    pub x2: u16,
    /// 右下角 Y 坐标（0-65535，归一化）
    pub y2: u16,
}

impl ZoomRect {
    /// 将归一化坐标转换为 GB28181 格式的 PTZ 命令字符串
    ///
    /// 格式：8F 00 5D PP QQ RR SS TT UU
    /// - PP: x1 高字节
    /// - QQ: x1 低字节
    /// - RR: y1 高字节
    /// - SS: y1 低字节
    /// - TT: x2 高字节
    /// - UU: x2 低字节
    pub fn to_ptz_cmd(&self) -> String {
        let (x1_h, x1_l) = ((self.x1 >> 8) as u8, (self.x1 & 0xFF) as u8);
        let (y1_h, y1_l) = ((self.y1 >> 8) as u8, (self.y1 & 0xFF) as u8);
        let (x2_h, x2_l) = ((self.x2 >> 8) as u8, (self.x2 & 0xFF) as u8);
        let (y2_h, y2_l) = ((self.y2 >> 8) as u8, (self.y2 & 0xFF) as u8);

        // 校验和：0x8F + 0x00 + 0x5D + x1_h + x1_l + y1_h + y1_l + x2_h + x2_l + y2_h + y2_l
        let sum: u8 = 0x8Fu8
            .wrapping_add(0x00u8)
            .wrapping_add(0x5Du8)
            .wrapping_add(x1_h)
            .wrapping_add(x1_l)
            .wrapping_add(y1_h)
            .wrapping_add(y1_l)
            .wrapping_add(x2_h)
            .wrapping_add(x2_l)
            .wrapping_add(y2_h)
            .wrapping_add(y2_l);

        format!(
            "8F005D{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            x1_h, x1_l, y1_h, y1_l, x2_h, x2_l, y2_h, y2_l, sum
        )
    }
}

/// 构建拉框放大 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.8：拉框放大
/// 指定的矩形区域将被放大至全屏
pub fn build_zoom_in_xml(channel_id: &str, sn: u32, rect: &ZoomRect) -> String {
    let ptz_cmd = rect.to_ptz_cmd();
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <ZoomIn>{ptz_cmd}</ZoomIn>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        ptz_cmd = ptz_cmd
    )
}

/// 构建拉框缩小 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.9：拉框缩小
/// 指定的矩形区域将被缩小显示
pub fn build_zoom_out_xml(channel_id: &str, sn: u32, rect: &ZoomRect) -> String {
    let ptz_cmd = rect.to_ptz_cmd();
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <ZoomOut>{ptz_cmd}</ZoomOut>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        ptz_cmd = ptz_cmd
    )
}

// ── PTZ 精准控制 ─────────────────────────────────────────────────────────────

/// PTZ 精准控制参数
#[derive(Debug, Clone)]
pub struct PtzPreciseParam {
    /// 水平绝对位置（0-10000）
    pub pan_position: u16,
    /// 垂直绝对位置（0-10000）
    pub tilt_position: u16,
    /// 变倍绝对位置（0-10000）
    pub zoom_position: u16,
    /// 聚焦绝对位置（0-10000）
    pub focus_position: Option<u16>,
    /// 光圈绝对位置（0-10000）
    pub iris_position: Option<u16>,
}

impl PtzPreciseParam {
    /// 转换为 GB28181 PTZ 精准控制命令字符串
    ///
    /// 格式：8F 00 91 PP PP QQ QQ RR RR [SS SS] [TT TT] KK
    /// - PP PP: 水平位置（2字节，大端）
    /// - QQ QQ: 垂直位置（2字节，大端）
    /// - RR RR: 变倍位置（2字节，大端）
    /// - SS SS: 聚焦位置（可选）
    /// - TT TT: 光圈位置（可选）
    /// - KK: 校验和
    pub fn to_ptz_cmd(&self) -> String {
        let (pan_h, pan_l) = (
            (self.pan_position >> 8) as u8,
            (self.pan_position & 0xFF) as u8,
        );
        let (tilt_h, tilt_l) = (
            (self.tilt_position >> 8) as u8,
            (self.tilt_position & 0xFF) as u8,
        );
        let (zoom_h, zoom_l) = (
            (self.zoom_position >> 8) as u8,
            (self.zoom_position & 0xFF) as u8,
        );

        if let (Some(focus), Some(iris)) = (self.focus_position, self.iris_position) {
            let (focus_h, focus_l) = ((focus >> 8) as u8, (focus & 0xFF) as u8);
            let (iris_h, iris_l) = ((iris >> 8) as u8, (iris & 0xFF) as u8);
            let sum: u8 = 0x8Fu8
                .wrapping_add(0x00u8)
                .wrapping_add(0x91u8)
                .wrapping_add(pan_h)
                .wrapping_add(pan_l)
                .wrapping_add(tilt_h)
                .wrapping_add(tilt_l)
                .wrapping_add(zoom_h)
                .wrapping_add(zoom_l)
                .wrapping_add(focus_h)
                .wrapping_add(focus_l)
                .wrapping_add(iris_h)
                .wrapping_add(iris_l);
            // 8F 00 91 + 水平(2) + 垂直(2) + 变倍(2) + 聚焦(2) + 光圈(2) + 校验和(1) = 13 字节
            format!(
                "8F0091{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                pan_h, pan_l, tilt_h, tilt_l, zoom_h, zoom_l, focus_h, focus_l, iris_h, iris_l, sum
            )
        } else {
            let sum: u8 = 0x8Fu8
                .wrapping_add(0x00u8)
                .wrapping_add(0x91u8)
                .wrapping_add(pan_h)
                .wrapping_add(pan_l)
                .wrapping_add(tilt_h)
                .wrapping_add(tilt_l)
                .wrapping_add(zoom_h)
                .wrapping_add(zoom_l);
            // 8F 00 91 + 水平(2) + 垂直(2) + 变倍(2) + 校验和(1) = 9 字节
            format!(
                "8F0091{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                pan_h, pan_l, tilt_h, tilt_l, zoom_h, zoom_l, sum
            )
        }
    }
}

/// 构建 PTZ 精准控制 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.11：PTZ 精准控制
/// 使用绝对位置进行云台控制，而非相对速度
pub fn build_ptz_precise_xml(channel_id: &str, sn: u32, param: &PtzPreciseParam) -> String {
    let ptz_cmd = param.to_ptz_cmd();
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <PTZPreciseCmd>{ptz_cmd}</PTZPreciseCmd>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        ptz_cmd = ptz_cmd
    )
}

// ── 存储卡管理 ────────────────────────────────────────────────────────────────

/// 构建存储卡格式化 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.13：存储卡格式化
pub fn build_storage_format_xml(device_id: &str, sn: u32, channel_id: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <ChannelID>{channel_id}</ChannelID>\r\n\
         <StorageFormat>Format</StorageFormat>\r\n\
         </Control>",
        sn = sn,
        device_id = device_id,
        channel_id = channel_id
    )
}

/// 存储卡状态信息
#[derive(Debug, Clone)]
pub struct StorageStatus {
    /// 设备 ID
    pub device_id: String,
    /// 存储类型（SD/NVR/Server）
    pub storage_type: String,
    /// 总容量（字节）
    pub total_space: u64,
    /// 可用容量（字节）
    pub free_space: u64,
    /// 存储状态（0=正常，1=异常，2=满）
    pub status: u8,
}

/// 解析存储卡状态查询响应（使用 quick-xml）
pub fn parse_storage_status(xml: &str) -> Option<StorageStatus> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut device_id = None;
    let mut total_space: u64 = 0;
    let mut free_space: u64 = 0;
    let mut status: u8 = 0;
    let mut storage_type = "SD".to_string();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"DeviceID" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            device_id = Some(s.trim().to_string());
                        }
                    }
                }
                b"TotalSpace" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            total_space = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"FreeSpace" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            free_space = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"Status" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            status = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"StorageType" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            storage_type = s.trim().to_string();
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    device_id.map(|id| StorageStatus {
        device_id: id,
        storage_type,
        total_space,
        free_space,
        status,
    })
}

/// 构建存储卡状态查询 XML（平台 → 设备）
///
/// GB28181-2022 A.2.4.14：存储卡状态查询（2022 新增）
pub fn build_storage_status_query_xml(device_id: &str, sn: u32, channel_id: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>StorageStatus</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <ChannelID>{channel_id}</ChannelID>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id,
        channel_id = channel_id
    )
}

// ── 目标跟踪 ──────────────────────────────────────────────────────────────────

/// 目标跟踪模式
#[derive(Debug, Clone, Copy)]
pub enum TargetTrackMode {
    /// 开始跟踪
    Start,
    /// 停止跟踪
    Stop,
}

/// 构建目标跟踪控制 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.14：目标跟踪（2022 新增）
pub fn build_target_track_xml(channel_id: &str, sn: u32, mode: TargetTrackMode) -> String {
    let cmd = match mode {
        TargetTrackMode::Start => "Start",
        TargetTrackMode::Stop => "Stop",
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Control>\r\n\
         <CmdType>DeviceControl</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{channel_id}</DeviceID>\r\n\
         <TargetTrack>{cmd}</TargetTrack>\r\n\
         </Control>",
        sn = sn,
        channel_id = channel_id,
        cmd = cmd
    )
}

// ── 巡航轨迹查询 ─────────────────────────────────────────────────────────────

/// 巡航轨迹段信息
#[derive(Debug, Clone)]
pub struct CruisePoint {
    /// 预置位编号
    pub preset_index: u8,
    /// 停留时间（秒）
    pub stay_time: u16,
    /// 速度
    pub speed: u8,
}

/// 巡航轨迹详情
#[derive(Debug, Clone)]
pub struct CruiseTrack {
    /// 巡航轨迹编号
    pub cruise_id: String,
    /// 巡航名称
    pub name: String,
    /// 巡航轨迹段列表
    pub points: Vec<CruisePoint>,
}

/// 构建巡航轨迹查询 XML（平台 → 设备）
///
/// GB28181-2022 A.2.4.12：巡航轨迹查询（2022 新增）
pub fn build_cruise_track_query_xml(device_id: &str, sn: u32, cruise_id: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>CruiseTrack</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <CruiseID>{cruise_id}</CruiseID>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id,
        cruise_id = cruise_id
    )
}

/// 解析巡航轨迹查询响应（使用 quick-xml）
pub fn parse_cruise_track(xml: &str) -> Vec<CruiseTrack> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut result = Vec::new();
    let mut buf = Vec::new();

    let mut in_item = false;
    let mut in_point = false;
    let mut cur_cruise_id = String::new();
    let mut cur_name = String::new();
    let mut cur_points: Vec<CruisePoint> = Vec::new();
    let mut cur_preset_index: u8 = 0;
    let mut cur_stay_time: u16 = 0;
    let mut cur_speed: u8 = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"Item" => {
                    in_item = true;
                    cur_cruise_id.clear();
                    cur_name.clear();
                    cur_points.clear();
                }
                b"CruiseID" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_cruise_id = s.trim().to_string();
                    }
                }
                b"CruiseName" => {
                    if in_item
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_name = s.trim().to_string();
                    }
                }
                b"Point" => {
                    in_point = true;
                    cur_preset_index = 0;
                    cur_stay_time = 0;
                    cur_speed = 0;
                }
                b"PresetID" => {
                    if in_point
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_preset_index = s.trim().parse().unwrap_or(0);
                    }
                }
                b"StayTime" => {
                    if in_point
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_stay_time = s.trim().parse().unwrap_or(0);
                    }
                }
                b"Speed" => {
                    if in_point
                        && let Ok(Event::Text(t)) = reader.read_event_into(&mut buf)
                        && let Ok(s) = t.xml_content()
                    {
                        cur_speed = s.trim().parse().unwrap_or(0);
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"Point" {
                    if in_point {
                        cur_points.push(CruisePoint {
                            preset_index: cur_preset_index,
                            stay_time: cur_stay_time,
                            speed: cur_speed,
                        });
                    }
                    in_point = false;
                } else if e.name().as_ref() == b"Item" {
                    if !cur_cruise_id.is_empty() {
                        result.push(CruiseTrack {
                            cruise_id: std::mem::take(&mut cur_cruise_id),
                            name: std::mem::take(&mut cur_name),
                            points: std::mem::take(&mut cur_points),
                        });
                    }
                    in_item = false;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    result
}

// ── PTZ 精准状态查询 ──────────────────────────────────────────────────────────

/// PTZ 精准位置信息
#[derive(Debug, Clone)]
pub struct PtzPreciseStatus {
    /// 设备 ID
    pub device_id: String,
    /// 水平位置（0-10000）
    pub pan_position: u16,
    /// 垂直位置（0-10000）
    pub tilt_position: u16,
    /// 变倍位置（0-10000）
    pub zoom_position: u16,
    /// 聚焦位置（可选）
    pub focus_position: Option<u16>,
    /// 光圈位置（可选）
    pub iris_position: Option<u16>,
}

/// 构建 PTZ 精准状态查询 XML（平台 → 设备）
///
/// GB28181-2022 A.2.4.13：PTZ 精准状态查询（2022 新增）
pub fn build_ptz_precise_status_query_xml(device_id: &str, sn: u32) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
         <Query>\r\n\
         <CmdType>PTZPreciseStatus</CmdType>\r\n\
         <SN>{sn}</SN>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         </Query>",
        sn = sn,
        device_id = device_id
    )
}

/// 解析 PTZ 精准状态响应（使用 quick-xml）
pub fn parse_ptz_precise_status(xml: &str) -> Option<PtzPreciseStatus> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut device_id = None;
    let mut pan_position: u16 = 0;
    let mut tilt_position: u16 = 0;
    let mut zoom_position: u16 = 0;
    let mut focus_position: Option<u16> = None;
    let mut iris_position: Option<u16> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"DeviceID" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            device_id = Some(s.trim().to_string());
                        }
                    }
                }
                b"PanPosition" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            pan_position = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"TiltPosition" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            tilt_position = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"ZoomPosition" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            zoom_position = s.trim().parse().unwrap_or(0);
                        }
                    }
                }
                b"FocusPosition" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            focus_position = s.trim().parse().ok();
                        }
                    }
                }
                b"IrisPosition" => {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        if let Ok(s) = t.xml_content() {
                            iris_position = s.trim().parse().ok();
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    device_id.map(|id| PtzPreciseStatus {
        device_id: id,
        pan_position,
        tilt_position,
        zoom_position,
        focus_position,
        iris_position,
    })
}

// ── 看守位控制（独立命令）────────────────────────────────────────────────────

/// 看守位控制模式
#[derive(Debug, Clone, Copy)]
pub enum GuardMode {
    /// 设置看守位
    Set,
    /// 调用看守位
    Call,
    /// 清除看守位
    Clear,
}

/// 构建看守位控制 XML（平台 → 设备）
///
/// GB28181-2022 A.2.3.1.10：看守位控制
pub fn build_guard_control_xml_v2(
    channel_id: &str,
    sn: u32,
    mode: GuardMode,
    preset_index: u8,
) -> String {
    let guard_cmd = match mode {
        GuardMode::Set => "SetGuard",
        GuardMode::Call => "CallGuard",
        GuardMode::Clear => "ResetGuard",
    };

    // 看守位设置/调用使用预置位命令
    let ptz_cmd = match mode {
        GuardMode::Set => format!("8F0{:02X}0000000{:02X}", preset_index, preset_index),
        GuardMode::Call => format!("8F0{:02X}0000000{:02X}", preset_index + 3, preset_index),
        GuardMode::Clear => format!("8F0{:02X}0000000{:02X}", preset_index + 6, preset_index),
    };

    format!(
"<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
<Control>\r\n\
<CmdType>DeviceControl</CmdType>\r\n\
<SN>{sn}</SN>\r\n\
<DeviceID>{channel_id}</DeviceID>\r\n\
<GuardCmd>{guard_cmd}</GuardCmd>\r\n\
<PTZCmd>{ptz_cmd}</PTZCmd>\r\n\
</Control>",
        sn = sn,
        channel_id = channel_id,
        guard_cmd = guard_cmd,
        ptz_cmd = ptz_cmd
    )
}

// ── 报警类型枚举 ─────────────────────────────────────────────────────────────

/// GB28181 报警类型枚举（GB28181-2022 §10.2.4.3）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlarmType {
    /// 1 - 紧急报警
    Emergency,
    /// 2 - 消防报警
    Fire,
    /// 3 - 非法入侵/布防区入侵
    Forced,
    /// 4 - 设备故障
    Equipment,
    /// 5 - 设备罩被打开
    EquipmentOpen,
    /// 6 - 周界入侵
    Pfa,
    /// 7 - 车辆检测
    Vehicle,
    /// 8 - 跨越围栏
    Pfd,
    /// 9 - 场景变更
    Scene,
    /// a - 移动侦测
    Vmd,
    /// b - 智能行为分析
    Smart,
    /// 其他自定义类型
    Custom(String),
}

impl AlarmType {
    /// 从 GB28181 AlarmType 字符串解析
    pub fn from_gb28181(s: &str) -> Self {
        match s.trim() {
            "1" => Self::Emergency,
            "2" => Self::Fire,
            "3" => Self::Forced,
            "4" => Self::Equipment,
            "5" => Self::EquipmentOpen,
            "6" => Self::Pfa,
            "7" => Self::Vehicle,
            "8" => Self::Pfd,
            "9" => Self::Scene,
            "a" | "A" => Self::Vmd,
            "b" | "B" => Self::Smart,
            other => Self::Custom(other.to_string()),
        }
    }

    /// 转换为 GB28181 标准编号字符串
    pub fn to_gb28181(&self) -> String {
        match self {
            Self::Emergency => "1".to_string(),
            Self::Fire => "2".to_string(),
            Self::Forced => "3".to_string(),
            Self::Equipment => "4".to_string(),
            Self::EquipmentOpen => "5".to_string(),
            Self::Pfa => "6".to_string(),
            Self::Vehicle => "7".to_string(),
            Self::Pfd => "8".to_string(),
            Self::Scene => "9".to_string(),
            Self::Vmd => "a".to_string(),
            Self::Smart => "b".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// 获取报警类型中文描述
    pub fn description(&self) -> &str {
        match self {
            Self::Emergency => "紧急报警",
            Self::Fire => "消防报警",
            Self::Forced => "非法入侵",
            Self::Equipment => "设备故障",
            Self::EquipmentOpen => "设备罩被打开",
            Self::Pfa => "周界入侵",
            Self::Vehicle => "车辆检测",
            Self::Pfd => "跨越围栏",
            Self::Scene => "场景变更",
            Self::Vmd => "移动侦测",
            Self::Smart => "智能行为分析",
            Self::Custom(_) => "自定义报警",
        }
    }
}

impl std::fmt::Display for AlarmType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

// ── 移动位置查询 ─────────────────────────────────────────────────────────────

/// 构建移动位置查询 MESSAGE body（平台 → 设备）
///
/// GB28181-2022 A.2.4.5：平台主动查询设备位置
/// `interval` 为 `None` 时表示仅查一次，`Some(secs)` 时表示设备按间隔持续上报
pub fn build_mobile_position_query_xml(
    device_id: &str,
    sn: u32,
    interval: Option<u32>,
) -> String {
    let interval_str = interval
        .map(|i| format!("\r\n<Interval>{}</Interval>", i))
        .unwrap_or_default();
    format!(
"<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
<Query>\r\n\
<CmdType>MobilePosition</CmdType>\r\n\
<SN>{sn}</SN>\r\n\
<DeviceID>{device_id}</DeviceID>\r\n\
{interval}</Query>",
        sn = sn,
        device_id = device_id,
        interval = interval_str,
    )
}

/// 构建取消移动位置订阅 XML（平台 → 设备）
///
/// interval=0 表示取消订阅
pub fn build_mobile_position_unsubscribe_xml(device_id: &str, sn: u32) -> String {
    build_mobile_position_query_xml(device_id, sn, Some(0))
}

// ── PTZ 锁定/解锁 ───────────────────────────────────────────────────────────

/// PTZ 云台锁定（A.2.3.1.12）
///
/// 锁定后其他平台无法控制该设备的云台
pub fn build_ptz_lock_xml(channel_id: &str, sn: u32) -> String {
    format!(
"<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
<Control>\r\n\
<CmdType>DeviceControl</CmdType>\r\n\
<SN>{sn}</SN>\r\n\
<DeviceID>{channel_id}</DeviceID>\r\n\
<PTZ>\r\n\
<lock>1</lock>\r\n\
</PTZ>\r\n\
</Control>",
        sn = sn,
        channel_id = channel_id,
    )
}

/// PTZ 云台解锁（A.2.3.1.12）
pub fn build_ptz_unlock_xml(channel_id: &str, sn: u32) -> String {
    format!(
"<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
<Control>\r\n\
<CmdType>DeviceControl</CmdType>\r\n\
<SN>{sn}</SN>\r\n\
<DeviceID>{channel_id}</DeviceID>\r\n\
<PTZ>\r\n\
<lock>0</lock>\r\n\
</PTZ>\r\n\
</Control>",
        sn = sn,
        channel_id = channel_id,
    )
}

// ── DeviceControl 手动抓拍 ───────────────────────────────────────────────────

/// DeviceControl 手动抓拍（非 INVITE 模式，A.2.3.1.4）
///
/// 通过 RecordCmd 控制设备抓拍，设备收到后主动推送图片
pub fn build_snapshot_control_xml(channel_id: &str, sn: u32) -> String {
    format!(
"<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
<Control>\r\n\
<CmdType>DeviceControl</CmdType>\r\n\
<SN>{sn}</SN>\r\n\
<DeviceID>{channel_id}</DeviceID>\r\n\
<RecordCmd>SHOOT</RecordCmd>\r\n\
</Control>",
        sn = sn,
        channel_id = channel_id,
    )
}

// ── 配置推送 ─────────────────────────────────────────────────────────────────

/// 推送配置参数到设备（A.2.3.2）
///
/// `device_id`: 设备 ID
/// `sn`: 消息序号
/// `config_type`: 配置类型
/// `params`: (name, value) 参数对列表
pub fn build_config_push_xml(
    device_id: &str,
    sn: u32,
    config_type: ConfigType,
    params: &[(String, String)],
) -> String {
    let params_xml: String = params
        .iter()
        .map(|(name, value)| format!("    <{}>{}</{}>", name, value, name))
        .collect::<Vec<_>>()
        .join("\r\n");
    let config_type_str = config_type.as_str();
    format!(
"<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
<Control>\r\n\
<CmdType>ConfigDownload</CmdType>\r\n\
<SN>{sn}</SN>\r\n\
<DeviceID>{device_id}</DeviceID>\r\n\
<ConfigType>{config_type_str}</ConfigType>\r\n\
<ConfigParam>\r\n\
{params}\r\n\
</ConfigParam>\r\n\
</Control>",
        sn = sn,
        device_id = device_id,
        params = params_xml,
    )
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xml_field() {
        let xml = "<Notify><CmdType>Keepalive</CmdType><SN>42</SN></Notify>";
        assert_eq!(
            parse_xml_field(xml, "CmdType"),
            Some("Keepalive".to_string())
        );
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
<Item><DeviceID>ch01</DeviceID><Name>Camera1</Name><Status>ON</Status><Manufacturer>Hikvision</Manufacturer><Model>DS-2CD</Model><Address>室内</Address></Item>
<Item><DeviceID>ch02</DeviceID><Name>Camera2</Name><Status>OFF</Status></Item>
</DeviceList>"#;
        let items = parse_catalog_items(xml);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].device_id, "ch01");
        assert_eq!(items[0].manufacturer, "Hikvision");
        assert_eq!(items[1].status.as_str(), "OFF");
    }

    #[test]
    fn test_encode_ptz_stop() {
        let cmd = encode_ptz_cmd(&PtzCommand::Stop);
        // stop: HH=00, v1=00, v2=00, v3=00, sum = (0xFF+1+0)&0xFF = 0
        assert!(cmd.starts_with("A50F01"));
    }

    #[test]
    fn test_encode_ptz_right() {
        let cmd = encode_ptz_cmd(&PtzCommand::Right(PtzSpeed {
            pan: 50,
            tilt: 0,
            zoom: 0,
        }));
        // HH = 0x01, v1=50, v2=0, v3=0
        assert!(cmd.contains("01"));
    }

    #[test]
    fn test_build_ptz_control_xml() {
        let xml = build_ptz_control_xml(
            "device1",
            "channel1",
            1,
            &PtzCommand::Right(PtzSpeed::default()),
        );
        assert!(xml.contains("DeviceControl"));
        assert!(xml.contains("PTZCmd"));
        assert!(xml.contains("channel1"));
    }

    #[test]
    fn test_parse_alarm_notify() {
        let xml = r#"<Notify>
<CmdType>Alarm</CmdType>
<SN>1</SN>
<DeviceID>34020000001310000001</DeviceID>
<StartAlarmTime>2024-01-01T10:00:00</StartAlarmTime>
<AlarmPriority>1</AlarmPriority>
<AlarmMethod>2</AlarmMethod>
<AlarmDescription>Motion Detected</AlarmDescription>
</Notify>"#;
        let alarm = parse_alarm_notify(xml).unwrap();
        assert_eq!(alarm.device_id, "34020000001310000001");
        assert_eq!(alarm.alarm_priority, 1);
    }

    #[test]
    fn test_parse_record_items() {
        let xml = r#"<RecordList Num="1">
<Item>
<DeviceID>34020000001320000001</DeviceID>
<Name>Rec1</Name>
<FilePath>/record/2024.mp4</FilePath>
<StartTime>2024-01-01T00:00:00</StartTime>
<EndTime>2024-01-01T01:00:00</EndTime>
<Secrecy>0</Secrecy>
<Type>time</Type>
</Item>
</RecordList>"#;
        let items = parse_record_items(xml);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].record_type, "time");
    }

    #[test]
    fn test_build_teleboot_xml() {
        let xml = build_teleboot_xml("34020000001320000001", 1);
        assert!(xml.contains("TeleBoot"));
        assert!(xml.contains("Boot"));
        assert!(xml.contains("DeviceControl"));
    }

    #[test]
    fn test_build_alarm_reset_xml() {
        let xml = build_alarm_reset_xml("34020000001320000001", 1, "1");
        assert!(xml.contains("AlarmReset"));
        assert!(xml.contains("1"));
    }

    #[test]
    fn test_build_make_video_record_xml() {
        let xml = build_make_video_record_xml("34020000001320000001", 1);
        assert!(xml.contains("MakeVideoRecord"));
        assert!(xml.contains("Send"));
    }

    #[test]
    fn test_zoom_rect_to_ptz_cmd() {
        let rect = ZoomRect {
            x1: 1000,
            y1: 2000,
            x2: 5000,
            y2: 4000,
        };
        let cmd = rect.to_ptz_cmd();
        assert!(cmd.starts_with("8F005D"));
        assert_eq!(cmd.len(), 24); // 8F 00 5D + 8字节坐标(16字符) + 1字节校验和(2字符)
    }

    #[test]
    fn test_build_zoom_in_xml() {
        let rect = ZoomRect {
            x1: 0,
            y1: 0,
            x2: 32768,
            y2: 32768,
        };
        let xml = build_zoom_in_xml("34020000001320000001", 1, &rect);
        assert!(xml.contains("ZoomIn"));
        assert!(xml.contains("8F005D"));
    }

    #[test]
    fn test_ptz_precise_param_to_cmd() {
        let param = PtzPreciseParam {
            pan_position: 5000,
            tilt_position: 5000,
            zoom_position: 5000,
            focus_position: Some(5000),
            iris_position: Some(5000),
        };
        let cmd = param.to_ptz_cmd();
        assert!(cmd.starts_with("8F0091"));
        // 8F 00 91(6) + pan(4) + tilt(4) + zoom(4) + focus(4) + iris(4) + 校验和(2) = 28字符
        assert_eq!(cmd.len(), 28);
    }

    #[test]
    fn test_build_ptz_precise_xml() {
        let param = PtzPreciseParam {
            pan_position: 5000,
            tilt_position: 5000,
            zoom_position: 5000,
            focus_position: None,
            iris_position: None,
        };
        let xml = build_ptz_precise_xml("34020000001320000001", 1, &param);
        assert!(xml.contains("PTZPreciseCmd"));
        assert!(xml.contains("8F0091"));
    }

    #[test]
    fn test_build_storage_format_xml() {
        let xml = build_storage_format_xml("34020000001320000001", 1, "34020000001320000011");
        assert!(xml.contains("StorageFormat"));
        assert!(xml.contains("Format"));
    }

    #[test]
    fn test_parse_storage_status() {
        let xml = r#"<Response>
<CmdType>StorageStatus</CmdType>
<SN>1</SN>
<DeviceID>34020000001320000001</DeviceID>
<StorageType>SD</StorageType>
<TotalSpace>32000000000</TotalSpace>
<FreeSpace>16000000000</FreeSpace>
<Status>0</Status>
</Response>"#;
        let status = parse_storage_status(xml).unwrap();
        assert_eq!(status.device_id, "34020000001320000001");
        assert_eq!(status.total_space, 32000000000);
        assert_eq!(status.free_space, 16000000000);
        assert_eq!(status.status, 0);
    }

    #[test]
    fn test_build_target_track_xml() {
        let xml_start = build_target_track_xml("34020000001320000001", 1, TargetTrackMode::Start);
        assert!(xml_start.contains("TargetTrack"));
        assert!(xml_start.contains("Start"));

        let xml_stop = build_target_track_xml("34020000001320000001", 2, TargetTrackMode::Stop);
        assert!(xml_stop.contains("Stop"));
    }

    #[test]
    fn test_parse_cruise_track() {
        let xml = r#"<Response>
<CmdType>CruiseTrack</CmdType>
<SN>1</SN>
<DeviceID>34020000001320000001</DeviceID>
<Item>
<CruiseID>1</CruiseID>
<CruiseName>Track1</CruiseName>
<Point><PresetID>1</PresetID><StayTime>5</StayTime><Speed>3</Speed></Point>
<Point><PresetID>2</PresetID><StayTime>10</StayTime><Speed>5</Speed></Point>
</Item>
</Response>"#;
        let tracks = parse_cruise_track(xml);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].cruise_id, "1");
        assert_eq!(tracks[0].name, "Track1");
        assert_eq!(tracks[0].points.len(), 2);
        assert_eq!(tracks[0].points[0].preset_index, 1);
        assert_eq!(tracks[0].points[0].stay_time, 5);
    }

    #[test]
    fn test_parse_ptz_precise_status() {
        let xml = r#"<Response>
<CmdType>PTZPreciseStatus</CmdType>
<SN>1</SN>
<DeviceID>34020000001320000001</DeviceID>
<PanPosition>5000</PanPosition>
<TiltPosition>3000</TiltPosition>
<ZoomPosition>2000</ZoomPosition>
<FocusPosition>4000</FocusPosition>
<IrisPosition>5000</IrisPosition>
</Response>"#;
        let status = parse_ptz_precise_status(xml).unwrap();
        assert_eq!(status.pan_position, 5000);
        assert_eq!(status.tilt_position, 3000);
        assert_eq!(status.zoom_position, 2000);
        assert_eq!(status.focus_position, Some(4000));
    }

    #[test]
    fn test_build_guard_control_v2() {
        let xml_set = build_guard_control_xml_v2("34020000001320000001", 1, GuardMode::Set, 1);
        assert!(xml_set.contains("SetGuard"));

        let xml_call = build_guard_control_xml_v2("34020000001320000001", 2, GuardMode::Call, 1);
        assert!(xml_call.contains("CallGuard"));

        let xml_clear = build_guard_control_xml_v2("34020000001320000001", 3, GuardMode::Clear, 1);
        assert!(xml_clear.contains("ResetGuard"));
    }
}
