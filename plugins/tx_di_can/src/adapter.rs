//! CAN 适配器抽象层
//!
//! 支持三种适配器：
//! - **SimBus**：仿真回环，适用于开发/测试/无硬件环境
//! - **SocketCAN**：Linux 原生 CAN，使用 libc PF_CAN
//! - **PCAN**：PEAK PCAN USB 设备（Windows），需要 `features = ["pcan"]`

pub use crate::config::AdapterKind;
use crate::frame::{CanFdFrame, CanFrame};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use crate::adapter::pcan_impl::get_pcan;
// ─────────────────────────────────────────────────────────────────────────────
// Trait 定义
// ─────────────────────────────────────────────────────────────────────────────

/// CAN 适配器统一接口
#[async_trait]
pub trait CanAdapter: Send + Sync {
    /// 打开总线
    async fn open(&self) -> Result<()>;
    /// 关闭总线
    async fn close(&self) -> Result<()>;
    /// 发送标准 CAN 帧
    async fn send(&self, frame: &CanFrame) -> Result<()>;
    /// 发送 CANFD 帧（若硬件不支持返回 Err）
    async fn send_fd(&self, frame: &CanFdFrame) -> Result<()>;
    /// 订阅接收通道（接收端持有 receiver，适配器内部推帧）
    fn subscribe(&self) -> broadcast::Receiver<CanFrame>;
    /// 订阅 CANFD 接收通道
    fn subscribe_fd(&self) -> broadcast::Receiver<CanFdFrame>;
    /// 适配器名称（日志用）
    fn name(&self) -> &str;
}

// ─────────────────────────────────────────────────────────────────────────────
// SimBus（仿真总线）：所有发出的帧立即回环到接收通道
// 适用于：单元测试 / 无 CAN 硬件的开发环境
// ─────────────────────────────────────────────────────────────────────────────

pub struct SimBusAdapter {
    name: String,
    tx: broadcast::Sender<CanFrame>,
    fd_tx: broadcast::Sender<CanFdFrame>,
}

impl SimBusAdapter {
    pub fn new(name: impl Into<String>, queue_size: usize) -> Self {
        let (tx, _) = broadcast::channel(queue_size);
        let (fd_tx, _) = broadcast::channel(queue_size);
        SimBusAdapter {
            name: name.into(),
            tx,
            fd_tx,
        }
    }
}

#[async_trait]
impl CanAdapter for SimBusAdapter {
    async fn open(&self) -> Result<()> {
        tracing::info!("[simbus] 仿真总线 '{}' 已就绪", self.name);
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }

    async fn send(&self, frame: &CanFrame) -> Result<()> {
        let _ = self.tx.send(frame.clone());
        Ok(())
    }

