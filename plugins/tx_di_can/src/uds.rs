//! UDS (ISO 14229) 诊断服务客户端
//!
//! ## 支持的服务
//! | SID  | 服务名                          | 方法                        |
//! |------|---------------------------------|-----------------------------|
//! | 0x10 | DiagnosticSessionControl        | `session_control()`         |
//! | 0x11 | ECUReset                        | `ecu_reset()`               |
//! | 0x14 | ClearDiagnosticInformation      | `clear_dtc()`               |
//! | 0x19 | ReadDTCInformation              | `read_dtc()`                |
//! | 0x22 | ReadDataByIdentifier            | `read_data()`               |
//! | 0x23 | ReadMemoryByAddress             | `read_memory()`             |
//! | 0x27 | SecurityAccess                  | `security_access()`         |
//! | 0x2E | WriteDataByIdentifier           | `write_data()`              |
//! | 0x2F | InputOutputControlByIdentifier  | `io_control()`              |
//! | 0x31 | RoutineControl                  | `routine_control()`         |
//! | 0x34 | RequestDownload                 | `request_download()`        |
//! | 0x35 | RequestUpload                   | `request_upload()`          |
//! | 0x36 | TransferData                    | `transfer_data()`           |
//! | 0x37 | RequestTransferExit             | `request_transfer_exit()`   |
//! | 0x3E | TesterPresent                   | `tester_present()`          |

use crate::adapter::CanAdapter;
use crate::event::{emit_event, CanEvent};
use crate::isotp::{IsoTpChannel, IsoTpConfig};
use anyhow::Result;
use std::sync::Arc;
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────────
// 类型定义
// ─────────────────────────────────────────────────────────────────────────────

/// UDS 服务 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UdsService {
    DiagnosticSessionControl = 0x10,
    EcuReset = 0x11,
    ClearDiagnosticInformation = 0x14,
    ReadDtcInformation = 0x19,
    ReadDataByIdentifier = 0x22,
    ReadMemoryByAddress = 0x23,
    SecurityAccess = 0x27,
    WriteDataByIdentifier = 0x2E,
    InputOutputControlByIdentifier = 0x2F,
    RoutineControl = 0x31,
    RequestDownload = 0x34,
    RequestUpload = 0x35,
    TransferData = 0x36,
    RequestTransferExit = 0x37,
    TesterPresent = 0x3E,
}

impl UdsService {
    pub fn response_sid(self) -> u8 {
        self as u8 + 0x40
    }
}

/// 负响应码 (NRC)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NrcCode {
    GeneralReject = 0x10,
    ServiceNotSupported = 0x11,
    SubFunctionNotSupported = 0x12,
    IncorrectMessageLengthOrInvalidFormat = 0x13,
    ResponseTooLong = 0x14,
    BusyRepeatRequest = 0x21,
    ConditionsNotCorrect = 0x22,
    RequestSequenceError = 0x24,
    RequestOutOfRange = 0x31,
    SecurityAccessDenied = 0x33,
    InvalidKey = 0x35,
    ExceededNumberOfAttempts = 0x36,
    RequiredTimeDelayNotExpired = 0x37,
    UploadDownloadNotAccepted = 0x70,
    TransferDataSuspended = 0x71,
    GeneralProgrammingFailure = 0x72,
    WrongBlockSequenceCounter = 0x73,
    ResponsePending = 0x78,
    SubFunctionNotSupportedInActiveSession = 0x7E,
    ServiceNotSupportedInActiveSession = 0x7F,
    Unknown(u8),
}

impl From<u8> for NrcCode {
    fn from(v: u8) -> Self {
        match v {
            0x10 => NrcCode::GeneralReject,
            0x11 => NrcCode::ServiceNotSupported,
            0x12 => NrcCode::SubFunctionNotSupported,
            0x13 => NrcCode::IncorrectMessageLengthOrInvalidFormat,
            0x14 => NrcCode::ResponseTooLong,
            0x21 => NrcCode::BusyRepeatRequest,
            0x22 => NrcCode::ConditionsNotCorrect,
            0x24 => NrcCode::RequestSequenceError,
            0x31 => NrcCode::RequestOutOfRange,
            0x33 => NrcCode::SecurityAccessDenied,
            0x35 => NrcCode::InvalidKey,
            0x36 => NrcCode::ExceededNumberOfAttempts,
            0x37 => NrcCode::RequiredTimeDelayNotExpired,
            0x70 => NrcCode::UploadDownloadNotAccepted,
            0x71 => NrcCode::TransferDataSuspended,
            0x72 => NrcCode::GeneralProgrammingFailure,
            0x73 => NrcCode::WrongBlockSequenceCounter,
            0x78 => NrcCode::ResponsePending,
            0x7E => NrcCode::SubFunctionNotSupportedInActiveSession,
            0x7F => NrcCode::ServiceNotSupportedInActiveSession,
            other => NrcCode::Unknown(other),
        }
    }
}

