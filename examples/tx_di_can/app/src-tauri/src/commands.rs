//! Tauri 命令：封装 `tx_di_can` 的公开 API 供前端调用
//!
//! 每个命令返回 `Result<T, String>`：成功值经 serde 序列化给前端，
//! 错误统一转为字符串消息。所有底层错误（`anyhow::Error` / `UdsError`）均 `.to_string()`。

use serde::Deserialize;
use std::sync::Arc;
use tx_di_can::{
    audit, dbc::Dbc, report, BusStats, CanConfig, CanFdFrame, CanFrame, CanPlugin, CsvAnalysis,
    FlashConfig, FrameFilter,     ProjectConfig, SessionType, UdsClient, A2l, A2lType, XcpMaster, parse_a2l,
};

/// 描述库 DID 摘要（供前端描述面板展示）
#[derive(Debug, serde::Serialize)]
pub struct DescDidInfo {
    pub id: u16,
    pub name: String,
    pub unit: String,
}

/// 描述库 DTC 摘要
#[derive(Debug, serde::Serialize)]
pub struct DescDtcInfo {
    pub code: u32,
    pub text: String,
}

/// DBC 信号摘要
#[derive(Debug, serde::Serialize)]
pub struct DbcSigInfo {
    pub name: String,
    pub unit: String,
    pub factor: f64,
    pub offset: f64,
    pub is_signed: bool,
}

/// DBC 消息摘要
#[derive(Debug, serde::Serialize)]
pub struct DbcMsgInfo {
    pub id: u32,
    pub name: String,
    pub dlc: u8,
    pub signals: Vec<DbcSigInfo>,
}

/// DBC 整体摘要
#[derive(Debug, serde::Serialize)]
pub struct DbcSummary {
    pub messages: Vec<DbcMsgInfo>,
}

/// DBC 解码结果
#[derive(Debug, serde::Serialize)]
pub struct DbcValue {
    pub name: String,
    pub value: f64,
}

/// 前端传入的帧过滤器（ID 范围 / 掩码匹配）
#[derive(Debug, Deserialize)]
pub struct FrameFilterInput {
    /// ID 下限（含），可空
    #[serde(default)]
    pub id_min: Option<u32>,
    /// ID 上限（含），可空
    #[serde(default)]
    pub id_max: Option<u32>,
    /// 掩码（0 表示不按掩码匹配）
    #[serde(default)]
    pub id_mask: u32,
    /// 期望匹配值
    #[serde(default)]
    pub id_match: u32,
}

impl From<FrameFilterInput> for FrameFilter {
    fn from(f: FrameFilterInput) -> Self {
        FrameFilter {
            id_min: f.id_min,
            id_max: f.id_max,
            id_mask: f.id_mask,
            id_match: f.id_match,
        }
    }
}

/// 前端传入的帧（JSON 友好，避免直接构造 `FrameId` 枚举）
#[derive(Debug, Deserialize)]
pub struct FrameInput {
    /// CAN ID（数值，>0x7FF 自动判为扩展帧）
    pub id: u32,
    /// 数据字节
    pub data: Vec<u8>,
    /// FD 帧：Bit Rate Switch
    #[serde(default)]
    pub brs: bool,
    /// FD 帧：Error State Indicator
    #[serde(default)]
    pub esi: bool,
}

/// 前端传入的刷写配置（仅暴露常用字段，其余用默认值）
#[derive(Debug, Deserialize)]
pub struct FlashConfigInput {
    /// ECU 物理寻址 ID（发送 CAN ID）
    pub target_id: u32,
    /// 安全访问等级（奇数，请求 seed）
    #[serde(default = "default_security_level")]
    pub security_level: u8,
    /// 目标内存起始地址
    #[serde(default)]
    pub memory_address: u32,
    /// 下载前是否显式全擦
    #[serde(default)]
    pub erase_before_download: bool,
    /// 每块数据最大字节数
    #[serde(default = "default_block_size")]
    pub default_block_size: usize,
}

fn default_security_level() -> u8 {
    0x01
}
fn default_block_size() -> usize {
    4096
}

