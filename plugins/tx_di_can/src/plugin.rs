//! CAN/CANFD 上位机插件主体

use crate::adapter::create_adapter;
use crate::adapter::CanAdapter;
use crate::config::{AdapterKind, CanConfig};
use crate::db::DescDb;
use crate::event::{emit_event, CanEvent};
use crate::flash::{FlashConfig, FlashEngine, FlashResult};
use crate::frame::{CanFdFrame, CanFrame};
use crate::isotp::{IsoTpChannel, IsoTpConfig};
use crate::record::Recorder;
use crate::sim_ecu::{spawn_sim_ecu, SimEcuConfig};
use crate::uds::UdsClient;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast::error::RecvError;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use tx_di_core::{App, Component, DepsTuple, RIE, Store};

/// 总线统计（帧计数 / 字节数 / 总线负载率）
#[derive(Default, Clone, serde::Serialize)]
pub struct BusStats {
    /// 累计标准 CAN 帧数
    pub frame_count: u64,
    /// 累计 CANFD 帧数
    pub fd_frame_count: u64,
    /// 累计数据字节数
    pub bytes: u64,
    /// 统计起始时刻（UNIX 毫秒）
    pub start_ms: u64,
    /// 估算总线负载率（千分比，0~1000）
    pub load_permille: u32,
}

impl BusStats {
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn new() -> Self {
        BusStats {
            start_ms: Self::now_ms(),
            ..Default::default()
        }
    }

    /// 记录一帧并更新负载率估算
    fn record(&mut self, is_fd: bool, dlc: usize, bitrate: u32) {
        if is_fd {
            self.fd_frame_count += 1;
        } else {
            self.frame_count += 1;
        }
        self.bytes += dlc as u64;
        if bitrate == 0 {
            return;
        }
        let elapsed_s = (Self::now_ms().saturating_sub(self.start_ms)) as f64 / 1000.0;
        if elapsed_s <= 0.0 {
            return;
        }
        // 帧位时间估算：经典 CAN ≈ 47 + 8*dlc 位；FD 数据段单独计，粗略按 2 倍
        let bits = if is_fd {
            67 + 8 * dlc * 2
        } else {
            47 + 8 * dlc
        } as f64;
        let bus_bits = bitrate as f64 * elapsed_s;
        if bus_bits > 0.0 {
            self.load_permille = ((bits * (self.frame_count + self.fd_frame_count) as f64
                / bus_bits)
                * 1000.0)
                .min(1000.0) as u32;
        }
    }
}

/// 应用层帧过滤器（ID 范围 / 掩码匹配）
#[derive(Default, Clone, serde::Serialize)]
pub struct FrameFilter {
    /// ID 下限（含），None 表示不限
    pub id_min: Option<u32>,
    /// ID 上限（含），None 表示不限
    pub id_max: Option<u32>,
    /// 掩码：仅比较 (id & id_mask) == (id_match & id_mask)
    pub id_mask: u32,
    /// 期望匹配值（与 id_mask 配合）
    pub id_match: u32,
}

impl FrameFilter {
    /// 不过滤（任何帧都通过）
    pub fn none() -> Self {
        FrameFilter::default()
    }

    /// 当前过滤器是否放行该 ID
    pub fn matches(&self, id: u32) -> bool {
        if let Some(lo) = self.id_min {
            if id < lo {
                return false;
            }
        }
        if let Some(hi) = self.id_max {
            if id > hi {
                return false;
            }
        }
        if self.id_mask != 0 {
            if (id & self.id_mask) != (self.id_match & self.id_mask) {
                return false;
            }
        }
        true
    }
}

/// 默认配置文件路径（可通过 #[tx_cst(my_custom_path())] 覆盖）
fn default_can_config_path() -> String {
    "configs/can.toml".to_string()
}

/// 全局服务实例（运行期可重置，以支持 connect/disconnect 重连）
static INSTANCE: RwLock<Option<Arc<CanPluginInner>>> = RwLock::new(None);

/// 全局配置路径（由 BuildContext 注入）
// CONFIG_PATH 已废弃：配置直接由 config_path 字段传入 inner_init