impl std::fmt::Display for NrcCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NrcCode::GeneralReject => write!(f, "通用拒绝(0x10)"),
            NrcCode::ServiceNotSupported => write!(f, "服务不支持(0x11)"),
            NrcCode::ConditionsNotCorrect => write!(f, "条件不满足(0x22)"),
            NrcCode::SecurityAccessDenied => write!(f, "安全访问被拒绝(0x33)"),
            NrcCode::InvalidKey => write!(f, "密钥无效(0x35)"),
            NrcCode::ResponsePending => write!(f, "响应挂起(0x78)"),
            NrcCode::GeneralProgrammingFailure => write!(f, "通用编程失败(0x72)"),
            NrcCode::WrongBlockSequenceCounter => write!(f, "块序号错误(0x73)"),
            NrcCode::Unknown(v) => write!(f, "未知NRC(0x{:02X})", v),
            other => write!(f, "{:?}", other),
        }
    }
}

/// UDS 诊断错误
#[derive(Debug, Error)]
pub enum UdsError {
    #[error("UDS 负响应: service=0x{service:02X}, NRC={nrc}")]
    NegativeResponse { service: u8, nrc: NrcCode },
    #[error("UDS 超时: service=0x{service:02X}")]
    Timeout { service: u8 },
    #[error("UDS 响应格式错误: {0}")]
    InvalidResponse(String),
    #[error("传输层错误: {0}")]
    Transport(#[from] anyhow::Error),
}

/// UDS 会话类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SessionType {
    Default = 0x01,
    Programming = 0x02,
    Extended = 0x03,
}

/// DTC 记录
#[derive(Debug, Clone)]
pub struct DtcRecord {
    pub dtc_code: u32,
    pub status_mask: u8,
}

/// UDS 客户端
pub struct UdsClient {
    isotp: IsoTpChannel,
    p2_timeout_ms: u64,
    p2_star_timeout_ms: u64,
}

impl UdsClient {
    pub fn new(
        adapter: Arc<dyn CanAdapter>,
        config: IsoTpConfig,
        p2_timeout_ms: u64,
        p2_star_timeout_ms: u64,
    ) -> Self {
        UdsClient {
            isotp: IsoTpChannel::new(adapter, config),
            p2_timeout_ms,
            p2_star_timeout_ms,
        }
    }

    // ─────────────────────────────────────────────
    // 底层收发
    // ─────────────────────────────────────────────

    /// 发送请求并等待正响应（自动处理 pendingResponse 0x78）
    async fn request(&self, service: UdsService, payload: &[u8]) -> Result<Vec<u8>, UdsError> {
        let mut req = vec![service as u8];
        req.extend_from_slice(payload);

        emit_event(CanEvent::UdsRequest {
            service: service as u8,
            payload: req.clone(),
        })
        .await;

        self.isotp
            .send(&req)
            .await
            .map_err(UdsError::Transport)?;

        // 等待响应，处理 NRC 0x78 pendingResponse
        let deadline = tokio::time::Instant::now()
            + std::time::Duration::from_millis(self.p2_star_timeout_ms);

        loop {
            let resp = self
                .isotp
                .recv(self.p2_timeout_ms)
                .await
                .map_err(|_| UdsError::Timeout { service: service as u8 })?;

            if resp.is_empty() {
                return Err(UdsError::InvalidResponse("空响应".to_string()));
            }

            // 负响应：0x7F <SID> <NRC>
            if resp[0] == 0x7F {
                if resp.len() < 3 {
                    return Err(UdsError::InvalidResponse("负响应格式错误".to_string()));
                }
                let nrc = NrcCode::from(resp[2]);
                if nrc == NrcCode::ResponsePending {
                    // ECU 还在处理，继续等（直到 p2_star_timeout）
                    if tokio::time::Instant::now() < deadline {
                        emit_event(CanEvent::UdsNegativeResponse {
                            service: service as u8,
                            nrc: 0x78,
                        })
                        .await;
                        continue;
                    } else {
                        return Err(UdsError::Timeout { service: service as u8 });
                    }
                }
                emit_event(CanEvent::UdsNegativeResponse {
                    service: service as u8,
                    nrc: resp[2],
                })
                .await;
                return Err(UdsError::NegativeResponse {
                    service: service as u8,
                    nrc,
                });
            }

            // 正响应校验
            let expected_resp_sid = service.response_sid();
            if resp[0] != expected_resp_sid {
                return Err(UdsError::InvalidResponse(format!(
                    "期望响应 SID 0x{:02X}，收到 0x{:02X}",
                    expected_resp_sid, resp[0]
                )));
            }

            emit_event(CanEvent::UdsResponse {
                service: service as u8,
                payload: resp.clone(),
            })
            .await;

            return Ok(resp[1..].to_vec()); // 去掉响应 SID
        }
    }