    async fn send_fd(&self, frame: &CanFdFrame) -> Result<()> {
        let _ = self.fd_tx.send(frame.clone());
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<CanFrame> {
        self.tx.subscribe()
    }

    fn subscribe_fd(&self) -> broadcast::Receiver<CanFdFrame> {
        self.fd_tx.subscribe()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SocketCAN 适配器（Linux only）
// 使用 Linux libc 直接操作 PF_CAN socket，无第三方依赖
// ─────────────────────────────────────────────────────────────────────────────

pub struct SocketCanAdapter {
    interface: String,
    #[cfg(target_os = "linux")]
    sock_fd: std::sync::Mutex<Option<libc::c_int>>,
    #[cfg(not(target_os = "linux"))]
    _sock_fd: std::sync::Mutex<Option<isize>>,
    tx: broadcast::Sender<CanFrame>,
    fd_tx: broadcast::Sender<CanFdFrame>,
    tasks: std::sync::Mutex<JoinSet<()>>,
    running: std::sync::Mutex<bool>,
}

impl SocketCanAdapter {
    pub fn new(interface: impl Into<String>, queue_size: usize) -> Self {
        let (tx, _) = broadcast::channel(queue_size);
        let (fd_tx, _) = broadcast::channel(queue_size);
        SocketCanAdapter {
            interface: interface.into(),
            #[cfg(target_os = "linux")]
            sock_fd: std::sync::Mutex::new(None),
            #[cfg(not(target_os = "linux"))]
            _sock_fd: std::sync::Mutex::new(None),
            tx,
            fd_tx,
            tasks: std::sync::Mutex::new(JoinSet::new()),
            running: std::sync::Mutex::new(false),
        }
    }

    #[cfg(target_os = "linux")]
    fn get_raw_fd(&self) -> Option<libc::c_int> {
        *self.sock_fd.lock().unwrap()
    }

    #[cfg(not(target_os = "linux"))]
    #[allow(dead_code)]
    pub(crate) fn get_raw_fd(&self) -> Option<isize> {
        None
    }
}

/// Linux CAN socket frame (struct can_frame from <linux/can.h>)
#[cfg(target_os = "linux")]
#[repr(C)]
struct CanFrameLinux {
    pub can_id: u32,
    pub can_dlc: u8,
    pub __pad: u8,
    pub __res0: u8,
    pub __res1: u8,
    pub data: [u8; 8],
}

/// Linux CANFD frame (struct canfd_frame from <linux/can.h>)
#[cfg(target_os = "linux")]
#[repr(C)]
struct CanFdFrameLinux {
    pub can_id: u32,
    pub len: u8,
    pub flags: u8,
    pub __res0: u8,
    pub __res1: u8,
    pub data: [u8; 64],
}

/// sockaddr_can (from <linux/can.h>)
#[cfg(target_os = "linux")]
#[repr(C)]
struct SockAddrCan {
    pub can_family: libc::c_short,
    pub can_ifindex: libc::c_int,
    pub __padding: [u8; 12],
}

#[cfg(target_os = "linux")]
fn make_linux_can_frame(frame: &CanFrame) -> CanFrameLinux {
    let dlc = frame.data.len().min(8) as u8;
    let mut cf = CanFrameLinux {
        can_id: frame.id.raw(),
        can_dlc: dlc,
        __pad: 0,
        __res0: 0,
        __res1: 0,
        data: [0u8; 8],
    };
    cf.data[..frame.data.len().min(8)].copy_from_slice(&frame.data[..frame.data.len().min(8)]);
    cf
}

#[cfg(target_os = "linux")]
fn make_linux_fd_frame(frame: &CanFdFrame) -> CanFdFrameLinux {
    let len = frame.data.len().min(64) as u8;
    let flags =
        0x04u8 // CANFD flag
        | if frame.brs { 0x01 } else { 0 }
        | if frame.esi { 0x02 } else { 0 };
    let mut cf = CanFdFrameLinux {
        can_id: frame.id.raw(),
        len,
        flags,
        __res0: 0,
        __res1: 0,
        data: [0u8; 64],
    };
    cf.data[..frame.data.len().min(64)].copy_from_slice(&frame.data[..frame.data.len().min(64)]);
    cf
}

#[async_trait]
impl CanAdapter for SocketCanAdapter {
    async fn open(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::ptr;

            // 创建 PF_CAN socket
            let sock = unsafe { libc::socket(29, 3, 1) }; // PF_CAN, SOCK_RAW, CAN_RAW
            if sock < 0 {
                return Err(anyhow::anyhow!(
                    "SocketCAN: socket() 失败: {}",
                    std::io::Error::last_os_error()
                ));
            }

            // 获取接口索引
            let ifname = self.interface.as_bytes();
            let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };
            let namelen = ifname.len().min(libc::IFNAMSIZ - 1);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    ifname.as_ptr(),
                    ifr.ifr_name.as_mut_ptr() as *mut u8,
                    namelen,
                );
            }

            let ret = unsafe {
                libc::ioctl(sock, libc::SIOCGIFINDEX as _, ptr::addr_of_mut!(ifr))
            };
            if ret < 0 {
                let err = std::io::Error::last_os_error();
                unsafe { libc::close(sock) };
                return Err(anyhow::anyhow!(
                    "SocketCAN: ioctl(SIOCGIFINDEX) 获取 '{}' 失败: {err}",
                    self.interface
                ));
            }

            let ifindex = unsafe { ifr.ifr_ifindex };

            // 绑定 socket 到 CAN 接口
            let mut addr: SockAddrCan = unsafe { std::mem::zeroed() };
            addr.can_family = 29i16 as libc::c_short;
            addr.can_ifindex = ifindex;

            let bind_ret = unsafe {
                libc::bind(
                    sock,
                    ptr::addr_of!(addr) as *const _,
                    std::mem::size_of::<SockAddrCan>() as libc::socklen_t,
                )
            };
            if bind_ret < 0 {
                let err = std::io::Error::last_os_error();
                unsafe { libc::close(sock) };
                return Err(anyhow::anyhow!(
                    "SocketCAN: bind 到 '{}' 失败: {err}",
                    self.interface
                ));
            }

            // 设置接收超时（1秒）
            let timeout = libc::timeval {
                tv_sec: 1,
                tv_usec: 0,
            };
            unsafe {
                libc::setsockopt(
                    sock,
                    libc::SOL_SOCKET,
                    libc::SO_RCVTIMEO,
                    ptr::addr_of!(timeout) as *const _,
                    std::mem::size_of::<libc::timeval>() as libc::socklen_t,
                );
            }

            // 设置发送超时（100ms）
            let send_timeout = libc::timeval {
                tv_sec: 0,
                tv_usec: 100_000,
            };
            unsafe {
                libc::setsockopt(
                    sock,
                    libc::SOL_SOCKET,
                    libc::SO_SNDTIMEO,
                    ptr::addr_of!(send_timeout) as *const _,
                    std::mem::size_of::<libc::timeval>() as libc::socklen_t,
                );
            }

            *self.sock_fd.lock().unwrap() = Some(sock);
            tracing::info!(
                "[socketcan] 接口 '{}' 已打开 (ifindex={}, fd={})",
                self.interface,
                ifindex,
                sock
            );
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(anyhow::anyhow!(
                "SocketCAN 仅支持 Linux。当前平台请使用 simbus。"
            ))
        }
    }

    async fn close(&self) -> Result<()> {
        {
            let mut running = self.running.lock().unwrap();
            *running = false;
        }

        // 取出 JoinSet 并 abort 所有任务
        let mut tasks = std::mem::take(&mut *self.tasks.lock().unwrap());
        tasks.abort_all();

        #[cfg(target_os = "linux")]
        {
            if let Some(fd) = self.get_raw_fd() {
                unsafe { libc::close(fd) };
            }
            *self.sock_fd.lock().unwrap() = None;
        }

        tracing::info!("[socketcan] 接口 '{}' 已关闭", self.interface);
        Ok(())
    }

    async fn send(&self, _frame: &CanFrame) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::ptr;

            let fd = match self.get_raw_fd() {
                Some(f) => f,
                None => return Err(anyhow::anyhow!("SocketCAN 未 open()")),
            };

            let can_frame = make_linux_can_frame(frame);
            let mut addr: SockAddrCan = unsafe { std::mem::zeroed() };
            addr.can_family = 29i16 as libc::c_short;

            let ret = unsafe {
                libc::sendto(
                    fd,
                    ptr::addr_of!(can_frame) as *const _,
                    std::mem::size_of::<CanFrameLinux>(),
                    0,
                    ptr::addr_of!(addr) as *const _,
                    std::mem::size_of::<SockAddrCan>() as libc::socklen_t,
                )
            };

            if ret < 0 {
                return Err(anyhow::anyhow!(
                    "SocketCAN TX 失败: {}",
                    std::io::Error::last_os_error()
                ));
            }

            tracing::trace!(
                "[socketcan] TX {:03X} [{:02X}]",
                frame.id.raw(),
                frame.data.len()
            );
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        Err(anyhow::anyhow!("SocketCAN TX 不支持当前平台"))
    }

    async fn send_fd(&self, _frame: &CanFdFrame) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::ptr;

            let fd = match self.get_raw_fd() {
                Some(f) => f,
                None => return Err(anyhow::anyhow!("SocketCAN 未 open()")),
            };

            // 启用 CAN_RAW_FD_FRAMES (33)
            let enable_fd: libc::c_int = 1;
            unsafe {
                libc::setsockopt(
                    fd,
                    29, // SOL_CAN_RAW
                    33, // CAN_RAW_FD_FRAMES
                    ptr::addr_of!(enable_fd) as *const _,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                );
            }

            let can_fd_frame = make_linux_fd_frame(frame);
            let mut addr: SockAddrCan = unsafe { std::mem::zeroed() };
            addr.can_family = 29i16 as libc::c_short;

            let ret = unsafe {
                libc::sendto(
                    fd,
                    ptr::addr_of!(can_fd_frame) as *const _,
                    std::mem::size_of::<CanFdFrameLinux>(),
                    0,
                    ptr::addr_of!(addr) as *const _,
                    std::mem::size_of::<SockAddrCan>() as libc::socklen_t,
                )
            };

            if ret < 0 {
                return Err(anyhow::anyhow!(
                    "SocketCAN FD TX 失败: {}",
                    std::io::Error::last_os_error()
                ));
            }
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        Err(anyhow::anyhow!("SocketCAN FD TX 不支持当前平台"))
    }

    fn subscribe(&self) -> broadcast::Receiver<CanFrame> {
        self.tx.subscribe()
    }

    fn subscribe_fd(&self) -> broadcast::Receiver<CanFdFrame> {
        self.fd_tx.subscribe()
    }

    fn name(&self) -> &str {
        &self.interface
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PCAN 适配器（Windows PEAK PCAN USB，使用 windows crate FFI）
// 通过动态加载 pcanbasic.dll 实现
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(all(windows, feature = "pcan"))]
mod pcan_impl {
    use super::*;
    use std::ffi::CString;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    // PCAN 通道常量
    pub const PCAN_USBBUS1: u32 = 0x51;
    pub const PCAN_USBBUS2: u32 = 0x52;
    pub const PCAN_USBBUS3: u32 = 0x53;
    pub const PCAN_USBBUS4: u32 = 0x54;
    pub const PCAN_USBBUS5: u32 = 0x55;
    pub const PCAN_USBBUS6: u32 = 0x56;
    pub const PCAN_USBBUS7: u32 = 0x57;
    pub const PCAN_USBBUS8: u32 = 0x58;
    pub const PCAN_USBPROFDBUS1: u32 = 0x131;
    pub const PCAN_USBPROFDBUS2: u32 = 0x132;
    pub const PCAN_PCIBUS1: u32 = 0x41;
    pub const PCAN_PCIBUS2: u32 = 0x42;
    pub const PCAN_NONEBUS: u32 = 0x00;

