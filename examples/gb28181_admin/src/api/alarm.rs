//! 报警订阅 / 报警查询 / 移动位置 API
//!
//! ## 功能
//! - **报警订阅**：向设备发送 SUBSCRIBE 订阅报警通知
//! - **报警复位**：清除设备端报警状态
//! - **报警列表**：从数据库查询持久化的报警记录（支持分页/筛选）
//! - **报警处理**：更新报警记录的处理状态和备注
//! - **移动位置查询**：主动请求设备上报 GPS 位置
//! - **移动位置取消订阅**：停止持续位置上报

use axum::{
    extract::{Path, Query, State, Json as ExtJson},
};
use serde::{Deserialize, Serialize};
use tx_di_axum::{DiComp, R};
use tx_di_gb28181::Gb28181Server;
use toasty::Db;

use crate::dto::PageData;
use crate::models::GbAlarmRecord;

// ============ 报警订阅 ============

/// 报警订阅请求体
#[derive(Deserialize)]
pub struct AlarmSubscribeReq {
    /// 报警类型码，0=所有报警
    #[serde(default)]
    pub alarm_type: u8,
    /// 订阅有效期（秒），0=永久
    #[serde(default = "default_expire")]
    pub expire: u32,
}

fn default_expire() -> u32 { 3600 }