    // ─────────────────────────────────────────────
    // 诊断服务 API
    // ─────────────────────────────────────────────

    /// 0x10 DiagnosticSessionControl
    pub async fn session_control(&self, session: SessionType) -> Result<(), UdsError> {
        self.request(UdsService::DiagnosticSessionControl, &[session as u8])
            .await?;
        Ok(())
    }

    /// 0x11 ECUReset
    /// - reset_type: 0x01=硬复位, 0x02=钥匙断电复位, 0x03=软复位
    pub async fn ecu_reset(&self, reset_type: u8) -> Result<(), UdsError> {
        self.request(UdsService::EcuReset, &[reset_type]).await?;
        Ok(())
    }

    /// 0x3E TesterPresent（保持会话活跃）
    pub async fn tester_present(&self) -> Result<(), UdsError> {
        // sub-function 0x00 with suppressPosRspMsgIndicationBit=0
        self.request(UdsService::TesterPresent, &[0x00]).await?;
        Ok(())
    }

    /// 0x22 ReadDataByIdentifier
    pub async fn read_data(&self, did: u16) -> Result<Vec<u8>, UdsError> {
        let payload = [(did >> 8) as u8, (did & 0xFF) as u8];
        let resp = self.request(UdsService::ReadDataByIdentifier, &payload).await?;
        // resp = [did_hi, did_lo, data...]
        if resp.len() < 2 {
            return Err(UdsError::InvalidResponse("ReadDataByIdentifier 响应太短".to_string()));
        }
        Ok(resp[2..].to_vec())
    }

    /// 0x2E WriteDataByIdentifier
    pub async fn write_data(&self, did: u16, data: &[u8]) -> Result<(), UdsError> {
        let mut payload = vec![(did >> 8) as u8, (did & 0xFF) as u8];
        payload.extend_from_slice(data);
        self.request(UdsService::WriteDataByIdentifier, &payload).await?;
        Ok(())
    }

    /// 0x27 SecurityAccess — 两步握手（seed-key）
    ///
    /// `key_fn`：由调用方实现密钥算法（ECU 相关，不内置）
    pub async fn security_access<F>(&self, level: u8, key_fn: F) -> Result<(), UdsError>
    where
        F: Fn(&[u8]) -> Vec<u8>,
    {
        // 步骤1：请求 seed（level = odd number，如 0x01）
        let seed_resp = self
            .request(UdsService::SecurityAccess, &[level])
            .await?;
        // seed_resp = [level+1, seed...]
        if seed_resp.is_empty() {
            return Err(UdsError::InvalidResponse("SecurityAccess seed 响应为空".to_string()));
        }
        let seed = &seed_resp[1..]; // 去掉 sub-function echo

        // 步骤2：发送 key（level+1 = even number）
        let key = key_fn(seed);
        let mut key_payload = vec![level + 1];
        key_payload.extend_from_slice(&key);
        self.request(UdsService::SecurityAccess, &key_payload)
            .await?;
        Ok(())
    }

    /// 0x14 ClearDiagnosticInformation
    pub async fn clear_dtc(&self, group_of_dtc: u32) -> Result<(), UdsError> {
        let payload = [
            ((group_of_dtc >> 16) & 0xFF) as u8,
            ((group_of_dtc >> 8) & 0xFF) as u8,
            (group_of_dtc & 0xFF) as u8,
        ];
        self.request(UdsService::ClearDiagnosticInformation, &payload)
            .await?;
        Ok(())
    }

    /// 0x19 ReadDTCInformation（sub-function 0x02: reportDTCByStatusMask）
    pub async fn read_dtc(&self, status_mask: u8) -> Result<Vec<DtcRecord>, UdsError> {
        let resp = self
            .request(UdsService::ReadDtcInformation, &[0x02, status_mask])
            .await?;
        // resp = [sub_fn_echo, dtc_status_availability_mask, dtc_and_status_records...]
        if resp.len() < 2 {
            return Ok(vec![]);
        }
        let mut records = vec![];
        let mut i = 2usize; // skip sub_fn + availability_mask
        while i + 3 < resp.len() {
            let code = ((resp[i] as u32) << 16)
                | ((resp[i + 1] as u32) << 8)
                | (resp[i + 2] as u32);
            let status = resp[i + 3];
            records.push(DtcRecord {
                dtc_code: code,
                status_mask: status,
            });
            i += 4;
        }
        Ok(records)
    }

