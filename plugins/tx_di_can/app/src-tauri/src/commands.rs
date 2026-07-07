//! Tauri 命令：封装 `tx_di_can` 的公开 API 供前端调用
//!
//! 每个命令返回 `Result<T, String>`：成功值经 serde 序列化给前端，
//! 错误统一转为字符串消息。所有底层错误（`anyhow::Error` / `UdsError`）均 `.to_string()`。

use serde::Deserialize;
use std::sync::Arc;
use tx_di_can::{
    CanConfig, CanFdFrame, CanFrame, CanPlugin, FlashConfig, SessionType, UdsClient,
};

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
    CanPlugin::connect(config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disconnect() -> Result<(), String> {
    CanPlugin::disconnect().await;
    Ok(())
}

// ── 原始帧收发 ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn send_frame(frame: FrameInput) -> Result<(), String> {
    let f = CanFrame::new(frame.id, frame.data);
    CanPlugin::send_frame(&f).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_fd_frame(frame: FrameInput) -> Result<(), String> {
    let mut f = CanFdFrame::new(frame.id, frame.data);
    f.brs = frame.brs;
    f.esi = frame.esi;
    CanPlugin::send_fd_frame(&f)
        .await
        .map_err(|e| e.to_string())
}

// ── UDS 诊断 ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn read_data(tx_id: u32, did: u16) -> Result<String, String> {
    let data = CanPlugin::read_data(tx_id, did)
        .await
        .map_err(|e| e.to_string())?;
    Ok(hex_encode(&data))
}

#[tauri::command]
pub async fn write_data(tx_id: u32, did: u16, data: Vec<u8>) -> Result<(), String> {
    CanPlugin::write_data(tx_id, did, &data)
        .await
        .map_err(|e| e.to_string())
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
    CanPlugin::flash(firmware_path, fc, key_fn)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
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
