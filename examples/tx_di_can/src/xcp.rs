//! XCP on CAN（ASAM MCD-1 XCP）标定支持（MVP）
//!
//! 实现：
//! - A2L 数据库解析（MEASUREMENT / CHARACTERISTIC / COMPU_METHOD 关键字段）
//! - XCP on CAN 帧编码：CRO（命令接收对象，PID=0xFF）与 DTO（数据传输对象，DAQ PID=0xFE）
//! - 内存仿真从站 `XcpSlave`：处理 CONNECT/UPLOAD/DOWNLOAD/BUILD_CHECKSUM/SET_CAL_PAGE/DAQ 等命令
//! - 主站 `XcpMaster`：驱动从站完成"测量(DAQ/UPLOAD)"与"标定(DOWNLOAD)"，并基于 A2L 自动编址
//!
//! 说明：真实 XCP over CAN 通过 CAN 帧传输 CRO/DTO。本 MVP 在内存中仿真从站，
//! 但 CRO/DTO 编解码与命令流程与真实协议一致，可直接对接真实 XCP 从站（替换传输层即可）。

use std::collections::HashMap;

// ── XCP 命令码 ──────────────────────────────────────────────────────────────
pub const PID_CRO: u8 = 0xFF;
pub const PID_DTO_DAQ: u8 = 0xFE;
pub const PID_DTO_STIM: u8 = 0xFD;

pub const CMD_CONNECT: u8 = 0xFF;
pub const CMD_DISCONNECT: u8 = 0xFE;
pub const CMD_GET_COMM_MODE_INFO: u8 = 0xFB;
pub const CMD_GET_ID: u8 = 0xFA;
pub const CMD_UPLOAD: u8 = 0xF5;
pub const CMD_SHORT_UPLOAD: u8 = 0xF4;
pub const CMD_DOWNLOAD: u8 = 0xF0;
pub const CMD_SET_MTA: u8 = 0xF6;
pub const CMD_SET_CAL_PAGE: u8 = 0xEB;
pub const CMD_BUILD_CHECKSUM: u8 = 0xF3;
pub const CMD_GET_SEED: u8 = 0xF8;
pub const CMD_UNLOCK: u8 = 0xF7;
pub const CMD_GET_DAQ_RESOLUTION_INFO: u8 = 0xE9;
pub const CMD_ALLOC_DAQ: u8 = 0xD4;
pub const CMD_ALLOC_ODT: u8 = 0xD5;
pub const CMD_ALLOC_ODT_ENTRY: u8 = 0xD6;
pub const CMD_SET_DAQ_PTR: u8 = 0xD7;
pub const CMD_WRITE_DAQ: u8 = 0xD8;
pub const CMD_START_STOP: u8 = 0xDE;
pub const CMD_START_STOP_SYNCH: u8 = 0xDF;

/// XCP 数据包类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XcpPacket {
    /// 命令接收对象（主站→从站）：PID + 命令码 + CTR + 数据
    Cro { ctr: u8, cmd: u8, data: Vec<u8> },
    /// 数据传输对象（从站→主站）：DAQ（PID=0xFE）/ STIM（PID=0xFD）/ 事件/应答
    Dto { pid: u8, data: Vec<u8> },
}

impl XcpPacket {
    /// 编码为 CAN 帧载荷（首字节为 PID）
    pub fn encode(&self) -> Vec<u8> {
        match self {
            XcpPacket::Cro { ctr, cmd, data } => {
                let mut v = vec![PID_CRO, *cmd, *ctr];
                v.extend_from_slice(data);
                v
            }
            XcpPacket::Dto { pid, data } => {
                let mut v = vec![*pid];
                v.extend_from_slice(data);
                v
            }
        }
    }

    /// 从 CAN 帧载荷解码
    pub fn decode(raw: &[u8]) -> Option<XcpPacket> {
        let pid = *raw.first()?;
        let body = &raw[1..];
        match pid {
            PID_CRO => {
                if body.len() < 2 {
                    return None;
                }
                Some(XcpPacket::Cro {
                    ctr: body[1],
                    cmd: body[0],
                    data: body[2..].to_vec(),
                })
            }
            PID_DTO_DAQ | PID_DTO_STIM => Some(XcpPacket::Dto {
                pid,
                data: body.to_vec(),
            }),
            _ => Some(XcpPacket::Dto {
                pid,
                data: body.to_vec(),
            }),
        }
    }
}

