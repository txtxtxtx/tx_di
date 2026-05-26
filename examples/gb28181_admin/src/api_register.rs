//! API 路由注册组件
//!
//! 在 DI 框架的 inner_init 阶段（早于 WebPlugin 的 i32::MAX）完成所有 API 路由注册，
//! 确保 WebPlugin::inner_init 调用 merge_routers() 时路由已经就位。
//!
//! ## SIP 注册持久化
//!
//! 本模块通过以下机制实现设备注册状态的跨重启持久化：
//!
//! 1. **事件→DB 同步**：注册第二个事件监听器，将 `DeviceRegistered` /
//!    `DeviceUnregistered` / `DeviceOffline` / `DeviceOnline` 事件
//!    同步写入 toasty 数据库
//! 2. **启动恢复**：`async_init` 中从数据库加载 `online=true` 的设备记录，
//!    调用 `Gb28181Server::restore_devices()` 恢复到内存注册表
//!
//! 数据流：
//! ```text
//! 设备 REGISTER → handlers.rs → registry.register() + emit(DeviceRegistered)
//!   └→ 监听器1 (SSE) → broadcast_event → 推送给前端
//!   └→ 监听器2 (DB同步) → upsert GbDeviceRecord(online=true)
//! ```

use std::sync::{Arc, OnceLock};
use toasty::Db;
use tracing::{error, info, warn};
use tx_di_axum::WebPlugin;
use tx_di_core::{BuildContext, CompInit, InnerContext, RIE, tx_comp, App, CancellationToken, BoxFuture};
use tx_di_gb28181::Gb28181Server;
use tx_di_sa_token::SaTokenPlugin;
use tx_di_toasty::ToastyPlugin;
use tx_gb28181::device::GbDevice;
use tx_gb28181::event::Gb28181Event;

use crate::{api, models};
use crate::models::GbDeviceRecord;

/// 静态 DB 引用，供事件监听器回写数据库
///
/// 在 `async_init` 中设置，在 `inner_init` 中注册的事件监听器会稍后读取。
static DB: OnceLock<Db> = OnceLock::new();

/// API 路由注册组件
///
/// init_sort = i32::MAX - 100，早于 WebPlugin（i32::MAX）执行，
/// 确保路由在 WebPlugin::merge_routers() 之前注册到 ROUTER_REGISTRY。
#[tx_comp(init)]
pub struct ApiRegisterComponent {}

impl CompInit for ApiRegisterComponent {

    fn inner_init(&mut self, ctx: &InnerContext) -> RIE<()>{
        let ctx: BuildContext = ctx.into();
        let toasty_plugin = ctx.inject::<ToastyPlugin>();

        // 1. 注册事件监听器（用于 SSE 推送）
        Gb28181Server::on_event(|event| async move {
            api::sse::broadcast_event(event);
            Ok(())
        });

        // 2. 注册事件监听器（用于 DB 同步 — 持久化设备注册状态）
        Gb28181Server::on_event(|event| async move {
            if let Some(db) = DB.get() {
                let mut db = db.clone();
                if let Err(e) = sync_event_to_db(event, &mut db).await {
                    error!(error = %e, "设备状态 DB 同步失败");
                }
            }
            Ok(())
        });

        // 3. 【关键】在 BuildContext::new() 之后、build() 之前注册数据库模型
        //    可以多次调用 register_models()，模型会合并（重复 ModelId 自动覆盖）
        toasty_plugin.register_models(toasty::models!(
            models::User,
            models::GbDeviceRecord,
            models::GbSessionRecord,
            models::GbAlarmRecord,
            models::GbAuditLog,
            models::GbDeviceGroup,
            models::GbDeviceGroupMember,
            models::GbRegisterAudit,
        ));
        Ok(())
    }

    fn async_init(ctx: Arc<App>, _token: CancellationToken) -> BoxFuture {
        Box::pin(async move {
            let toasty_plugin = ctx.inject::<ToastyPlugin>();
            let db = toasty_plugin.db().clone();

            // 4. 设置静态 DB（事件监听器将在后续事件中可用）
            let _ = DB.set(db.clone());

            // 5. 从数据库恢复之前在线设备到内存注册表
            if let Err(e) = restore_devices_from_db(&db, &ctx).await {
                error!(error = %e, "从数据库恢复设备状态失败");
            }

            // 6. 获取 sa_token 插件实例，提取 SaTokenState
            let sa_plugin = ctx.inject::<SaTokenPlugin>();
            let sa_state = sa_plugin.state().clone();

            // 7. 注册带 State 的 API 路由
            WebPlugin::add_router(api::router(db, sa_state));
            info!("gb28181_admin 初始化完成");
            Ok(())
        })
    }