/// POST /api/v1/gb28181/devices/:id/alarm/subscribe — 订阅设备报警
///
/// 向设备发送 SUBSCRIBE 指令，订阅指定类型的报警通知。
/// 设备后续会通过 NOTIFY 上报报警事件，触发 `AlarmReceived` SSE 事件。
pub async fn subscribe_alarm(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<AlarmSubscribeReq>,
) -> R<String> {
    match srv.subscribe_alarm(&id, req.alarm_type, req.expire).await {
        Ok(_) => R::ok("报警订阅已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/alarm/reset — 报警复位
///
/// 向设备发送报警复位指令，清除指定类型的报警状态。
#[derive(Deserialize)]
pub struct AlarmResetReq {
    /// 报警类型（如 "1", "2", "All"）
    #[serde(default = "default_alarm_type")]
    pub alarm_type: String,
}
fn default_alarm_type() -> String { "All".to_string() }

pub async fn reset_alarm(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<AlarmResetReq>,
) -> R<String> {
    match srv.alarm_reset(&id, &req.alarm_type).await {
        Ok(_) => R::ok("报警复位指令已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

// ============ 报警记录查询（DB 持久化） ============

/// 报警记录 DTO（序列化友好）
#[derive(Serialize)]
pub struct AlarmDto {
    pub id: i64,
    pub device_id: String,
    pub channel_id: String,
    pub alarm_method: i32,
    pub alarm_type: String,
    pub alarm_level: i32,
    pub description: String,
    pub alarm_time: String,
    pub status: i32,
    pub handler: String,
    pub handle_remark: String,
    pub created_at: String,
}

impl From<GbAlarmRecord> for AlarmDto {
    fn from(r: GbAlarmRecord) -> Self {
        Self {
            id: r.id,
            device_id: r.device_id,
            channel_id: r.channel_id,
            alarm_method: r.alarm_method,
            alarm_type: r.alarm_type,
            alarm_level: r.alarm_level,
            description: r.description,
            alarm_time: r.alarm_time.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            status: r.status,
            handler: r.handler,
            handle_remark: r.handle_remark,
            created_at: r.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

/// 报警查询筛选参数
#[derive(Debug, Deserialize)]
pub struct AlarmQueryParams {
    /// 设备 ID 筛选
    pub device_id: Option<String>,
    /// 报警类型筛选
    pub alarm_type: Option<String>,
    /// 处理状态：0=未处理 1=已确认 2=已处理
    pub status: Option<i32>,
    /// 页码
    #[serde(default = "default_page")]
    pub page: u64,
    /// 每页条数
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}
fn default_page() -> u64 { 1 }
fn default_page_size() -> u64 { 20 }

/// GET /api/v1/gb28181/alarms — 报警记录列表（分页）
///
/// 从 toasty 数据库查询持久化的报警记录。
/// 支持按 device_id、alarm_type、status 筛选。
/// toasty 0.6 的 filter_by_xxx 方法由 #[index] 字段自动生成。
pub async fn list_alarms(
    State(mut db): State<Db>,
    Query(qp): Query<AlarmQueryParams>,
) -> R<PageData<AlarmDto>> {
    // toasty 0.6 查询模式：
    // - 无筛选条件时用 GbAlarmRecord::all()
    // - 有筛选条件时用 GbAlarmRecord::filter_by_xxx(val)
    // - count/exec 均需要 &mut Db
    let total = if qp.device_id.is_some() || qp.alarm_type.is_some() || qp.status.is_some() {
        // 有筛选条件：通过 filter_by 构建查询
        // 注意：toasty 0.6 不支持动态链式 filter，只能按优先级选一个 filter_by 入口
        // 多条件筛选需要逐步缩小结果集（先查再内存过滤），或使用最关键的索引字段
        let base_query = match &qp.device_id {
            Some(did) => GbAlarmRecord::filter_by_device_id(did.clone()),
            None => GbAlarmRecord::all(),
        };
        let count = base_query.count().exec(&mut db).await;
        match count {
            Ok(n) => n,
            Err(e) => return R::error(500, format!("查询报警总数失败: {}", e)),
        }
    } else {
        match GbAlarmRecord::all().count().exec(&mut db).await {
            Ok(n) => n,
            Err(e) => return R::error(500, format!("查询报警总数失败: {}", e)),
        }
    };

    // 分页查询
    let offset = (qp.page.saturating_sub(1)) * qp.page_size as u64;
    let records = if let Some(ref did) = qp.device_id {
        GbAlarmRecord::filter_by_device_id(did.clone())
            .offset(offset as usize)
            .limit(qp.page_size as usize)
            .exec(&mut db)
            .await
    } else {
        GbAlarmRecord::all()
            .offset(offset as usize)
            .limit(qp.page_size as usize)
            .exec(&mut db)
            .await
    };

    let records = match records {
        Ok(r) => r,
        Err(e) => return R::error(500, format!("查询报警列表失败: {}", e)),
    };

    // 内存中二次筛选（alarm_type 和 status 条件）
    let filtered: Vec<AlarmDto> = records
        .into_iter()
        .map(AlarmDto::from)
        .filter(|a| {
            if let Some(ref at) = qp.alarm_type {
                if a.alarm_type != *at { return false; }
            }
            if let Some(s) = qp.status {
                if a.status != s { return false; }
            }
            true
        })
        .collect();

    let total_pages = if qp.page_size == 0 {
        0
    } else {
        (total + qp.page_size - 1) / qp.page_size
    };

    R::ok(PageData {
        items: filtered,
        total,
        page: qp.page,
        page_size: qp.page_size,
        total_pages,
    })
}

/// GET /api/v1/gb28181/alarms/:id — 报警详情
///
/// toasty Model derive 对 #[key] 字段生成 get_by_id(db, key) → Result<Model, Error>
pub async fn get_alarm(
    State(mut db): State<Db>,
    Path(id): Path<i64>,
) -> R<AlarmDto> {
    match GbAlarmRecord::get_by_id(&mut db, id).await {
        Ok(record) => R::ok(AlarmDto::from(record)),
        Err(e) => R::error(404, format!("报警记录不存在或查询失败: {}", e)),
    }
}

/// 报警处理请求体
#[derive(Deserialize)]
pub struct AlarmHandleReq {
    /// 处理状态：1=已确认 2=已处理
    pub status: i32,
    /// 处理人
    #[serde(default)]
    pub handler: String,
    /// 处理备注
    #[serde(default)]
    pub handle_remark: String,
}

/// PUT /api/v1/gb28181/alarms/:id — 处理报警
///
/// 更新报警记录的处理状态、处理人和备注。
/// toasty 更新方式：先 get 到实例，修改字段后 save。
pub async fn handle_alarm(
    State(mut db): State<Db>,
    Path(id): Path<i64>,
    ExtJson(req): ExtJson<AlarmHandleReq>,
) -> R<String> {
    let mut record = match GbAlarmRecord::get_by_id(&mut db, id).await {
        Ok(r) => r,
        Err(e) => return R::error(404, format!("报警记录不存在: {}", e)),
    };

    // 修改字段并保存
    // toasty 0.6 update 模式：record.update() 返回 update builder → .exec(&mut db)
    match record
        .update()
        .status(req.status)
        .handler(req.handler)
        .handle_remark(req.handle_remark)
        .exec(&mut db)
        .await
    {
        Ok(_) => R::ok("报警已处理".to_string()),
        Err(e) => R::error(500, format!("更新失败: {}", e)),
    }
}

// ============ 移动位置 ============

/// 移动位置查询请求体
#[derive(Deserialize)]
pub struct MobilePositionReq {
    /// 上报间隔（秒），None 或 0 = 仅查一次
    #[serde(default)]
    pub interval: Option<u32>,
}

/// POST /api/v1/gb28181/devices/:id/mobile_position/query — 查询移动位置
///
/// 向移动设备发送位置查询 MESSAGE。
/// - interval=None：仅查询一次
/// - interval=N>0：设备按 N 秒间隔持续上报（触发 `MobilePosition` SSE 事件）
pub async fn query_mobile_position(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<MobilePositionReq>,
) -> R<String> {
    match srv.query_mobile_position(&id, req.interval).await {
        Ok(_) => R::ok("位置查询已发送".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}

/// POST /api/v1/gb28181/devices/:id/mobile_position/unsubscribe — 取消位置订阅
///
/// 停止设备的持续位置上报（实际发送 interval=0 的查询指令）。
pub async fn unsubscribe_mobile_position(
    Path(id): Path<String>,
    srv: DiComp<Gb28181Server>,
) -> R<String> {
    match srv.unsubscribe_mobile_position(&id).await {
        Ok(_) => R::ok("位置订阅已取消".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}