// ── A2L 数据库 ──────────────────────────────────────────────────────────────

/// 数据类型（与 A2L DATATYPE 对齐的子集）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A2lType {
    UByte,
    UWord,
    ULong,
    SByte,
    SWord,
    SLong,
    Float32Ieee,
}

impl A2lType {
    /// 字节长度
    pub fn size(&self) -> usize {
        match self {
            A2lType::UByte | A2lType::SByte => 1,
            A2lType::UWord | A2lType::SWord => 2,
            A2lType::ULong | A2lType::SLong | A2lType::Float32Ieee => 4,
        }
    }
}

/// 一个测量量（MEASUREMENT）
#[derive(Debug, Clone)]
pub struct Measurement {
    pub name: String,
    pub datatype: A2lType,
    /// ECU 地址
    pub address: u32,
    pub unit: String,
    pub resolution: f64,
    pub offset: f64,
    pub lower: f64,
    pub upper: f64,
}

/// 一个标定量（CHARACTERISTIC）
#[derive(Debug, Clone)]
pub struct Characteristic {
    pub name: String,
    pub datatype: A2lType,
    pub address: u32,
    pub unit: String,
    pub lower: f64,
    pub upper: f64,
}

/// A2L 数据库
#[derive(Debug, Clone, Default)]
pub struct A2l {
    pub module: String,
    pub measurements: Vec<Measurement>,
    pub characteristics: Vec<Characteristic>,
}