    fn init_sort() -> i32 {
        i32::MAX - 100
    }
}

// ── 事件 → DB 同步 ────────────────────────────────────────────────────────────

/// 将 GB28181 事件同步到数据库
async fn sync_event_to_db(event: Gb28181Event, db: &mut Db) -> RIE<()> {
    match event {
        Gb28181Event::DeviceRegistered { device_id, contact, remote_addr } => {
            upsert_device(db, &device_id, &contact, &remote_addr).await?;
        }
        Gb28181Event::DeviceUnregistered { device_id } => {
            set_device_online(db, &device_id, false).await?;
        }
        Gb28181Event::DeviceOffline { device_id } => {
            set_device_online(db, &device_id, false).await?;
        }
        Gb28181Event::DeviceOnline { device_id } => {
            set_device_online(db, &device_id, true).await?;
        }
        // 其他事件暂不写入 DB
        _ => {}
    }
    Ok(())
}

/// 插入或更新设备记录（设备注册时）
async fn upsert_device(db: &mut Db, device_id: &str, contact: &str, remote_addr: &str) -> RIE<()> {
    match GbDeviceRecord::filter_by_device_id(device_id.to_string())
        .first()
        .exec(&mut *db)
        .await
    {
        Ok(Some(mut existing)) => {
            // 更新已有记录
            existing
                .update()
                .contact(contact.to_string())
                .remote_addr(remote_addr.to_string())
                .online(true)
                .exec(&mut *db)
                .await
                .map_err(|e| anyhow::anyhow!("更新设备记录失败: {e}"))?;
            info!(device_id = %device_id, "设备 DB 记录已更新（上线）");
        }
        Ok(None) => {
            // 新建设备记录（toasty 0.6 create! 宏：显式字段 + 其余走 #[default]）
            match toasty::create!(GbDeviceRecord {
                device_id: device_id.to_string(),
                contact: contact.to_string(),
                remote_addr: remote_addr.to_string(),
                online: true,
            })
            .exec(&mut *db)
            .await
            {
                Ok(_) => info!(device_id = %device_id, "设备 DB 记录已创建"),
                Err(e) => warn!(device_id = %device_id, error = %e, "创建设备 DB 记录失败"),
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("查询设备记录失败: {e}").into());
        }
    }
    Ok(())
}

/// 设置设备在线状态
async fn set_device_online(db: &mut Db, device_id: &str, online: bool) -> RIE<()> {
    match GbDeviceRecord::filter_by_device_id(device_id.to_string())
        .first()
        .exec(&mut *db)
        .await
    {
        Ok(Some(mut record)) => {
            record
                .update()
                .online(online)
                .exec(&mut *db)
                .await
                .map_err(|e| anyhow::anyhow!("更新设备在线状态失败: {e}"))?;
            let status = if online { "上线" } else { "离线" };
            info!(device_id = %device_id, "设备 DB 记录已更新（{status}）");
        }
        Ok(None) => {
            warn!(device_id = %device_id, "设备 DB 记录不存在，跳过状态更新");
        }
        Err(e) => {
            return Err(anyhow::anyhow!("查询设备记录失败: {e}").into());
        }
    }
    Ok(())
}

// ── 启动恢复 ─────────────────────────────────────────────────────────────────

/// 从数据库恢复之前在线的设备到内存注册表
///
/// 恢复后的设备默认标记为离线（`online = false`），
/// 设备重新 REGISTER 或发送心跳后会自动上线。
async fn restore_devices_from_db(db: &Db, app: &App) -> RIE<()> {
    let records = match GbDeviceRecord::all().exec(&mut db.clone()).await {
        Ok(r) => r,
        Err(e) => {
            return Err(anyhow::anyhow!("加载设备列表失败: {e}").into());
        }
    };

    let online_count = records.iter().filter(|r| r.online).count();
    if online_count == 0 {
        info!("无在线设备需要恢复");
        return Ok(());
    }

    // 构造 GbDevice 列表
    let devices: Vec<GbDevice> = records
        .into_iter()
        .filter(|r| r.online)
        .map(|r| {
            let mut dev = GbDevice::new_device(&r.device_id, &r.device_id);
            dev.contact = r.contact;
            dev.remote_addr = r.remote_addr;
            dev.channel = r.channel_count as u32;
            // online 会在 restore() 中被设为 false
            dev
        })
        .collect();

    let dev_count = devices.len();
    info!(
        count = dev_count,
        "正在恢复 {} 个设备到内存注册表（标记为离线等待重新注册）",
        dev_count
    );

    Gb28181Server::restore_devices(app, devices);
    Ok(())
}