    // 波特率（BTR0BTR1）
    pub const PCAN_BAUD_1M: u16 = 0x0014;
    pub const PCAN_BAUD_500K: u16 = 0x001C;
    pub const PCAN_BAUD_250K: u16 = 0x011C;
    pub const PCAN_BAUD_125K: u16 = 0x031C;
    pub const PCAN_ERROR_OK: u32 = 0x00000;

    // 消息类型
    pub const PCAN_MESSAGE_STANDARD: u32 = 0x000;
    pub const PCAN_MESSAGE_FD: u32 = 0x04;
    pub const PCAN_MESSAGE_BRS: u32 = 0x08;
    pub const PCAN_MESSAGE_ESI: u32 = 0x10;

    /// PCAN 句柄
    pub type PcanHandle = isize;

    /// PCAN 消息（与 C TPCANMsg 对齐）
    #[repr(C)]
    pub struct PcanMsg {
        pub id: u32,
        pub msg_type: u32,
        pub len: u8,
        pub data: [u8; 64],
    }

    impl PcanMsg {
        pub fn new(id: u32, msg_type: u32, len: u8, data: &[u8]) -> Self {
            let mut msg = PcanMsg {
                id,
                msg_type,
                len,
                data: [0u8; 64],
            };
            let n = len.min(64) as usize;
            msg.data[..n].copy_from_slice(&data[..n]);
            msg
        }
    }