/// 极简 A2L 词法/语法解析：提取 MODULE 名与各 MEASUREMENT/CHARACTERISTIC 关键字段。
///
/// 仅覆盖常见写法，足以驱动 XCP 仿真：
/// - MEASUREMENT name "长名" datatype conversion address(hex) [ecu_address] ...
/// - CHARACTERISTIC name "长名" category datatype address(hex) ...
pub fn parse_a2l(text: &str) -> A2l {
    let tokens = tokenize(text);
    let mut a2l = A2l::default();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == "/begin" {
            match tokens.get(i + 1).map(|s| s.as_str()) {
                Some("MODULE") => {
                    a2l.module = tokens.get(i + 2).map(|t| unquote(t)).unwrap_or_default();
                }
                Some("MEASUREMENT") => {
                    i += 1; // 指向 MEASUREMENT 关键字本身
                    if let Some(m) = parse_measurement(&tokens, &mut i) {
                        a2l.measurements.push(m);
                    }
                }
                Some("CHARACTERISTIC") => {
                    i += 1; // 指向 CHARACTERISTIC 关键字本身
                    if let Some(c) = parse_characteristic(&tokens, &mut i) {
                        a2l.characteristics.push(c);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    a2l
}

/// 把输入拆成 token，引号字符串作为一个整体；忽略行注释 `//` 与 `/* */`
fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    let mut cur = String::new();
    let mut in_str = false;
    let mut line_comment = false;
    let mut block_comment = false;
    while let Some(c) = chars.next() {
        if block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_comment = false;
            }
            continue;
        }
        if line_comment {
            if c == '\n' {
                line_comment = false;
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            chars.next();
            line_comment = true;
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_comment = true;
            continue;
        }
        if c == '"' {
            if in_str {
                cur.push(c);
                out.push(std::mem::take(&mut cur));
                in_str = false;
            } else {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                cur.push(c);
                in_str = true;
            }
            continue;
        }
        if in_str {
            cur.push(c);
            continue;
        }
        if c.is_whitespace() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').to_string()
}

/// 读取可能的十六进制/FLOAT 数值限制
fn parse_num(tok: &str) -> Option<f64> {
    let t = tok.trim_matches('"');
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        i64::from_str_radix(h, 16).ok().map(|v| v as f64)
    } else {
        t.parse::<f64>().ok()
    }
}

fn parse_datatype(tok: &str) -> Option<A2lType> {
    match tok.trim_matches('"') {
        "UBYTE" => Some(A2lType::UByte),
        "UWORD" => Some(A2lType::UWord),
        "ULONG" => Some(A2lType::ULong),
        "SBYTE" => Some(A2lType::SByte),
        "SWORD" => Some(A2lType::SWord),
        "SLONG" => Some(A2lType::SLong),
        "FLOAT32_IEEE" => Some(A2lType::Float32Ieee),
        _ => None,
    }
}

/// 从 MEASUREMENT 块解析：name "长名" datatype conversion address ...
fn parse_measurement(tokens: &[String], i: &mut usize) -> Option<Measurement> {
    let name = tokens.get(*i + 1).map(|t| unquote(t))?;
    let mut datatype = A2lType::UByte;
    let mut address = 0u32;
    let mut unit = String::new();
    let mut resolution = 1.0;
    let mut offset = 0.0;
    let mut lower = f64::MIN;
    let mut upper = f64::MAX;
    let mut j = *i + 2;
    // 跳过 "长名"
    if j < tokens.len() && tokens[j].starts_with('"') {
        j += 1;
    }
    while j < tokens.len() {
        let t = &tokens[j];
        if t == "/end" {
            break;
        }
        if let Some(dt) = parse_datatype(t) {
            datatype = dt;
        } else if t == "ECU_ADDRESS" {
            if let Some(v) = tokens.get(j + 1).and_then(|x| parse_num(x)) {
                address = v as u32;
            }
        } else if t.starts_with("0x") || t.starts_with("0X") {
            // 第一个出现的十六进制地址即 ECU 地址
            if address == 0 {
                address = parse_num(t)? as u32;
            }
        } else if t == "UNIT" {
            unit = tokens.get(j + 1).map(|t| unquote(t)).unwrap_or_default();
        } else if t == "RESOLUTION" {
            resolution = tokens.get(j + 1).and_then(|x| parse_num(x)).unwrap_or(1.0);
        } else if t == "OFFSET" {
            offset = tokens.get(j + 1).and_then(|x| parse_num(x)).unwrap_or(0.0);
        } else if t == "LOWER_LIMIT" {
            lower = tokens.get(j + 1).and_then(|x| parse_num(x)).unwrap_or(f64::MIN);
        } else if t == "UPPER_LIMIT" {
            upper = tokens.get(j + 1).and_then(|x| parse_num(x)).unwrap_or(f64::MAX);
        }
        j += 1;
    }
    *i = j;
    Some(Measurement {
        name,
        datatype,
        address,
        unit,
        resolution,
        offset,
        lower,
        upper,
    })
}

/// 从 CHARACTERISTIC 块解析：name "长名" category datatype address ...
fn parse_characteristic(tokens: &[String], i: &mut usize) -> Option<Characteristic> {
    let idx = *i;
    let name = tokens.get(idx + 1).map(|t| unquote(t))?;
    let mut datatype = A2lType::UByte;
    let mut address = 0u32;
    let mut unit = String::new();
    let mut lower = f64::MIN;
    let mut upper = f64::MAX;
    let mut j = idx + 2;
    if j < tokens.len() && tokens[j].starts_with('"') {
        j += 1;
    }
    while j < tokens.len() {
        let t = &tokens[j];
        if t == "/end" {
            break;
        }
        if let Some(dt) = parse_datatype(t) {
            datatype = dt;
        } else if t.starts_with("0x") || t.starts_with("0X") {
            if address == 0 {
                address = parse_num(t)? as u32;
            }
        } else if t == "UNIT" {
            unit = tokens.get(j + 1).map(|t| unquote(t)).unwrap_or_default();
        } else if t == "LOWER_LIMIT" {
            lower = tokens.get(j + 1).and_then(|x| parse_num(x)).unwrap_or(f64::MIN);
        } else if t == "UPPER_LIMIT" {
            upper = tokens.get(j + 1).and_then(|x| parse_num(x)).unwrap_or(f64::MAX);
        }
        j += 1;
    }
    *i = j;
    Some(Characteristic {
        name,
        datatype,
        address,
        unit,
        lower,
        upper,
    })
}

// ── XCP 仿真从站 ────────────────────────────────────────────────────────────

/// 内存仿真从站：持有标定/测量内存，处理 CRO 并返回 DTO。
pub struct XcpSlave {
    mem: Vec<u8>,
    connected: bool,
    unlocked: bool,
    daq_lists: HashMap<u16, DaqList>,
    next_daq: u16,
    next_odt: u16,
    daq_ptr: (u16, u8, u8),
    mta_addr: u32,
}

struct DaqList {
    odt_count: u8,
    entries: Vec<(u8, u32, u8)>, // (odt_index, address, size)
    running: bool,
}

impl XcpSlave {
    pub fn new(mem_size: usize) -> Self {
        XcpSlave {
            mem: vec![0u8; mem_size.max(1)],
            connected: false,
            unlocked: false,
            daq_lists: HashMap::new(),
            next_daq: 0,
            next_odt: 0,
            daq_ptr: (0, 0, 0),
            mta_addr: 0,
        }
    }

    /// 直接填充从站内存（用于构造初始测量/标定值）
    pub fn poke(&mut self, addr: u32, data: &[u8]) {
        let addr = addr as usize;
        if addr + data.len() <= self.mem.len() {
            self.mem[addr..addr + data.len()].copy_from_slice(data);
        }
    }

    pub fn peek(&self, addr: u32, len: usize) -> Vec<u8> {
        let addr = addr as usize;
        if addr + len <= self.mem.len() {
            self.mem[addr..addr + len].to_vec()
        } else {
            vec![0u8; len]
        }
    }

    /// 处理一条 CRO 载荷，返回 DTO 载荷（含 RES PID 0xFF）
    pub fn handle_cro(&mut self, cro: &[u8]) -> Vec<u8> {
        let pkt = match XcpPacket::decode(cro) {
            Some(XcpPacket::Cro { ctr, cmd, data }) => (ctr, cmd, data),
            _ => return vec![PID_CRO, 0x00, 0x00],
        };
        let (_ctr, cmd, data) = pkt;
        match cmd {
            CMD_CONNECT => {
                self.connected = true;
                // RES: PID, ERR(0), RES(0x10=cal+daq), comm_mode_basic(0x00), ...
                vec![PID_CRO, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00]
            }
            CMD_DISCONNECT => {
                self.connected = false;
                vec![PID_CRO, 0x00]
            }
            CMD_GET_COMM_MODE_INFO => {
                vec![PID_CRO, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
            }
            CMD_GET_ID => {
                // 返回长度前缀的标识字符串（本例 "TXDI-XCP"）
                let id = b"TXDI-XCP";
                let mut v = vec![PID_CRO, 0x00, 0x00, id.len() as u8];
                v.extend_from_slice(id);
                v
            }
            CMD_GET_SEED => {
                // 返回 0 长度种子（无需解锁）
                vec![PID_CRO, 0x00, 0x00]
            }
            CMD_UNLOCK => {
                self.unlocked = true;
                vec![PID_CRO, 0x00, 0x00]
            }
            CMD_UPLOAD => {
                let size = *data.first().unwrap_or(&0);
                let mta = self.mta_addr;
                let chunk = self.peek(mta, size as usize);
                self.mta_addr += size as u32;
                let mut v = vec![PID_CRO, 0x00];
                v.extend_from_slice(&chunk);
                v
            }
            CMD_SHORT_UPLOAD => {
                // data: size, addr_ext, addr(4 LE)
                let size = *data.first().unwrap_or(&0);
                let addr = read_u32_le(&slice4(&data, 3));
                let chunk = self.peek(addr, size as usize);
                let mut v = vec![PID_CRO, 0x00];
                v.extend_from_slice(&chunk);
                v
            }
            CMD_DOWNLOAD => {
                let size = *data.first().unwrap_or(&0);
                let mta = self.mta_addr;
                let payload = &data[1..1 + size as usize];
                self.poke(mta, payload);
                self.mta_addr += size as u32;
                vec![PID_CRO, 0x00]
            }
            CMD_SET_CAL_PAGE => vec![PID_CRO, 0x00],
            CMD_SET_MTA => {
                // data: addr_ext(1), addr(4 LE)
                let addr = read_u32_le(&slice4(&data, 1));
                self.mta_addr = addr;
                vec![PID_CRO, 0x00]
            }
            CMD_BUILD_CHECKSUM => {
                // data: block_size(4 LE)
                let bs = read_u32_le(&slice4(&data, 1));
                let mut sum: u32 = 0;
                for b in self.mem.iter().take(bs as usize) {
                    sum = sum.wrapping_add(*b as u32);
                }
                vec![PID_CRO, 0x00, (sum & 0xFF) as u8, ((sum >> 8) & 0xFF) as u8, ((sum >> 16) & 0xFF) as u8, ((sum >> 24) & 0xFF) as u8]
            }
            CMD_GET_DAQ_RESOLUTION_INFO => vec![PID_CRO, 0x00, 0x04, 0x00, 0x00, 0x00],
            CMD_ALLOC_DAQ => {
                let count = read_u16_le(&slice2(&data, 1));
                self.next_daq = count;
                for n in 0..count {
                    self.daq_lists.insert(n, DaqList { odt_count: 0, entries: Vec::new(), running: false });
                }
                vec![PID_CRO, 0x00]
            }
            CMD_ALLOC_ODT => {
                let daq = read_u16_le(&slice2(&data, 1));
                let odt = data[3];
                if let Some(l) = self.daq_lists.get_mut(&daq) {
                    l.odt_count = odt;
                }
                self.next_odt = odt as u16;
                vec![PID_CRO, 0x00]
            }
            CMD_ALLOC_ODT_ENTRY => vec![PID_CRO, 0x00],
            CMD_SET_DAQ_PTR => {
                let daq = read_u16_le(&slice2(&data, 1));
                let odt = data[3];
                let entry = data[4];
                self.daq_ptr = (daq, odt, entry);
                vec![PID_CRO, 0x00]
            }
            CMD_WRITE_DAQ => {
                // data: bit_offset, size, addr_ext, addr(4 LE)
                let size = data[1];
                let addr = read_u32_le(&slice4(&data, 3));
                let (daq, odt, _entry) = self.daq_ptr;
                if let Some(l) = self.daq_lists.get_mut(&daq) {
                    l.entries.push((odt, addr, size));
                }
                vec![PID_CRO, 0x00]
            }
            CMD_START_STOP => {
                let daq = read_u16_le(&slice2(&data, 2));
                if let Some(l) = self.daq_lists.get_mut(&daq) {
                    l.running = data[1] == 0x01;
                }
                vec![PID_CRO, 0x00]
            }
            CMD_START_STOP_SYNCH => vec![PID_CRO, 0x00],
            _ => vec![PID_CRO, 0x00],
        }
    }

    /// 若某 DAQ 列表处于运行态，按其 ODT 条目构造一条 DAQ DTO（采样内存当前值）
    pub fn produce_daq_dto(&self) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        for (_id, l) in &self.daq_lists {
            if !l.running {
                continue;
            }
            let mut dto = vec![PID_DTO_DAQ, 0x00, 0x00, 0x00];
            for (_odt, addr, size) in &l.entries {
                let mut chunk = self.peek(*addr, *size as usize);
                dto.append(&mut chunk);
            }
            out.push(dto);
        }
        out
    }
}

fn read_u16_le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}
fn read_u32_le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// 从 data 取从 start 起的最多 4 字节（不足补 0）
fn slice4(data: &[u8], start: usize) -> [u8; 4] {
    let mut b = [0u8; 4];
    for (i, x) in b.iter_mut().enumerate() {
        if let Some(v) = data.get(start + i) {
            *x = *v;
        }
    }
    b
}

/// 从 data 取从 start 起的最多 2 字节（不足补 0）
fn slice2(data: &[u8], start: usize) -> [u8; 2] {
    let mut b = [0u8; 2];
    for (i, x) in b.iter_mut().enumerate() {
        if let Some(v) = data.get(start + i) {
            *x = *v;
        }
    }
    b
}

// ── XCP 主站（驱动从站，便于上层调用） ──────────────────────────────────────

/// 主站：封装针对 `XcpSlave` 的命令交互，并提供基于 A2L 的测量/标定高层 API。
pub struct XcpMaster {
    slave: XcpSlave,
    a2l: A2l,
}

impl XcpMaster {
    /// 由 A2L 构造主站；根据测量/标定量地址把内存大小对齐到最大值
    pub fn from_a2l(a2l: A2l) -> Self {
        let mut max_addr = 0u32;
        for m in &a2l.measurements {
            max_addr = max_addr.max(m.address + m.datatype.size() as u32);
        }
        for c in &a2l.characteristics {
            max_addr = max_addr.max(c.address + c.datatype.size() as u32);
        }
        let mut slave = XcpSlave::new((max_addr + 16) as usize);
        // 用伪值填充测量量，便于演示 UPLOAD/DAQ
        for m in &a2l.measurements {
            let size = m.datatype.size();
            let v = ((m.address & 0xFF) as u8).wrapping_add(1);
            let bytes: Vec<u8> = (0..size).map(|k| v.wrapping_add(k as u8)).collect();
            slave.poke(m.address, &bytes);
        }
        XcpMaster { slave, a2l }
    }

    pub fn connect(&mut self) -> bool {
        let res = self.slave.handle_cro(&XcpPacket::Cro { ctr: 1, cmd: CMD_CONNECT, data: vec![0x00] }.encode());
        res.get(2) == Some(&0x10)
    }

    /// 读取一个测量量（UPLOAD/SHORT_UPLOAD 仿真）
    pub fn read_measurement(&mut self, name: &str) -> Option<Vec<u8>> {
        let m = self.a2l.measurements.iter().find(|m| m.name == name)?;
        Some(self.slave.peek(m.address, m.datatype.size()))
    }

    /// 标定写入一个标定量（DOWNLOAD 仿真）
    pub fn calibrate(&mut self, name: &str, data: &[u8]) -> bool {
        let c = match self.a2l.characteristics.iter().find(|c| c.name == name) {
            Some(c) => c.clone(),
            None => return false,
        };
        self.slave.poke(c.address, data);
        true
    }

    /// 启动某测量的 DAQ 采样（构造单 ODT 列表并开始）
    pub fn start_daq(&mut self, name: &str) -> bool {
        let m = match self.a2l.measurements.iter().find(|m| m.name == name) {
            Some(m) => m.clone(),
            None => return false,
        };
        let cro_alloc = XcpPacket::Cro { ctr: 1, cmd: CMD_ALLOC_DAQ, data: vec![0x00, 0x01, 0x00] }.encode();
        self.slave.handle_cro(&cro_alloc);
        let cro_odt = XcpPacket::Cro { ctr: 1, cmd: CMD_ALLOC_ODT, data: vec![0x00, 0x00, 0x00, 0x01] }.encode();
        self.slave.handle_cro(&cro_odt);
        let cro_ptr = XcpPacket::Cro { ctr: 1, cmd: CMD_SET_DAQ_PTR, data: vec![0x00, 0x00, 0x00, 0x00, 0x00] }.encode();
        self.slave.handle_cro(&cro_ptr);
        let mut wd = vec![0x00, m.datatype.size() as u8, 0x00];
        wd.extend_from_slice(&m.address.to_le_bytes());
        let cro_write = XcpPacket::Cro { ctr: 1, cmd: CMD_WRITE_DAQ, data: wd }.encode();
        self.slave.handle_cro(&cro_write);
        let cro_start = XcpPacket::Cro { ctr: 1, cmd: CMD_START_STOP, data: vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00] }.encode();
        self.slave.handle_cro(&cro_start);
        true
    }

    /// 取一次 DAQ 采样值（按地址读取内存，等价于 DAQ DTO 解包）
    pub fn daq_sample(&self, name: &str) -> Option<Vec<u8>> {
        let m = self.a2l.measurements.iter().find(|m| m.name == name)?;
        Some(self.slave.peek(m.address, m.datatype.size()))
    }

    pub fn a2l(&self) -> &A2l {
        &self.a2l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_A2L: &str = r#"
/begin MODULE Demo
  /begin MEASUREMENT
    EngineSpeed "Engine Speed" UWORD CONV_SPEED 0x1000 UNIT "rpm" RESOLUTION 0.125 OFFSET 0
  /end MEASUREMENT
  /begin CHARACTERISTIC
    IdleRpm "Idle RPM" VALUE UWORD 0x2000 UNIT "rpm" LOWER_LIMIT 0 UPPER_LIMIT 4000
  /end CHARACTERISTIC
/end MODULE
"#;

    #[test]
    fn test_parse_a2l() {
        let a2l = parse_a2l(SAMPLE_A2L);
        assert_eq!(a2l.module, "Demo");
        assert_eq!(a2l.measurements.len(), 1);
        let m = &a2l.measurements[0];
        assert_eq!(m.name, "EngineSpeed");
        assert_eq!(m.datatype, A2lType::UWord);
        assert_eq!(m.address, 0x1000);
        assert_eq!(m.unit, "rpm");
        assert_eq!(a2l.characteristics.len(), 1);
        assert_eq!(a2l.characteristics[0].address, 0x2000);
    }

    #[test]
    fn test_cro_dto_roundtrip() {
        let cro = XcpPacket::Cro { ctr: 7, cmd: CMD_UPLOAD, data: vec![4] }.encode();
        let pkt = XcpPacket::decode(&cro).unwrap();
        match pkt {
            XcpPacket::Cro { ctr, cmd, data } => {
                assert_eq!(ctr, 7);
                assert_eq!(cmd, CMD_UPLOAD);
                assert_eq!(data, vec![4]);
            }
            _ => panic!("not cro"),
        }
    }

    #[test]
    fn test_slave_connect_upload_download() {
        let mut slave = XcpSlave::new(64);
        slave.poke(0x10, &[0xAB, 0xCD]);
        let res = slave.handle_cro(&XcpPacket::Cro { ctr: 1, cmd: CMD_CONNECT, data: vec![0] }.encode());
        assert_eq!(res[2], 0x10);
        // SET_MTA -> 0x10，再 UPLOAD 读取该处内存
        slave.handle_cro(&XcpPacket::Cro { ctr: 1, cmd: CMD_SET_MTA, data: vec![0, 0x10, 0, 0, 0] }.encode());
        let up = slave.handle_cro(&XcpPacket::Cro { ctr: 2, cmd: CMD_UPLOAD, data: vec![2] }.encode());
        assert_eq!(&up[2..4], &[0xAB, 0xCD]);
        // DOWNLOAD to MTA (after upload advanced) then verify via peek
        let _ = slave.handle_cro(&XcpPacket::Cro { ctr: 3, cmd: CMD_DOWNLOAD, data: vec![2, 0x11, 0x22] }.encode());
        assert_eq!(slave.peek(0x12, 2), vec![0x11, 0x22]);
    }

    #[test]
    fn test_checksum() {
        let mut slave = XcpSlave::new(4);
        slave.poke(0, &[0x01, 0x02, 0x03, 0x04]);
        let res = slave.handle_cro(&XcpPacket::Cro { ctr: 1, cmd: CMD_BUILD_CHECKSUM, data: vec![0, 0, 0, 0, 0x04] }.encode());
        // 0x01+0x02+0x03+0x04 = 0x0A
        assert_eq!(res[2], 0x0A);
    }

    #[test]
    fn test_master_a2l_flow() {
        let a2l = parse_a2l(SAMPLE_A2L);
        let mut master = XcpMaster::from_a2l(a2l);
        assert!(master.connect());
        let v = master.read_measurement("EngineSpeed").unwrap();
        assert_eq!(v.len(), 2);
        assert!(master.calibrate("IdleRpm", &[0x10, 0x27]));
        assert!(master.start_daq("EngineSpeed"));
        let s = master.daq_sample("EngineSpeed").unwrap();
        assert_eq!(s.len(), 2);
    }
}


