//! 设备相关 API 处理器
//!
//! ## 提取器策略
//!
//! - **Db 操作**（列表、详情、统计） → `State<Db>` — 从 axum State 获取数据库连接
//! - **Gb28181Server 实时操作**（PTZ、点播、SIP 指令） → `DiComp<Gb28181Server>` — 从 DI 容器提取
//!
//! 两者可以共存于同一 handler：axum 支持多提取器组合。
//! `DiComp<T>` 通过 `FromRequestParts` 实现，从请求 extensions 中的 `AppStatus`
//! → `App` → DI 容器提取已注册的单例组件。

use axum::extract::{Json as ExtJson, Path, Query as AxumQuery, State};
use serde::Deserialize;
use toasty::Db;
use toasty::stmt::{List, Paginate, Query as ToastyQuery};
use tx_di_axum::{DiComp, R};
use tx_di_gb28181::Gb28181Server;
use tx_di_gb28181::xml::{PtzCommand, PtzSpeed};

use crate::dto::{ChannelDto, DeviceDto, PageData, Pagination, StatsDto};
use crate::models::GbDeviceRecord;

// ============ 统计 ============

/// GET /api/v1/gb28181/stats — 统计概要
///
/// 从 Gb28181Server 内存获取实时统计数据（无需数据库）
pub async fn stats(srv: DiComp<Gb28181Server>) -> R<StatsDto> {
    R::ok(StatsDto {
        total: srv.device_count(),
        online: srv.online_count(),
        sessions: srv.active_sessions().len(),
    })
}

// ============ 设备列表/详情 ============

/// GET /api/v1/gb28181/devices — 设备列表（从数据库查询，支持游标分页）
pub async fn list(
    State(mut db): State<Db>,
    AxumQuery(pagination): AxumQuery<Pagination>,
) -> R<PageData<DeviceDto>> {
    // 查询总数
    let total = match GbDeviceRecord::all().count().exec(&mut db).await {
        Ok(n) => n,
        Err(e) => return R::error(500, format!("查询设备总数失败: {}", e)),
    };

    // 构建查询并按 ID 排序（游标分页必需）
    let mut query = ToastyQuery::<List<GbDeviceRecord>>::all();
    query.order_by(GbDeviceRecord::fields().id().asc());

    // 创建分页器并执行查询
    let page = match Paginate::new(query, pagination.page_size as usize)
        .exec(&mut db)
        .await
    {
        Ok(p) => p,
        Err(e) => return R::error(500, format!("查询设备列表失败: {}", e)),
    };

    // 转换数据
    let dtos: Vec<DeviceDto> = page.items.into_iter().map(DeviceDto::from).collect();
    let next_cursor = page.next_cursor.and_then(|c| match c {
        toasty::stmt::Value::I64(id) => Some(id.to_string()),
        _ => None,
    });

    R::ok(PageData {
        items: dtos,
        total,
        page: pagination.page,
        page_size: pagination.page_size,
        total_pages: if pagination.page_size == 0 {
            0
        } else {
            (total + pagination.page_size - 1) / pagination.page_size
        },
        next_cursor,
    })
}

/// GET /api/v1/gb28181/devices/:id — 设备详情（含通道）
///
/// 组合使用 `DiComp<Gb28181Server>`（获取实时在线状态和通道信息）+ `State<Db>`（获取设备记录）
pub async fn detail(
    Path(id): Path<String>,
    State(mut db): State<Db>,
    srv: DiComp<Gb28181Server>,
) -> R<DeviceDto> {
    // 从数据库查询设备记录
    let record = match GbDeviceRecord::filter_by_device_id(id.clone())
        .first()
        .exec(&mut db)
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return R::error(404, format!("设备 {} 不存在", id)),
        Err(e) => return R::error(500, format!("查询设备失败: {}", e)),
    };

    // 补充实时在线状态
    let mut dto = DeviceDto::from(record);
    dto.online = srv.get_device(&id).is_some();

    // 补充通道信息（从 Gb28181Server 内存获取）
    let channels: Vec<ChannelDto> = srv.get_channels(&id).iter().map(ChannelDto::from).collect();
    if !channels.is_empty() {
        dto.channels = Some(channels);
    }

    R::ok(dto)
}

// ============ 设备操作（SIP 指令） ============

/// POST /api/v1/gb28181/devices/:id/catalog — 触发目录查询
pub async fn query_catalog(Path(id): Path<String>, srv: DiComp<Gb28181Server>) -> R<String> {
    match srv.query_catalog(&id).await {
        Ok(_) => R::ok("已发送目录查询".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/info — 触发设备信息查询
pub async fn query_info(Path(id): Path<String>, srv: DiComp<Gb28181Server>) -> R<String> {
    match srv.query_device_info(&id).await {
        Ok(_) => R::ok("已发送设备信息查询".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/status — 触发设备状态查询
pub async fn query_status(Path(id): Path<String>, srv: DiComp<Gb28181Server>) -> R<String> {
    match srv.query_device_status(&id).await {
        Ok(_) => R::ok("已发送状态查询".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ============ PTZ 控制 ============

/// PTZ 控制请求体
#[derive(Deserialize)]
pub struct PtzReq {
    pub channel_id: String,
    /// 方向: stop/left/right/up/down/upleft/upright/downleft/downright/zoomin/zoomout
    pub direction: String,
    /// 水平速度 0-255
    #[serde(default = "default_speed")]
    pub pan: u8,
    /// 垂直速度 0-255
    #[serde(default = "default_speed")]
    pub tilt: u8,
    /// 变倍速度 0-255
    #[serde(default)]
    pub zoom: u8,
}

fn default_speed() -> u8 {
    64
}

/// POST /api/v1/gb28181/devices/:id/ptz — PTZ 控制
pub async fn ptz(
    srv: DiComp<Gb28181Server>,
    Path(id): Path<String>,
    ExtJson(req): ExtJson<PtzReq>,
) -> R<String> {
    let speed = PtzSpeed {
        pan: req.pan,
        tilt: req.tilt,
        zoom: req.zoom,
    };
    let cmd = match req.direction.to_lowercase().as_str() {
        "left" => PtzCommand::Left(speed),
        "right" => PtzCommand::Right(speed),
        "up" => PtzCommand::Up(speed),
        "down" => PtzCommand::Down(speed),
        "upleft" => PtzCommand::LeftUp(speed),
        "upright" => PtzCommand::RightUp(speed),
        "downleft" => PtzCommand::LeftDown(speed),
        "downright" => PtzCommand::RightDown(speed),
        "zoomin" => PtzCommand::ZoomIn(speed),
        "zoomout" => PtzCommand::ZoomOut(speed),
        _ => PtzCommand::Stop,
    };
    match srv.ptz_control(&id, &req.channel_id, cmd).await {
        Ok(_) => R::ok("PTZ 指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/teleboot — 远程重启
pub async fn teleboot(Path(id): Path<String>, srv: DiComp<Gb28181Server>) -> R<String> {
    match srv.teleboot(&id).await {
        Ok(_) => R::ok("重启指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/alarm_reset — 报警复位
#[derive(Deserialize)]
pub struct AlarmResetReq {
    #[serde(default = "default_alarm_type")]
    pub alarm_type: String,
}
fn default_alarm_type() -> String {
    "0".to_string()
}

pub async fn alarm_reset(
    srv: DiComp<Gb28181Server>,
    Path(id): Path<String>,
    ExtJson(req): ExtJson<AlarmResetReq>,
) -> R<String> {
    match srv.alarm_reset(&id, &req.alarm_type).await {
        Ok(_) => R::ok("报警复位指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}