    // FFI 函数指针
    type PcanOpen = unsafe extern "system" fn(channel: u32) -> PcanHandle;
    type PcanClose = unsafe extern "system" fn(handle: PcanHandle) -> u32;
    type PcanInit = unsafe extern "system" fn(handle: PcanHandle, btr0btr1: u16, hwtype: u32) -> u32;
    type PcanWrite = unsafe extern "system" fn(handle: PcanHandle, msg: *const PcanMsg) -> u32;

    /// PCAN DLL 加载器
    pub struct PcanDll {
        pub CAN_Open: PcanOpen,
        pub CAN_Close: PcanClose,
        pub CAN_Init: PcanInit,
        pub CAN_Write: PcanWrite,
    }

    impl PcanDll {
        pub fn load() -> anyhow::Result<Self> {
            use windows::core::PCWSTR;
            use std::ptr;

            let dll_paths = [
                "pcanbasic.dll",
                "C:\\Program Files\\PEAK-System\\PCAN-Basic\\pcanbasic.dll",
                "C:\\Program Files (x86)\\PEAK-System\\PCAN-Basic\\pcanbasic.dll",
            ];

            let mut dll: Option<HMODULE> = None;
            for path in &dll_paths {
                let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
                let h = unsafe { LoadLibraryW(PCWSTR(wide.as_ptr())) };
                if h.is_ok() {
                    dll = Some(h.unwrap());
                    tracing::info!("[pcan] 已加载 DLL: {}", path);
                    break;
                }
            }

            let dll = dll.ok_or_else(|| {
                anyhow::anyhow!("未找到 pcanbasic.dll。请安装 PEAK PCAN-Basic SDK。")
            })?;

            let get_sym = |name: &str| -> anyhow::Result<unsafe extern "system" fn() -> isize> {
                let cname = CString::new(name).unwrap();
                let sym = unsafe {
                    GetProcAddress(dll, windows::core::PCSTR(cname.as_bytes_with_nul().as_ptr()))
                };
                if sym.is_none() {
                    return Err(anyhow::anyhow!(
                        "GetProcAddress({name}) 失败，请确认 pcanbasic.dll 版本正确"
                    ));
                }
                Ok(unsafe { std::mem::transmute(sym.unwrap()) })
            };

            Ok(PcanDll {
                CAN_Open: unsafe { std::mem::transmute(get_sym("CAN_Open")?) },
                CAN_Close: unsafe { std::mem::transmute(get_sym("CAN_Close")?) },
                CAN_Init: unsafe { std::mem::transmute(get_sym("CAN_Init")?) },
                CAN_Write: unsafe { std::mem::transmute(get_sym("CAN_Write")?) },
            })
        }
    }

