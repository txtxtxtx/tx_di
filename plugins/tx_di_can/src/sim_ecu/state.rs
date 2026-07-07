//! ECU 仿真状态机与 UDS 应答逻辑
//!
//! 维护会话/安全等级/内存镜像/DID 存储/下载状态，根据收到的 UDS 请求
//! 生成正响应或负响应（NRC）。支持刷写 7 步所需的最小 bootloader 行为。

use crate::db::DescDb;
use crate::sim_ecu::seedkey::{compute_key, generate_seed};
use crate::uds::{NrcCode, SessionType};
use std::collections::BTreeMap;

/// ECU 仿真运行时状态
pub struct SimEcuState {
    /// 当前会话
    pub session: SessionType,
    /// 已解锁的安全等级（None 表示未解锁）
    pub unlocked_level: Option<u8>,
    /// DID 数据存储（read_data/write_data）
    pub did_store: BTreeMap<u16, Vec<u8>>,
    /// 内存镜像（read_memory_by_address / 下载写入）
    pub memory: BTreeMap<u32, u8>,
    /// 当前下载目标地址（0x34 设置）
    pub download_base: u32,
    /// 已写入偏移（0x36 推进）
    pub download_offset: u32,
    /// 是否已通过请求下载（下载激活）
    pub download_active: bool,
    /// 压缩方法（0x00 = 不压缩；来自 0x34 协商）
    pub compression_method: u8,
    /// 加密方法（0x00 = 不加密；来自 0x34 协商）
    pub encryption_method: u8,
    /// 已协商的下载总大小（来自 0x34 的 memorySize）
    pub download_size: u32,
    /// 受支持的 DTC 列表
    pub dtc_codes: Vec<u32>,
    /// 安全性：是否需要先解锁才能下载
    pub require_security_for_flash: bool,
    /// 是否需要编程会话才能下载
    pub require_programming_session: bool,
    /// 暂存 seed（请求 seed 时记录，用于校验 key）
    pub pending_seed: Vec<u8>,
    /// 暂存请求 seed 的 level
    pub pending_level: u8,
}

impl SimEcuState {
    pub fn new(db: &DescDb) -> Self {
        let mut did_store = BTreeMap::new();
        for id in db.supported_dids() {
            if let Some(data) = db.did_default_data(id) {
                did_store.insert(id, data);
            }
        }
        SimEcuState {
            session: SessionType::Default,
            unlocked_level: None,
            did_store,
            memory: BTreeMap::new(),
            download_base: 0,
            download_offset: 0,
            download_active: false,
            compression_method: 0,
            encryption_method: 0,
            download_size: 0,
            dtc_codes: db.supported_dtc_codes(),
            require_security_for_flash: true,
            require_programming_session: true,
            pending_seed: Vec::new(),
            pending_level: 0,
        }
    }