    /// 0x23 ReadMemoryByAddress
    pub async fn read_memory(
        &self,
        address: u32,
        length: u16,
        addr_len: u8, // 地址长度（字节数，1-4）
        len_len: u8,  // 长度字段长度（字节数，1-2）
    ) -> Result<Vec<u8>, UdsError> {
        let addr_and_len_fmt = (len_len << 4) | addr_len;
        let mut payload = vec![addr_and_len_fmt];
        for i in (0..addr_len).rev() {
            payload.push(((address >> (i * 8)) & 0xFF) as u8);
        }
        for i in (0..len_len).rev() {
            payload.push(((length >> (i * 8)) & 0xFF) as u8);
        }
        let resp = self.request(UdsService::ReadMemoryByAddress, &payload).await?;
        Ok(resp)
    }

    /// 0x31 RoutineControl
    pub async fn routine_control(
        &self,
        sub_fn: u8,
        routine_id: u16,
        option_data: &[u8],
    ) -> Result<Vec<u8>, UdsError> {
        let mut payload = vec![sub_fn, (routine_id >> 8) as u8, (routine_id & 0xFF) as u8];
        payload.extend_from_slice(option_data);
        let resp = self.request(UdsService::RoutineControl, &payload).await?;
        Ok(resp)
    }

    // ─────────────────────────────────────────────
    // 下载/上传原语（供刷写引擎使用）
    // ─────────────────────────────────────────────

    /// 0x34 RequestDownload
    /// 返回 maxBlockSize（ECU 单次 TransferData 最大字节数）
    pub async fn request_download(
        &self,
        address: u32,
        length: u32,
        compression: u8, // 0x00=不压缩
        encrypting: u8,  // 0x00=不加密
        addr_len: u8,
        size_len: u8,
    ) -> Result<usize, UdsError> {
        let data_format = (compression << 4) | (encrypting & 0x0F);
        let addr_and_len_fmt = (size_len << 4) | addr_len;
        let mut payload = vec![data_format, addr_and_len_fmt];
        for i in (0..addr_len).rev() {
            payload.push(((address >> (i * 8)) & 0xFF) as u8);
        }
        for i in (0..size_len).rev() {
            payload.push(((length >> (i * 8)) & 0xFF) as u8);
        }
        let resp = self.request(UdsService::RequestDownload, &payload).await?;
        // resp[0] = lengthFormatIdentifier, resp[1..] = maxBlockSize
        if resp.is_empty() {
            return Err(UdsError::InvalidResponse("RequestDownload 响应为空".to_string()));
        }
        let size_field_len = ((resp[0] & 0xF0) >> 4) as usize;
        if resp.len() < 1 + size_field_len {
            return Err(UdsError::InvalidResponse(
                "RequestDownload 响应长度不足".to_string(),
            ));
        }
        let mut max_block = 0usize;
        for b in &resp[1..=size_field_len] {
            max_block = (max_block << 8) | (*b as usize);
        }
        Ok(max_block)
    }

    /// 0x35 RequestUpload
    pub async fn request_upload(
        &self,
        address: u32,
        length: u32,
        addr_len: u8,
        size_len: u8,
    ) -> Result<usize, UdsError> {
        let addr_and_len_fmt = (size_len << 4) | addr_len;
        let mut payload = vec![0x00, addr_and_len_fmt];
        for i in (0..addr_len).rev() {
            payload.push(((address >> (i * 8)) & 0xFF) as u8);
        }
        for i in (0..size_len).rev() {
            payload.push(((length >> (i * 8)) & 0xFF) as u8);
        }
        let resp = self.request(UdsService::RequestUpload, &payload).await?;
        if resp.is_empty() {
            return Err(UdsError::InvalidResponse("RequestUpload 响应为空".to_string()));
        }
        let size_field_len = ((resp[0] & 0xF0) >> 4) as usize;
        if resp.len() < 1 + size_field_len {
            return Err(UdsError::InvalidResponse("RequestUpload 响应长度不足".to_string()));
        }
        let mut max_block = 0usize;
        for b in &resp[1..=size_field_len] {
            max_block = (max_block << 8) | (*b as usize);
        }
        Ok(max_block)
    }

    /// 0x36 TransferData（单块）
    pub async fn transfer_data(&self, block_seq: u8, data: &[u8]) -> Result<Vec<u8>, UdsError> {
        let mut payload = vec![block_seq];
        payload.extend_from_slice(data);
        self.request(UdsService::TransferData, &payload).await
    }

    /// 0x37 RequestTransferExit
    pub async fn request_transfer_exit(&self) -> Result<(), UdsError> {
        self.request(UdsService::RequestTransferExit, &[]).await?;
        Ok(())
    }
}