    /// 全局 PCAN DLL（延迟加载）
    static PCAN_DLL: std::sync::OnceLock<PcanDll> = std::sync::OnceLock::new();

    pub fn get_pcan() -> &'static PcanDll {
        PCAN_DLL.get_or_init(|| PcanDll::load().expect("[pcan] 加载 pcanbasic.dll 失败"))
    }

    /// 解析通道名到 PCAN 句柄常量
    pub fn resolve_channel(name: &str) -> u32 {
        let upper = name.to_uppercase();
        if upper.contains("USBFD") || upper.contains("PROFD") {
            if upper.contains('1') { PCAN_USBPROFDBUS1 }
            else if upper.contains('2') { PCAN_USBPROFDBUS2 }
            else { PCAN_USBBUS1 }
        } else if upper.contains("USB1") { PCAN_USBBUS1 }
        else if upper.contains("USB2") { PCAN_USBBUS2 }
        else if upper.contains("USB3") { PCAN_USBBUS3 }
        else if upper.contains("USB4") { PCAN_USBBUS4 }
        else if upper.contains("USB5") { PCAN_USBBUS5 }
        else if upper.contains("USB6") { PCAN_USBBUS6 }
        else if upper.contains("USB7") { PCAN_USBBUS7 }
        else if upper.contains("USB8") { PCAN_USBBUS8 }
        else if upper.contains("PCI1") { PCAN_PCIBUS1 }
        else if upper.contains("PCI2") { PCAN_PCIBUS2 }
        else { PCAN_NONEBUS }
    }

    pub struct PcanState {
        pub handle: PcanHandle,
    }
}

// PcanAdapter 必须定义在 pcan_impl 外部（对 create_adapter 可见），
// 但条件化整个 struct 以避免非 Windows 下的"private struct"错误
#[cfg(all(windows, feature = "pcan"))]
pub struct PcanAdapter {
    channel: String,
    state: std::sync::Mutex<Option<pcan_impl::PcanState>>,
    tx: broadcast::Sender<CanFrame>,
    fd_tx: broadcast::Sender<CanFdFrame>,
    tasks: std::sync::Mutex<JoinSet<()>>,
    running: std::sync::Mutex<bool>,
    bitrate: u16,
}

#[cfg(not(all(windows, feature = "pcan")))]
#[allow(dead_code)]
pub struct PcanAdapter {
    channel: String,
    state: std::sync::Mutex<Option<()>>,
    tx: broadcast::Sender<CanFrame>,
    fd_tx: broadcast::Sender<CanFdFrame>,
    tasks: std::sync::Mutex<JoinSet<()>>,
    running: std::sync::Mutex<bool>,
    bitrate: u16,
}