#[derive(Clone)]
pub(crate) struct CanPluginInner {
    config: Arc<CanConfig>,
    adapter: Arc<dyn CanAdapter>,
    /// 默认 UDS 客户端
    #[allow(dead_code)]
    uds_default: Arc<UdsClient>,
    /// 多通道 UDS 客户端（按 tx_id 缓存）
    uds_channels: dashmap::DashMap<u32, Arc<UdsClient>>,
    /// 描述库（DID/DTC 应答与展示数据来源）
    db: Arc<DescDb>,
    /// 运行标志（Arc 共享，可在闭包间传递）
    running: Arc<AtomicBool>,
    /// 总线统计（帧计数 / 字节数 / 负载率）
    stats: Arc<Mutex<BusStats>>,
    /// 应用层帧过滤器（None 表示不过滤）
    filter: Arc<RwLock<Option<FrameFilter>>>,
}

impl CanPluginInner {
    fn new(config: CanConfig) -> Self {
        let config = Arc::new(config);
        let adapter = create_adapter(
            &config.adapter,
            &config.interface,
            config.rx_queue_size,
            config.bitrate,
        );

        let isotp_config = IsoTpConfig {
            tx_id: config.isotp_tx_id,
            rx_id: config.isotp_rx_id,
            block_size: config.isotp_block_size,
            st_min_ms: config.isotp_st_min_ms,
            is_fd: config.enable_fd,
            ..Default::default()
        };
        let uds_default = Arc::new(UdsClient::new(
            adapter.clone(),
            isotp_config,
            config.uds_p2_timeout_ms,
            config.uds_p2_star_timeout_ms,
        ));

        CanPluginInner {
            config,
            adapter,
            uds_default,
            uds_channels: dashmap::DashMap::new(),
            db: Arc::new(DescDb::builtin()),
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(Mutex::new(BusStats::new())),
            filter: Arc::new(RwLock::new(None)),
        }
    }

