//! CAN/CANFD 上位机插件主体

use crate::adapter::create_adapter;
use crate::adapter::CanAdapter;
use crate::config::CanConfig;
use crate::event::{emit_event, CanEvent};
use crate::flash::{FlashConfig, FlashEngine, FlashResult};
use crate::frame::{CanFdFrame, CanFrame};
use crate::isotp::{IsoTpChannel, IsoTpConfig};
use crate::uds::UdsClient;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tx_di_core::{tx_comp, App, BoxFuture, BuildContext, CompInit, InnerContext, RIE};

/// 默认配置文件路径（可通过 #[tx_cst(my_custom_path())] 覆盖）
fn default_can_config_path() -> String {
    "configs/can.toml".to_string()
}

/// 全局服务实例
static INSTANCE: OnceLock<CanPluginInner> = OnceLock::new();

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
    /// 运行标志（Arc 共享，可在闭包间传递）
    running: Arc<AtomicBool>,
}

impl CanPluginInner {
    fn new(config: CanConfig) -> Self {
        let config = Arc::new(config);
        let adapter = create_adapter(
            &config.adapter,
            &config.interface,
            config.rx_queue_size,
        );

        let isotp_config = IsoTpConfig {
            tx_id: config.isotp_tx_id,
            rx_id: config.isotp_rx_id,
            block_size: config.isotp_block_size,
            st_min_ms: config.isotp_st_min_ms,
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
            running: Arc::new(AtomicBool::new(false)),
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

            let rx = inner.adapter.subscribe();
            inner.running.store(true, Ordering::SeqCst);
            let running = Arc::clone(&inner.running); // running: Arc<AtomicBool>
            let mut rec = rx;
            while running.load(Ordering::SeqCst) {
                match rec.recv().await {
                    Ok(frame) => {
                        emit_event(CanEvent::FrameReceived(frame)).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("[can] 接收队列溢出，丢弃 {} 帧", n);
                    }
                    Err(_) => break,
                }
            }
        });
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
#[tx_comp(init)]
pub struct CanPlugin {
    /// 配置文件路径，默认为 `configs/can.toml`
    #[tx_cst(default_can_config_path())]
    pub config_path: String,
}

impl CanPlugin {
    /// 获取全局实例（必须在 BuildContext::build() 之后调用）
    #[allow(private_interfaces)]
    pub fn instance() -> &'static CanPluginInner {
        INSTANCE.get().expect("CanPlugin 未初始化，请先 BuildContext::build()")
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
}

// ─────────────────────────────────────────────────────────────────────────────
// CompInit 实现（同步 + 异步分离，与 GB28181 插件模式一致）
// ─────────────────────────────────────────────────────────────────────────────

impl CompInit for CanPlugin {
    /// 同步初始化：加载配置，建立全局实例
    fn inner_init(&mut self, _: &InnerContext) -> RIE<()> {
        let config = CanConfig::load_from_toml(&self.config_path)
            .map_err(|e| {
                tx_di_core::IE::from(anyhow::anyhow!(
                    "[can] 加载配置失败 {}: {}",
                    self.config_path,
                    e
                ))
            })?;

        if INSTANCE.set(CanPluginInner::new(config)).is_err() {
            warn!("[can] CanPlugin 重复初始化");
        }
        Ok(())
    }

    /// 异步初始化：启动帧接收循环
    fn async_init(_ctx: Arc<App>, token: CancellationToken) -> BoxFuture<'static, RIE<()>> {
        Box::pin(async move {
            if let Some(inner_ref) = INSTANCE.get() {
                let inner = Arc::new(inner_ref.clone());
                let running = inner.running.clone();
                
                // 监听取消信号，停止接收循环
                let cancel_task = tokio::spawn(async move {
                    token.cancelled().await;
                    running.store(false, Ordering::SeqCst);
                    info!("[can] 收到停止信号，关闭 CAN 接收循环");
                });
                
                inner.start_rx_loop().await;
                cancel_task.abort(); // 清理取消任务
            }
            Ok(())
        })
    }

    fn init_sort() -> i32 {
        i32::MAX - 4
    }
}