// impl 条件必须和 struct 完全一致
#[cfg(all(windows, feature = "pcan"))]
impl PcanAdapter {
    pub fn new(interface: impl Into<String>, queue_size: usize) -> Self {
        let (tx, _) = broadcast::channel(queue_size);
        let (fd_tx, _) = broadcast::channel(queue_size);
        PcanAdapter {
            channel: interface.into(),
            state: std::sync::Mutex::new(None),
            tx,
            fd_tx,
            tasks: std::sync::Mutex::new(JoinSet::new()),
            running: std::sync::Mutex::new(false),
            bitrate: pcan_impl::PCAN_BAUD_500K,
        }
    }

    pub fn with_bitrate(mut self, bitrate: u16) -> Self {
        self.bitrate = bitrate;
        self
    }

    fn resolve_channel(name: &str) -> u32 {
        pcan_impl::resolve_channel(name)
    }
}

#[cfg(not(all(windows, feature = "pcan")))]
#[allow(dead_code)]
impl PcanAdapter {
    pub fn new(interface: impl Into<String>, queue_size: usize) -> Self {
        let (tx, _) = broadcast::channel(queue_size);
        let (fd_tx, _) = broadcast::channel(queue_size);
        PcanAdapter {
            channel: interface.into(),
            state: std::sync::Mutex::new(None),
            tx,
            fd_tx,
            tasks: std::sync::Mutex::new(JoinSet::new()),
            running: std::sync::Mutex::new(false),
            bitrate: 0x001C,
        }
    }

    pub fn with_bitrate(mut self, bitrate: u16) -> Self {
        self.bitrate = bitrate;
        self
    }

    fn resolve_channel(_name: &str) -> u32 {
        0
    }
}

/// PCAN 适配器（未启用 feature 时的占位实现）
#[async_trait]
#[cfg(not(all(windows, feature = "pcan")))]
impl CanAdapter for PcanAdapter {
    async fn open(&self) -> Result<()> {
        Err(anyhow::anyhow!(
            "PCAN 适配器需要 Windows + `pcan` feature。\n\
             在 Cargo.toml 中：tx_di_can = {{ features = [\"pcan\"] }}\n\
             并确保已安装 PEAK PCAN-Basic SDK。"
        ))
    }
    async fn close(&self) -> Result<()> {
        Ok(())
    }
    async fn send(&self, _frame: &CanFrame) -> Result<()> {
        #[allow(unused_variables)]
        let _ = _frame;
        Err(anyhow::anyhow!("PCAN 适配器不可用"))
    }
    async fn send_fd(&self, _frame: &CanFdFrame) -> Result<()> {
        #[allow(unused_variables)]
        let _ = _frame;
        Err(anyhow::anyhow!("PCAN FD 适配器不可用"))
    }
    fn subscribe(&self) -> broadcast::Receiver<CanFrame> {
        let (tx, _) = broadcast::channel(1);
        tx.subscribe()
    }
    fn subscribe_fd(&self) -> broadcast::Receiver<CanFdFrame> {
        let (tx, _) = broadcast::channel(1);
        tx.subscribe()
    }
    fn name(&self) -> &str {
        &self.channel
    }
}

#[async_trait]
#[cfg(all(windows, feature = "pcan"))]
impl CanAdapter for PcanAdapter {
    async fn open(&self) -> Result<()> {
        use pcan_impl::*;

        let channel = Self::resolve_channel(&self.channel);
        if channel == PCAN_NONEBUS {
            return Err(anyhow::anyhow!(
                "PCAN: 无法识别通道 '{}'，示例：PCAN_USBBUS1, PCAN_USBPROFDDBUS1",
                self.channel
            ));
        }

        let pcan = get_pcan();

        // 打开设备
        let handle = unsafe { (pcan.CAN_Open)(channel) };
        if handle == 0 || handle == -1isize {
            return Err(anyhow::anyhow!("PCAN: CAN_Open({:#x}) 失败", channel));
        }

        // 初始化波特率（hwtype=0 表示 USB）
        let result = unsafe { (pcan.CAN_Init)(handle, self.bitrate, 0) };
        if result != PCAN_ERROR_OK {
            unsafe { (pcan.CAN_Close)(handle) };
            return Err(anyhow::anyhow!(
                "PCAN: CAN_Init 失败，错误码: 0x{:08X}",
                result
            ));
        }

        *self.state.lock().unwrap() = Some(PcanState { handle });
        tracing::info!(
            "[pcan] 设备 '{}' 已打开，handle={:#x}，bitrate=0x{:04X}",
            self.channel,
            handle as usize,
            self.bitrate
        );
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        {
            let mut running = self.running.lock().unwrap();
            *running = false;
        }

        let mut tasks = std::mem::take(&mut *self.tasks.lock().unwrap());
        tasks.abort_all();

        if let Some(state) = self.state.lock().unwrap().take() {
            let pcan = get_pcan();
            unsafe { (pcan.CAN_Close)(state.handle) };
        }
        tracing::info!("[pcan] 设备 '{}' 已关闭", self.channel);
        Ok(())
    }