    /// 处理一条 UDS 请求，返回完整响应报文（含响应 SID 或 0x7F 负响应）
    pub fn handle(&mut self, req: &[u8]) -> Vec<u8> {
        if req.is_empty() {
            return negative(0x00, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let service = req[0];
        let payload = &req[1..];
        match service {
            0x10 => self.svc_session(payload),
            0x11 => self.svc_ecu_reset(payload),
            0x3E => self.svc_tester_present(payload),
            0x22 => self.svc_read_data(payload),
            0x2E => self.svc_write_data(payload),
            0x27 => self.svc_security(payload),
            0x14 => self.svc_clear_dtc(payload),
            0x19 => self.svc_read_dtc(payload),
            0x23 => self.svc_read_memory(payload),
            0x31 => self.svc_routine(payload),
            0x34 => self.svc_request_download(payload),
            0x36 => self.svc_transfer_data(payload),
            0x37 => self.svc_transfer_exit(payload),
            _ => negative(service, NrcCode::ServiceNotSupported),
        }
    }

    // ── 0x10 DiagnosticSessionControl ──
    fn svc_session(&mut self, p: &[u8]) -> Vec<u8> {
        if p.is_empty() {
            return negative(0x10, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let s = match p[0] {
            0x01 => SessionType::Default,
            0x02 => SessionType::Programming,
            0x03 => SessionType::Extended,
            _ => return negative(0x10, NrcCode::SubFunctionNotSupported),
        };
        self.session = s;
        // 正响应：0x50 <session> <P2_server_max_ms hi lo> <P2*_server_max hi lo>
        vec![0x50, p[0], 0x01, 0x90, 0x13, 0x88]
    }

    // ── 0x11 ECUReset ──
    fn svc_ecu_reset(&mut self, p: &[u8]) -> Vec<u8> {
        if p.is_empty() {
            return negative(0x11, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let rt = p[0];
        if rt == 0x01 || rt == 0x02 || rt == 0x03 {
            // 复位后恢复默认会话与锁定状态
            self.session = SessionType::Default;
            self.unlocked_level = None;
            self.download_active = false;
            vec![0x51, rt]
        } else {
            negative(0x11, NrcCode::SubFunctionNotSupported)
        }
    }

    // ── 0x3E TesterPresent ──
    fn svc_tester_present(&mut self, _p: &[u8]) -> Vec<u8> {
        vec![0x7E, 0x00]
    }

    // ── 0x22 ReadDataByIdentifier（支持多 DID 一次读）──
    fn svc_read_data(&mut self, p: &[u8]) -> Vec<u8> {
        if p.len() < 2 || p.len() % 2 != 0 {
            return negative(0x22, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let mut r = vec![0x62];
        let mut idx = 0;
        while idx + 2 <= p.len() {
            let did = ((p[idx] as u16) << 8) | (p[idx + 1] as u16);
            match self.did_store.get(&did) {
                Some(data) => {
                    r.push(p[idx]);
                    r.push(p[idx + 1]);
                    r.extend_from_slice(data);
                }
                None => return negative(0x22, NrcCode::RequestOutOfRange),
            }
            idx += 2;
        }
        r
    }

    // ── 0x2E WriteDataByIdentifier ──
    fn svc_write_data(&mut self, p: &[u8]) -> Vec<u8> {
        if p.len() < 2 {
            return negative(0x2E, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let did = ((p[0] as u16) << 8) | (p[1] as u16);
        let data = p[2..].to_vec();
        self.did_store.insert(did, data);
        vec![0x6E, p[0], p[1]]
    }

    // ── 0x27 SecurityAccess ──
    fn svc_security(&mut self, p: &[u8]) -> Vec<u8> {
        if p.is_empty() {
            return negative(0x27, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let level = p[0];
        if level % 2 == 1 {
            // 请求 seed
            let seed = generate_seed(4, level);
            self.pending_seed = seed.clone();
            self.pending_level = level;
            let mut r = vec![0x67, level];
            r.extend_from_slice(&seed);
            r
        } else {
            // 发送 key
            let key = &p[1..];
            let expected = compute_key(&self.pending_seed, self.pending_level);
            if key == expected.as_slice() {
                self.unlocked_level = Some(self.pending_level);
                vec![0x67, level]
            } else {
                negative(0x27, NrcCode::InvalidKey)
            }
        }
    }

    // ── 0x14 ClearDiagnosticInformation ──
    fn svc_clear_dtc(&mut self, _p: &[u8]) -> Vec<u8> {
        vec![0x54]
    }

    // ── 0x19 ReadDTCInformation ──
    fn svc_read_dtc(&mut self, p: &[u8]) -> Vec<u8> {
        if p.is_empty() {
            return negative(0x19, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let sub = p[0];
        match sub {
            // 0x02 reportDTCByStatusMask
            0x02 => {
                let mask = p.get(1).copied().unwrap_or(0xFF);
                let mut r = vec![0x59, 0x02, mask]; // 第三字节为状态可用性掩码
                for code in &self.dtc_codes {
                    r.push((code >> 16) as u8);
                    r.push((code >> 8) as u8);
                    r.push(*code as u8);
                    r.push(0x08); // 全部视作 confirmed
                }
                r
            }
            // 0x01 reportNumberOfDTCByStatusMask
            0x01 => vec![0x59, 0x01, 0xFF, (self.dtc_codes.len() as u8).wrapping_mul(4)],
            // 0x0A reportSupportedDTCs
            0x0A => {
                let mut r = vec![0x59, 0x0A, 0xFF];
                for code in &self.dtc_codes {
                    r.push((code >> 16) as u8);
                    r.push((code >> 8) as u8);
                    r.push(*code as u8);
                    r.push(0x08);
                }
                r
            }
            _ => negative(0x19, NrcCode::SubFunctionNotSupported),
        }
    }

    // ── 0x23 ReadMemoryByAddress ──
    fn svc_read_memory(&mut self, p: &[u8]) -> Vec<u8> {
        if p.is_empty() {
            return negative(0x23, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let fmt = p[0];
        let addr_len = (fmt & 0x0F) as usize;
        let len_len = ((fmt >> 4) & 0x0F) as usize;
        let mut idx = 1;
        if idx + addr_len + len_len > p.len() {
            return negative(0x23, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let mut addr = 0u32;
        for _ in 0..addr_len {
            addr = (addr << 8) | p[idx] as u32;
            idx += 1;
        }
        let mut len = 0u32;
        for _ in 0..len_len {
            len = (len << 8) | p[idx] as u32;
            idx += 1;
        }
        let mut data = Vec::with_capacity(len as usize);
        for i in 0..len {
            let b = self.memory.get(&(addr + i)).copied().unwrap_or(0xFF);
            data.push(b);
        }
        data
    }

    // ── 0x31 RoutineControl ──
    fn svc_routine(&mut self, p: &[u8]) -> Vec<u8> {
        if p.len() < 3 {
            return negative(0x31, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let sub = p[0];
        let rid = ((p[1] as u16) << 8) | (p[2] as u16);
        // 显式擦除例程 0xFF00：将下载区间填充为 0xFF（模拟 flash 擦除）
        if rid == 0xFF00 {
            if self.require_programming_session && self.session != SessionType::Programming {
                return negative(0x31, NrcCode::ConditionsNotCorrect);
            }
            let end = self
                .download_size
                .checked_add(self.download_base)
                .unwrap_or(self.download_base);
            for a in self.download_base..end {
                self.memory.insert(a, 0xFF);
            }
            return vec![0x71, sub, p[1], p[2], 0x00];
        }
        match sub {
            0x01 => {
                let mut r = vec![0x71, 0x01, p[1], p[2]];
                r.push(0x00); // 例程状态：成功
                r
            }
            0x02 => vec![0x71, 0x02, p[1], p[2], 0x00],
            0x03 => vec![0x71, 0x03, p[1], p[2], 0x00],
            _ => negative(0x31, NrcCode::SubFunctionNotSupported),
        }
    }

    // ── 0x34 RequestDownload（bootloader 支撑 + 压缩/加密协商）──
    fn svc_request_download(&mut self, p: &[u8]) -> Vec<u8> {
        if p.len() < 3 {
            return negative(0x34, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        if self.require_programming_session && self.session != SessionType::Programming {
            return negative(0x34, NrcCode::ConditionsNotCorrect);
        }
        if self.require_security_for_flash && self.unlocked_level.is_none() {
            return negative(0x34, NrcCode::SecurityAccessDenied);
        }
        // payload[0] = dataFormatIdentifier（高4位压缩方法，低4位加密方法）
        let data_format = p[0];
        let comp = (data_format >> 4) & 0x0F;
        let enc = data_format & 0x0F;
        // 仅支持 0x00（无压缩/无加密），其余协商拒绝
        if comp != 0 || enc != 0 {
            return negative(0x34, NrcCode::RequestOutOfRange);
        }
        self.compression_method = comp;
        self.encryption_method = enc;
        // payload[1] = lengthFormatIdentifier（高4位=size长度，低4位=addr长度）
        let fmt = p[1];
        let addr_len = (fmt & 0x0F) as usize;
        let size_len = ((fmt >> 4) & 0x0F) as usize;
        let mut idx = 2;
        if idx + addr_len + size_len > p.len() {
            return negative(0x34, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        let mut addr = 0u32;
        for _ in 0..addr_len {
            addr = (addr << 8) | p[idx] as u32;
            idx += 1;
        }
        let mut len = 0u32;
        for _ in 0..size_len {
            len = (len << 8) | p[idx] as u32;
            idx += 1;
        }
        self.download_base = addr;
        self.download_offset = 0;
        self.download_size = len;
        self.download_active = true;
        // 正响应：0x74 <lengthFormatId=0x20（maxBlockSize 2字节）> <maxBlockSize=0x1000>
        vec![0x74, 0x20, 0x10, 0x00]
    }

    // ── 0x36 TransferData（bootloader 支撑）──
    fn svc_transfer_data(&mut self, p: &[u8]) -> Vec<u8> {
        if p.is_empty() {
            return negative(0x36, NrcCode::IncorrectMessageLengthOrInvalidFormat);
        }
        if !self.download_active {
            return negative(0x36, NrcCode::RequestSequenceError);
        }
        let seq = p[0];
        let data = &p[1..];
        for (i, &b) in data.iter().enumerate() {
            self.memory
                .insert(self.download_base + self.download_offset + i as u32, b);
        }
        self.download_offset += data.len() as u32;
        vec![0x76, seq]
    }

    // ── 0x37 RequestTransferExit ──
    fn svc_transfer_exit(&mut self, _p: &[u8]) -> Vec<u8> {
        self.download_active = false;
        vec![0x77]
    }
}

/// 构造负响应：0x7F <service> <nrc>
fn negative(service: u8, nrc: NrcCode) -> Vec<u8> {
    vec![0x7F, service, nrc.code()]
}