// ── 连接管理 ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn default_config() -> CanConfig {
    CanPlugin::default_config()
}

#[tauri::command]
pub fn get_config() -> Option<CanConfig> {
    CanPlugin::get_config()
}

#[tauri::command]
pub fn is_connected() -> bool {
    CanPlugin::is_connected()
}

#[tauri::command]
pub async fn connect(config: CanConfig) -> Result<(), String> {
    let r = CanPlugin::connect(config).await;
    match &r {
        Ok(()) => audit::ok("connect", "ok"),
        Err(e) => audit::fail("connect", "fail", &e.to_string()),
    }
    r.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disconnect() -> Result<(), String> {
    CanPlugin::disconnect().await;
    audit::ok("disconnect", "ok");
    Ok(())
}

// ── 原始帧收发 ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn send_frame(frame: FrameInput) -> Result<(), String> {
    let len = frame.data.len();
    let f = CanFrame::new(frame.id, frame.data);
    let r = CanPlugin::send_frame(&f).await;
    match &r {
        Ok(()) => audit::ok("send", &format!("id=0x{:X} len={}", frame.id, len)),
        Err(e) => audit::fail("send", &format!("id=0x{:X}", frame.id), &e.to_string()),
    }
    r.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_fd_frame(frame: FrameInput) -> Result<(), String> {
    let len = frame.data.len();
    let mut f = CanFdFrame::new(frame.id, frame.data);
    f.brs = frame.brs;
    f.esi = frame.esi;
    let r = CanPlugin::send_fd_frame(&f).await;
    match &r {
        Ok(()) => audit::ok("send_fd", &format!("id=0x{:X} len={}", frame.id, len)),
        Err(e) => audit::fail("send_fd", &format!("id=0x{:X}", frame.id), &e.to_string()),
    }
    r.map_err(|e| e.to_string())
}

// ── UDS 诊断 ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn read_data(tx_id: u32, did: u16) -> Result<String, String> {
    let r = CanPlugin::read_data(tx_id, did).await;
    match &r {
        Ok(d) => audit::ok("read_did", &format!("tx=0x{:X} did=0x{:X} len={}", tx_id, did, d.len())),
        Err(e) => audit::fail("read_did", &format!("tx=0x{:X} did=0x{:X}", tx_id, did), &e.to_string()),
    }
    r.map(|data| hex_encode(&data)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_data(tx_id: u32, did: u16, data: Vec<u8>) -> Result<(), String> {
    let r = CanPlugin::write_data(tx_id, did, &data).await;
    match &r {
        Ok(()) => audit::ok("write_did", &format!("tx=0x{:X} did=0x{:X} len={}", tx_id, did, data.len())),
        Err(e) => audit::fail("write_did", &format!("tx=0x{:X} did=0x{:X}", tx_id, did), &e.to_string()),
    }
    r.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn session_control(tx_id: u32, session: u8) -> Result<(), String> {
    uds(tx_id)
        .session_control(session_type_from_u8(session))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ecu_reset(tx_id: u32, reset_type: u8) -> Result<(), String> {
    uds(tx_id).ecu_reset(reset_type).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tester_present(tx_id: u32) -> Result<(), String> {
    uds(tx_id).tester_present().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn security_access(tx_id: u32, level: u8, key_algo: String) -> Result<(), String> {
    let key_fn = make_key_fn(&key_algo);
    uds(tx_id)
        .security_access(level, &key_fn)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_dtc(tx_id: u32, status_mask: u8) -> Result<Vec<String>, String> {
    let dtcs = uds(tx_id).read_dtc(status_mask).await.map_err(|e| e.to_string())?;
    Ok(dtcs
        .into_iter()
        .map(|d| format!("DTC {:04X} status {:02X}", d.dtc_code, d.status_mask))
        .collect())
}

// ── 刷写 ───────────────────────────────────────────────────────────────────
//
// 进度/完成/失败通过 `can://event` 的 FlashProgress/FlashComplete/FlashError 推送，
// 命令本身仅在刷写全部完成后返回（或失败返回错误串）。

#[tauri::command]
pub async fn flash(
    firmware_path: String,
    config: FlashConfigInput,
    key_algo: String,
) -> Result<(), String> {
    let fc = build_flash_config(&config);
    let key_fn = make_key_fn(&key_algo);
    let r = CanPlugin::flash(firmware_path.clone(), fc, key_fn).await;
    match &r {
        Ok(_) => audit::ok("flash", &format!("fw={} target=0x{:X}", firmware_path, config.target_id)),
        Err(e) => audit::fail("flash", &format!("fw={} target=0x{:X}", firmware_path, config.target_id), &e.to_string()),
    }
    r.map(|_| ()).map_err(|e| e.to_string())
}

// ── 监控增强：统计与过滤 ───────────────────────────────────────────────────

#[tauri::command]
pub fn get_bus_stats() -> Option<BusStats> {
    CanPlugin::get_stats()
}

#[tauri::command]
pub fn reset_stats() {
    CanPlugin::reset_stats()
}

#[tauri::command]
pub fn set_frame_filter(filter: Option<FrameFilterInput>) -> Result<(), String> {
    CanPlugin::set_filter(filter.map(Into::into));
    Ok(())
}

#[tauri::command]
pub fn get_frame_filter() -> Option<FrameFilter> {
    CanPlugin::get_filter()
}

// ── ISO-TP 原始收发 ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn send_isotp(tx_id: u32, rx_id: u32, data: Vec<u8>) -> Result<(), String> {
    let ch = CanPlugin::create_isotp_channel(tx_id, rx_id);
    let r = ch.send(&data).await;
    match &r {
        Ok(()) => audit::ok("isotp", &format!("tx=0x{:X} rx=0x{:X} len={}", tx_id, rx_id, data.len())),
        Err(e) => audit::fail("isotp", &format!("tx=0x{:X} rx=0x{:X}", tx_id, rx_id), &e.to_string()),
    }
    r.map_err(|e| e.to_string())
}

// ── 描述库查询 ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_desc_dids() -> Vec<DescDidInfo> {
    match CanPlugin::desc_db() {
        Some(db) => db
            .supported_dids()
            .into_iter()
            .filter_map(|id| {
                let m = db.did_meta(id)?;
                Some(DescDidInfo {
                    id,
                    name: m.name.clone(),
                    unit: m.unit.clone().unwrap_or_default(),
                })
            })
            .collect(),
        None => Vec::new(),
    }
}

#[tauri::command]
pub fn get_desc_dtcs() -> Vec<DescDtcInfo> {
    match CanPlugin::desc_db() {
        Some(db) => db
            .supported_dtc_codes()
            .into_iter()
            .map(|code| DescDtcInfo {
                code,
                text: db.dtc_text(code).unwrap_or("未知").to_string(),
            })
            .collect(),
        None => Vec::new(),
    }
}

#[tauri::command]
pub fn sim_ecu_status() -> bool {
    CanPlugin::sim_ecu_enabled()
}

// ── 录制 / 回放 ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn record_csv(path: String, duration_ms: u64) -> Result<u32, String> {
    let r = CanPlugin::record_csv(&path, duration_ms).await;
    match &r {
        Ok(n) => audit::ok("record", &format!("path={} frames={}", path, n)),
        Err(e) => audit::fail("record", &path, &e.to_string()),
    }
    r.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn replay_csv(path: String, speed_factor: f64) -> Result<u32, String> {
    let r = CanPlugin::replay_csv(&path, speed_factor).await;
    match &r {
        Ok(n) => audit::ok("replay", &format!("path={} frames={}", path, n)),
        Err(e) => audit::fail("replay", &path, &e.to_string()),
    }
    r.map_err(|e| e.to_string())
}

// ── DBC 解码 ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn load_dbc(path: String) -> Result<DbcSummary, String> {
    let txt = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let dbc = Dbc::parse(&txt).map_err(|e| e.to_string())?;
    Ok(DbcSummary {
        messages: dbc
            .messages
            .into_iter()
            .map(|m| DbcMsgInfo {
                id: m.id,
                name: m.name,
                dlc: m.dlc,
                signals: m
                    .signals
                    .into_iter()
                    .map(|s| DbcSigInfo {
                        name: s.name,
                        unit: s.unit,
                        factor: s.factor,
                        offset: s.offset,
                        is_signed: s.is_signed,
                    })
                    .collect(),
            })
            .collect(),
    })
}

#[tauri::command]
pub fn decode_dbc(path: String, can_id: u32, data: Vec<u8>) -> Result<Vec<DbcValue>, String> {
    let txt = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let dbc = Dbc::parse(&txt).map_err(|e| e.to_string())?;
    Ok(dbc
        .decode(can_id, &data)
        .into_iter()
        .map(|(name, value)| DbcValue { name, value })
        .collect())
}

// ── 辅助 ───────────────────────────────────────────────────────────────────

/// 取得指定 tx/rx 的 UDS 客户端（rx = tx + 8，符合标准请求/响应配对）
fn uds(tx_id: u32) -> Arc<UdsClient> {
    CanPlugin::uds_client(tx_id, tx_id.wrapping_add(8))
}

/// UDS 会话类型映射（0x01 默认 / 0x02 编程 / 0x03 扩展）
fn session_type_from_u8(v: u8) -> SessionType {
    match v {
        0x01 => SessionType::Default,
        0x02 => SessionType::Programming,
        0x03 => SessionType::Extended,
        _ => SessionType::Default,
    }
}

/// 安全访问密钥算法：目前支持 `none`（原样）与默认 `negate`（按位取反）
fn make_key_fn(algo: &str) -> Box<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync> {
    match algo {
        "none" => Box::new(|s: &[u8]| s.to_vec()),
        _ => Box::new(|s: &[u8]| s.iter().map(|b| !b).collect()),
    }
}

/// 由前端输入构造完整 FlashConfig（其余字段取合理默认）
fn build_flash_config(c: &FlashConfigInput) -> FlashConfig {
    FlashConfig {
        target_id: c.target_id,
        security_level: c.security_level,
        session_type: SessionType::Programming,
        default_block_size: c.default_block_size,
        erase_before_download: c.erase_before_download,
        verify_routine_id: 0x02,
        memory_address: c.memory_address,
        memory_size_len: 4,
        compression: 0x00,
        encryption: 0x00,
        erase_routine_id: 0xFF,
        routine_option: vec![],
    }
}

/// 字节数组转空格分隔的大写十六进制串
fn hex_encode(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

// ── A. XCP 标定（A2L + 仿真从站/主站） ─────────────────────────────────────

/// XCP 变量信息（测量量/标定量）
#[derive(Debug, serde::Serialize)]
pub struct XcpVarInfo {
    pub name: String,
    pub datatype: String,
    pub address: u32,
    pub unit: String,
}

/// XCP A2L 解析结果
#[derive(Debug, serde::Serialize)]
pub struct XcpA2lInfo {
    pub module: String,
    pub measurements: Vec<XcpVarInfo>,
    pub characteristics: Vec<XcpVarInfo>,
}

/// XCP 采样值（测量量当前值）
#[derive(Debug, serde::Serialize)]
pub struct XcpValue {
    pub name: String,
    pub hex: String,
    pub raw: Vec<u8>,
}

fn a2l_type_name(t: A2lType) -> String {
    match t {
        A2lType::UByte => "UBYTE",
        A2lType::UWord => "UWORD",
        A2lType::ULong => "ULONG",
        A2lType::SByte => "SBYTE",
        A2lType::SWord => "SWORD",
        A2lType::SLong => "SLONG",
        A2lType::Float32Ieee => "FLOAT32_IEEE",
    }
    .to_string()
}

fn to_var_info(name: &str, dt: A2lType, addr: u32, unit: &str) -> XcpVarInfo {
    XcpVarInfo {
        name: name.to_string(),
        datatype: a2l_type_name(dt),
        address: addr,
        unit: unit.to_string(),
    }
}

#[tauri::command]
pub fn xcp_parse_a2l(path: String) -> Result<XcpA2lInfo, String> {
    let txt = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let a2l = parse_a2l(&txt);
    Ok(XcpA2lInfo {
        module: a2l.module.clone(),
        measurements: a2l
            .measurements
            .iter()
            .map(|m| to_var_info(&m.name, m.datatype, m.address, &m.unit))
            .collect(),
        characteristics: a2l
            .characteristics
            .iter()
            .map(|c| to_var_info(&c.name, c.datatype, c.address, &c.unit))
            .collect(),
    })
}

/// 连接仿真从站并对所有测量量执行 UPLOAD，返回当前值
#[tauri::command]
pub fn xcp_measure_all(path: String) -> Result<Vec<XcpValue>, String> {
    let txt = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let a2l = parse_a2l(&txt);
    let mut master = XcpMaster::from_a2l(a2l);
    if !master.connect() {
        return Err("XCP CONNECT 失败".to_string());
    }
    let mut out = Vec::new();
    for m in master.a2l().measurements.clone() {
        if let Some(raw) = master.read_measurement(&m.name) {
            out.push(XcpValue {
                name: m.name,
                hex: hex_encode(&raw),
                raw,
            });
        }
    }
    Ok(out)
}

/// 对标定量执行 DOWNLOAD（标定写入）
#[tauri::command]
pub fn xcp_calibrate(path: String, name: String, data: Vec<u8>) -> Result<(), String> {
    let txt = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let a2l = parse_a2l(&txt);
    let mut master = XcpMaster::from_a2l(a2l);
    if !master.calibrate(&name, &data) {
        return Err(format!("未找到标定量 {name}"));
    }
    audit::ok("xcp", &format!("calibrate {name}"));
    Ok(())
}

/// 对单个测量量建立 DAQ 并采样一次
#[tauri::command]
pub fn xcp_daq_sample(path: String, name: String) -> Result<XcpValue, String> {
    let txt = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let a2l = parse_a2l(&txt);
    let mut master = XcpMaster::from_a2l(a2l);
    if !master.connect() {
        return Err("XCP CONNECT 失败".to_string());
    }
    if !master.start_daq(&name) {
        return Err(format!("未找到测量量 {name}"));
    }
    match master.daq_sample(&name) {
        Some(raw) => Ok(XcpValue {
            name,
            hex: hex_encode(&raw),
            raw,
        }),
        None => Err("DAQ 采样失败".to_string()),
    }
}

// ── B. 审计 / 报表导出 ────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, Clone)]
pub struct AuditEntryInfo {
    pub ts_ms: u64,
    pub kind: String,
    pub detail: String,
    pub result: String,
}

#[tauri::command]
pub fn audit_log() -> Vec<AuditEntryInfo> {
    audit::log()
        .into_iter()
        .map(|e| AuditEntryInfo {
            ts_ms: e.ts_ms,
            kind: e.kind,
            detail: e.detail,
            result: e.result,
        })
        .collect()
}

#[tauri::command]
pub fn audit_clear() {
    audit::clear();
}

#[tauri::command]
pub fn export_report(path: String, format: String) -> Result<(), String> {
    let entries = audit::log();
    match format.as_str() {
        "pdf" => report::export_pdf(&path, "CAN 诊断上位机 操作审计报表", &entries)
            .map_err(|e| e.to_string()),
        _ => report::export_html(&path, "CAN 诊断上位机 操作审计报表", &entries)
            .map_err(|e| e.to_string()),
    }
}

// ── C. 离线分析 ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn analyze_csv(path: String, bitrate: u32) -> Result<CsvAnalysis, String> {
    tx_di_can::record::analyze_csv(&path, bitrate)
        .map_err(|e| e.to_string())
}

// ── D. 工程管理（.canproj） ───────────────────────────────────────────────

#[tauri::command]
pub fn save_project(path: String, cfg: ProjectConfig) -> Result<(), String> {
    cfg.save(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_project(path: String) -> Result<ProjectConfig, String> {
    ProjectConfig::load(&path).map_err(|e| e.to_string())
}