    async fn send(&self, frame: &CanFrame) -> Result<()> {
        use pcan_impl::*;

        let handle = {
            let state = self.state.lock().unwrap();
            match *state {
                Some(ref s) => s.handle,
                None => return Err(anyhow::anyhow!("PCAN 未打开")),
            }
        };

        let pcan = get_pcan();
        let msg = PcanMsg::new(
            frame.id.raw(),
            PCAN_MESSAGE_STANDARD,
            frame.data.len().min(8) as u8,
            &frame.data,
        );

        let result = unsafe { (pcan.CAN_Write)(handle, &msg) };
        if result != PCAN_ERROR_OK {
            return Err(anyhow::anyhow!(
                "PCAN: CAN_Write 失败，错误码: 0x{:08X}",
                result
            ));
        }

        tracing::trace!(
            "[pcan] TX {:03X} [{:02X}]",
            frame.id.raw(),
            frame.data.len()
        );
        Ok(())
    }

    async fn send_fd(&self, frame: &CanFdFrame) -> Result<()> {
        use pcan_impl::*;

        let handle = {
            let state = self.state.lock().unwrap();
            match *state {
                Some(ref s) => s.handle,
                None => return Err(anyhow::anyhow!("PCAN 未打开")),
            }
        };

        let pcan = get_pcan();
        let len = frame.data.len().min(64) as u8;
        let flags =
            PCAN_MESSAGE_FD
            | if frame.brs { PCAN_MESSAGE_BRS } else { 0 }
            | if frame.esi { PCAN_MESSAGE_ESI } else { 0 };
        let msg = PcanMsg::new(frame.id.raw(), flags, len, &frame.data);

        let result = unsafe { (pcan.CAN_Write)(handle, &msg) };
        if result != PCAN_ERROR_OK {
            return Err(anyhow::anyhow!(
                "PCAN FD: CAN_Write 失败，错误码: 0x{:08X}",
                result
            ));
        }
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<CanFrame> {
        self.tx.subscribe()
    }

    fn subscribe_fd(&self) -> broadcast::Receiver<CanFdFrame> {
        self.fd_tx.subscribe()
    }

    fn name(&self) -> &str {
        &self.channel
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 工厂函数：根据配置创建对应适配器
// ─────────────────────────────────────────────────────────────────────────────

pub fn create_adapter(
    kind: &AdapterKind,
    interface: &str,
    queue_size: usize,
) -> Arc<dyn CanAdapter> {
    match kind {
        AdapterKind::SimBus => Arc::new(SimBusAdapter::new(interface, queue_size)),
        AdapterKind::SocketCan => Arc::new(SocketCanAdapter::new(interface, queue_size)),
        #[cfg(all(windows, feature = "pcan"))]
        AdapterKind::Pcan => Arc::new(PcanAdapter::new(interface, queue_size)),
        #[cfg(not(all(windows, feature = "pcan")))]
        AdapterKind::Pcan => {
            tracing::warn!("[pcan] 适配器未启用，请使用 `features = [\"pcan\"]` 并在 Windows 上编译");
            Arc::new(SimBusAdapter::new(interface, queue_size))
        }
        AdapterKind::Kvaser => {
            tracing::warn!("[kvaser] 适配器占位，降级为 SimBus");
            Arc::new(SimBusAdapter::new(interface, queue_size))
        }
    }
}