    /// 启动帧接收后台任务（异步打开适配器 + 循环推帧到事件总线）
    async fn start_rx_loop(self: Arc<Self>) {
        let inner = self.clone();
        tokio::spawn(async move {
            let adapter_name = inner.adapter.name().to_string();

            if let Err(e) = inner.adapter.open().await {
                warn!("[can] 适配器打开失败: {e}，CAN 总线不可用");
                emit_event(CanEvent::BusError {
                    description: e.to_string(),
                })
                .await;
            } else {
                emit_event(CanEvent::BusReady {
                    interface: adapter_name.clone(),
                })
                .await;
                info!("[can] CAN 总线已就绪: {}", adapter_name);
            }

            // 适配器就绪后启动 ECU 仿真节点（SimBus 下自动开启，便于无设备联调）
            inner.start_sim_ecu();

            let rx = inner.adapter.subscribe();
            let fd_rx = inner.adapter.subscribe_fd();
            inner.running.store(true, Ordering::SeqCst);
            let running = Arc::clone(&inner.running); // running: Arc<AtomicBool>
            let mut rec = rx;
            let mut fd_rec = fd_rx;
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                tokio::select! {
                    frame = rec.recv() => {
                        match frame {
                            Ok(f) => {
                                let pass = {
                                    let g = inner.filter.read().unwrap();
                                    match &*g {
                                        Some(ft) => ft.matches(f.id.raw()),
                                        None => true,
                                    }
                                };
                                if pass {
                                    emit_event(CanEvent::FrameReceived(f.clone())).await;
                                }
                                inner
                                    .stats
                                    .lock()
                                    .unwrap()
                                    .record(false, f.data.len(), inner.config.bitrate);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                warn!("[can] 接收队列溢出，丢弃 {} 帧", n);
                            }
                            Err(_) => break,
                        }
                    }
                    fd = fd_rec.recv() => {
                        match fd {
                            Ok(f) => {
                                let pass = {
                                    let g = inner.filter.read().unwrap();
                                    match &*g {
                                        Some(ft) => ft.matches(f.id.raw()),
                                        None => true,
                                    }
                                };
                                if pass {
                                    emit_event(CanEvent::FdFrameReceived(f.clone())).await;
                                }
                                inner
                                    .stats
                                    .lock()
                                    .unwrap()
                                    .record(true, f.data.len(), inner.config.bitrate);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                warn!("[can] FD 接收队列溢出，丢弃 {} 帧", n);
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        });
    }

    /// 启动 ECU 仿真节点（若配置启用或当前为 SimBus 适配器）
    ///
    /// 仿真任务作为订阅者挂在接收循环之后，对诊断帧生成 UDS 响应并回发，
    /// 使 SimBus 从"回环"变为"ECU 应答"。真实适配器（PCAN/SocketCAN）不受影响。
    fn start_sim_ecu(self: &Arc<Self>) {
        let enabled = self.config.sim_ecu || matches!(self.config.adapter, AdapterKind::SimBus);
        if !enabled {
            return;
        }
        let cfg = SimEcuConfig::from_can_config(&self.config);
        let adapter = self.adapter.clone();
        let db = self.db.clone();
        let running = self.running.clone();
        spawn_sim_ecu(adapter, cfg, db, running);
    }
}

/// CAN/CANFD 上位机插件
///
/// # 配置文件
/// ```toml
/// [can_config]
/// adapter    = "simbus"
/// interface  = "vcan0"
/// bitrate    = 500_000
/// enable_fd  = false
/// isotp_tx_id = 0x7E0
/// isotp_rx_id = 0x7E8
/// ```
///
/// # 使用示例
/// ```rust,ignore
/// use tx_di_can::{CanPlugin, CanEvent, FlashConfig};
/// use tx_di_core::BuildContext;
///
/// CanPlugin::on_event(|ev| async move {
///     match ev {
///         CanEvent::UdsResponse { service, payload } => {
///             println!("UDS {:02X} 响应: {:02X?}", service, payload);
///         }
///         CanEvent::FlashProgress { block_seq, total_blocks, bytes_sent, total_bytes } => {
///             println!("刷写 {}/{} 块 ({} / {} bytes)",
///                 block_seq, total_blocks, bytes_sent, total_bytes);
///         }
///         _ => {}
///     }
///     Ok(())
/// });
///
/// let mut ctx = BuildContext::new(Some("configs/can.toml"));
/// ctx.build().await.unwrap();
///
/// // UDS 诊断
/// let sw_version = CanPlugin::read_data(0x7DF, 0xF189).await.unwrap();
///
/// // 刷写固件
/// CanPlugin::flash("firmware.bin", FlashConfig {
///     target_id: 0x7E0,
///     security_level: 0x01,
///     memory_address: 0x08000000,
///     ..Default::default()
/// }, |seed| seed.iter().map(|b| !b).collect()).await.unwrap();
/// ```
#[derive(Component)]
#[component(init, app_async_init, shutdown, init_sort = 2147483643)]
pub struct CanPlugin {
    /// 配置文件路径，默认为 `configs/can.toml`
    #[tx_cst(default_can_config_path())]
    pub config_path: String,
}

impl CanPlugin {
    /// 获取全局实例（必须在 connect() 或 BuildContext::build() 之后调用）
    #[allow(private_interfaces)]
    pub fn instance() -> Arc<CanPluginInner> {
        INSTANCE
            .read()
            .unwrap()
            .clone()
            .expect("CanPlugin 未初始化，请先调用 connect() 或 BuildContext::build()")
    }

    /// 运行期连接：构造新实例、打开适配器并启动接收循环（先断开旧连接）
    pub async fn connect(config: CanConfig) -> RIE<()> {
        Self::disconnect().await;
        let inner = Arc::new(CanPluginInner::new(config));
        // 打开适配器并启动接收后台循环（内部会 emit BusReady / BusError 事件）
        // start_rx_loop 消费 Arc，故用 clone 启动、原 Arc 存入 INSTANCE
        inner.clone().start_rx_loop().await;
        *INSTANCE.write().unwrap() = Some(inner);
        Ok(())
    }

    /// 断开连接：停止接收循环并关闭适配器
    pub async fn disconnect() {
        let inner = INSTANCE.read().unwrap().clone();
        if let Some(inner) = inner {
            inner.running.store(false, Ordering::SeqCst);
            let adapter = inner.adapter.clone();
            let _ = adapter.close().await;
        }
    }

    /// 是否已连接（接收循环运行中）
    pub fn is_connected() -> bool {
        INSTANCE
            .read()
            .unwrap()
            .as_ref()
            .map_or(false, |i| i.running.load(Ordering::SeqCst))
    }

    /// 读取当前生效配置（未连接时返回 None）
    pub fn get_config() -> Option<CanConfig> {
        INSTANCE
            .read()
            .unwrap()
            .as_ref()
            .map(|i| (*i.config).clone())
    }

    /// 默认配置（供 UI 初始化表单）
    pub fn default_config() -> CanConfig {
        CanConfig::default()
    }

    /// 订阅事件
    pub fn on_event<F, Fut>(handler: F)
    where
        F: Fn(CanEvent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        crate::event::on_event(handler);
    }

    /// 创建指定 tx/rx ID 的 ISO-TP 通道
    pub fn create_isotp_channel(tx_id: u32, rx_id: u32) -> IsoTpChannel {
        let inner = Self::instance();
        IsoTpChannel::new(
            inner.adapter.clone(),
            IsoTpConfig {
                tx_id,
                rx_id,
                is_fd: inner.config.enable_fd,
                ..Default::default()
            },
        )
    }

    /// 获取指定 tx/rx ID 的 UDS 客户端（按 tx_id 缓存）
    pub fn uds_client(tx_id: u32, rx_id: u32) -> Arc<UdsClient> {
        let inner = Self::instance();
        if let Some(client) = inner.uds_channels.get(&tx_id) {
            return client.clone();
        }
        let client = Arc::new(UdsClient::new(
            inner.adapter.clone(),
            IsoTpConfig {
                tx_id,
                rx_id,
                is_fd: inner.config.enable_fd,
                ..Default::default()
            },
            inner.config.uds_p2_timeout_ms,
            inner.config.uds_p2_star_timeout_ms,
        ));
        inner.uds_channels.insert(tx_id, client.clone());
        client
    }

    /// UDS 读取数据标识符（0x22 DID）
    pub async fn read_data(tx_id: u32, did: u16) -> Result<Vec<u8>, crate::uds::UdsError> {
        Self::uds_client(tx_id, tx_id.wrapping_add(8))
            .read_data(did)
            .await
    }

    /// UDS 写入数据标识符（0x2E DID）
    pub async fn write_data(
        tx_id: u32,
        did: u16,
        data: &[u8],
    ) -> Result<(), crate::uds::UdsError> {
        Self::uds_client(tx_id, tx_id.wrapping_add(8))
            .write_data(did, data)
            .await
    }

    /// 发送原始 CAN 帧
    pub async fn send_frame(frame: &CanFrame) -> Result<()> {
        Self::instance().adapter.send(frame).await
    }

    /// 发送 CANFD 帧
    pub async fn send_fd_frame(frame: &CanFdFrame) -> Result<()> {
        Self::instance().adapter.send_fd(frame).await
    }

    /// 创建刷写引擎
    pub fn flash_engine(config: FlashConfig) -> FlashEngine {
        let client = Self::uds_client(config.target_id, config.target_id.wrapping_add(8));
        FlashEngine::new(client, config)
    }

    /// 刷写固件
    pub async fn flash<F>(
        firmware: impl std::convert::AsRef<std::path::Path>,
        config: FlashConfig,
        key_fn: F,
    ) -> Result<FlashResult>
    where
        F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        let client = Self::uds_client(config.target_id, config.target_id.wrapping_add(8));
        let engine = FlashEngine::new(client, config);
        engine.flash(firmware, key_fn).await
    }

    /// 读取总线统计（帧计数 / 字节数 / 负载率）
    pub fn get_stats() -> Option<BusStats> {
        INSTANCE
            .read()
            .unwrap()
            .as_ref()
            .map(|i| i.stats.lock().unwrap().clone())
    }

    /// 重置总线统计计数
    pub fn reset_stats() {
        if let Some(inner) = INSTANCE.read().unwrap().as_ref() {
            *inner.stats.lock().unwrap() = BusStats::new();
        }
    }

    /// 设置应用层帧过滤器（None 表示不过滤）
    pub fn set_filter(filter: Option<FrameFilter>) {
        if let Some(inner) = INSTANCE.read().unwrap().as_ref() {
            *inner.filter.write().unwrap() = filter;
        }
    }

    /// 读取当前帧过滤器
    pub fn get_filter() -> Option<FrameFilter> {
        INSTANCE
            .read()
            .unwrap()
            .as_ref()
            .and_then(|i| i.filter.read().unwrap().clone())
    }

    /// 取得描述库（DID/DTC 应答与展示数据来源）
    pub fn desc_db() -> Option<Arc<DescDb>> {
        INSTANCE.read().unwrap().as_ref().map(|i| i.db.clone())
    }

    /// 当前是否启用了 ECU 仿真节点（SimBus 或显式开启）
    pub fn sim_ecu_enabled() -> bool {
        INSTANCE.read().unwrap().as_ref().map_or(false, |i| {
            i.config.sim_ecu || matches!(i.config.adapter, AdapterKind::SimBus)
        })
    }

    /// 录制总线帧到 CSV（持续 duration_ms 毫秒）
    pub async fn record_csv(path: &str, duration_ms: u64) -> Result<u32, String> {
        let inner = Self::instance();
        let mut rx = inner.adapter.subscribe();
        let mut fd = inner.adapter.subscribe_fd();
        let mut rec = Recorder::new(path).map_err(|e| e.to_string())?;
        let start = std::time::Instant::now();
        let mut count = 0u32;
        loop {
            if start.elapsed().as_millis() as u64 >= duration_ms {
                break;
            }
            tokio::select! {
                f = rx.recv() => match f {
                    Ok(fr) => { let _ = rec.record_can(&fr); count += 1; }
                    Err(RecvError::Lagged(_)) => {}
                    Err(_) => break,
                },
                ff = fd.recv() => match ff {
                    Ok(fr) => { let _ = rec.record_fd(&fr); count += 1; }
                    Err(RecvError::Lagged(_)) => {}
                    Err(_) => break,
                },
            }
        }
        Ok(count)
    }

    /// 从 CSV 回放帧到总线（speed_factor 倍速；0 表示尽快）
    pub async fn replay_csv(path: &str, speed_factor: f64) -> Result<u32, String> {
        let inner = Self::instance();
        let n = crate::record::replay_csv(path, inner.adapter.as_ref(), speed_factor)
            .await
            .map_err(|e| e.to_string())?;
        Ok(n as u32)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 生命周期（新框架 API：#[derive(Component)] 覆写 inner_init / async_init / shutdown）
// ─────────────────────────────────────────────────────────────────────────────

/// inner_init（构建期调用）：加载配置并构造全局实例
/// 若已通过 connect() 设置了实例，则不覆盖（避免 DI 启动覆盖运行期配置）
fn init(comp: &mut CanPlugin, _store: &Store) -> RIE<()> {
    if INSTANCE.read().unwrap().is_some() {
        return Ok(());
    }
    let config = match CanConfig::load_from_toml(&comp.config_path) {
        Ok(c) => c,
        Err(e) => {
            warn!(
                "[can] 加载配置失败 {}: {}，使用默认配置",
                comp.config_path, e
            );
            CanConfig::default()
        }
    };
    *INSTANCE.write().unwrap() = Some(Arc::new(CanPluginInner::new(config)));
    Ok(())
}

/// async_init（运行期调用）：启动帧接收后台循环
async fn app_async_init(comp: Arc<CanPlugin>, app: Arc<App>) -> RIE<()> {
    let _ = comp;
    let inner = INSTANCE.read().unwrap().clone();
    if let Some(inner) = inner {
        let running = inner.running.clone();
        let token = app.shutdown_token.clone();
        let cancel_task = tokio::spawn(async move {
            token.cancelled().await;
            running.store(false, Ordering::SeqCst);
            info!("[can] 收到停止信号，关闭 CAN 接收循环");
        });
        inner.start_rx_loop().await;
        cancel_task.abort();
    }
    Ok(())
}

/// shutdown：停止接收循环并关闭适配器
fn shutdown(comp: &CanPlugin) {
    let _ = comp;
    let inner = INSTANCE.read().unwrap().clone();
    if let Some(inner) = inner {
        inner.running.store(false, Ordering::SeqCst);
        let adapter = inner.adapter.clone();
        tokio::spawn(async move {
            let _ = adapter.close().await;
        });
        info!("[can] CAN 接收循环已请求停止");
    }
}
