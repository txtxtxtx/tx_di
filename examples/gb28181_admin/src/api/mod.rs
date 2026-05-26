//! API 路由汇总（版本化 v1，认证分级：open / readonly / write）
//!
//! ## 认证分级
//! | 分组 | 说明 |
//! |------|------|
//! | `open` | 完全公开，无需 token（登录入口、SSE 事件流）|
//! | `readonly` | 需登录，只读操作（设备列表/详情/统计/审计日志等）|
//! | `write` | 需登录，写/控制操作（PTZ/录像/广播/对讲/分组/审核等）|
//!
//! ## 路由清单（精简）
//!   POST /api/v1/auth/login            — 登录（open）
//!   POST /api/v1/auth/logout           — 登出（readonly）
//!   GET  /api/v1/auth/info             — 当前用户信息（readonly）
//!   GET  /api/v1/users                 — 用户列表（readonly）
//!   POST /api/v1/users                 — 创建用户（readonly）
//!   GET  /api/v1/users/:id             — 用户详情（readonly）
//!   PUT  /api/v1/users/:id             — 更新用户（readonly）
//!   DEL  /api/v1/users/:id             — 删除用户（readonly）
//!   GET  /api/v1/gb28181/stats         — 统计概要（readonly）
//!   GET  /api/v1/gb28181/dashboard     — 仪表盘（readonly）
//!   GET  /api/v1/gb28181/devices       — 设备列表（readonly）
//!   GET  /api/v1/gb28181/devices/:id   — 设备详情（readonly）
//!   POST /api/v1/gb28181/devices/:id/catalog   — 目录查询（write）
//!   POST /api/v1/gb28181/devices/:id/ptz       — PTZ 控制（write）
//!   POST /api/v1/gb28181/devices/:id/teleboot  — 远程重启（write）
//!   GET  /api/v1/gb28181/alarms        — 报警列表（readonly）
//!   PUT  /api/v1/gb28181/alarms/:id    — 处理报警（write）
//!   GET  /api/v1/gb28181/sessions      — 会话列表（readonly）
//!   POST /api/v1/gb28181/sessions      — 发起点播（write）
//!   DEL  /api/v1/gb28181/sessions/:id  — 挂断（write）
//!   GET  /api/v1/gb28181/events        — SSE 实时事件（open）
//!   ... 完整列表见源码
//!
//! ## 中间件架构（readonly / write 组）
//!
//! ```text
//! 请求 → SaTokenLayer (解析 token，注入 extensions)
//!     → SaCheckLoginLayer (检查是否已登录, 401 if not)
//!         → Handler (通过 LoginIdExtractor 获取 login_id)
//! ```

pub mod admin;
pub mod alarm;
pub mod audit;
pub mod auth;
pub mod broadcast;
pub mod devices;
pub mod group;
pub mod playback;
pub mod sessions;
pub mod sse;

use axum::Router;
use axum::routing::{get, post, delete, put};
use tx_di_sa_token::{
    SaCheckLoginLayer,
    SaTokenLayer,
    SaTokenState,
};
use tx_di_toasty::ToastyDb;

/// 构建 /api/v1/ 路由树
///
/// 返回 `Router`（即 `Router<()>`），State 已在函数内部通过
/// `.with_state(db)` 注入，调用方无需再处理 State。
///
/// # 路由分组策略
/// - `open`：完全公开，无需任何 token（登录接口、SSE 事件流）
/// - `readonly`：需登录，只读操作（设备列表/详情/统计/审计日志等）
/// - `write`：需登录，写/控制操作（PTZ/录像/报警/广播/对讲/分组/审核等）
///
/// # 参数
/// - `db`: toasty 数据库连接（Clone，线程安全）
/// - `sa_state`: sa-token 认证状态（Clone，内部用 Arc<SaTokenManager>）
pub fn router(db: ToastyDb, sa_state: SaTokenState) -> Router {
    // ════════════════════════════════
    //  完全公开路由（无需认证）
    //  仅保留：登录入口 + SSE 流
    // ════════════════════════════════
    let open = Router::new()
        .route("/auth/login", post(auth::login))
        // SSE 保留公开，便于前端初始化前订阅事件；生产环境可按需迁移到 readonly
        .route("/gb28181/events", get(sse::handler))
        .with_state(db.clone());

    // ════════════════════════════════
    //  只读受保护路由（需登录，GET / 查询）
    // ════════════════════════════════
    let readonly = Router::new()
        // ── 认证 ──
        .route("/auth/logout", post(auth::logout))
        .route("/auth/info", get(auth::get_info))
        // ── 用户管理 ──
        .route("/users", get(auth::list_users))
        .route("/users", post(auth::create_user))
        .route("/users/{id}", get(auth::get_user))
        .route("/users/{id}", put(auth::update_user))
        .route("/users/{id}", delete(auth::delete_user))
        // ── GB28181 只读查询 ──
        .route("/gb28181/stats", get(devices::stats))
        .route("/gb28181/dashboard", get(admin::dashboard))
        .route("/gb28181/devices", get(devices::list))
        .route("/gb28181/devices/{id}", get(devices::detail))
        // ── 报警记录只读 ──
        .route("/gb28181/alarms", get(alarm::list_alarms))
        .route("/gb28181/alarms/{id}", get(alarm::get_alarm))
        // ── 会话只读 ──
        .route("/gb28181/sessions", get(sessions::list))
        // ── 审计日志只读 ──
        .route("/gb28181/audit_logs", get(admin::list_audit_logs))
        .route("/gb28181/audit_logs/{id}", get(admin::get_audit_log))
        // ── 分组只读 ──
        .route("/gb28181/groups", get(group::list_groups))
        .route("/gb28181/groups/{id}", get(group::get_group))
        .route("/gb28181/groups/{id}/members", get(group::list_members))
        // ── 注册审核只读 ──
        .route("/gb28181/register_audit", get(audit::list_audits))
        .route("/gb28181/register_audit/{id}", get(audit::get_audit))
        .with_state(db.clone())
        .layer(SaTokenLayer::new(sa_state.clone()))
        .layer(SaCheckLoginLayer::new());

    // ════════════════════════════════
    //  写/控制受保护路由（需登录，POST / PUT / DELETE 操作）
    // ════════════════════════════════
    let write = Router::new()
        // ── 设备查询指令（向设备发送 SIP MESSAGE）──
        .route("/gb28181/devices/{id}/catalog", post(devices::query_catalog))
        .route("/gb28181/devices/{id}/info", post(devices::query_info))
        .route("/gb28181/devices/{id}/status", post(devices::query_status))
        // ── 设备控制 ──
        .route("/gb28181/devices/{id}/ptz", post(devices::ptz))
        .route("/gb28181/devices/{id}/teleboot", post(devices::teleboot))
        .route("/gb28181/devices/{id}/alarm_reset", post(devices::alarm_reset))
        // ── 报警订阅/复位指令 ──
        .route("/gb28181/devices/{id}/alarm/subscribe", post(alarm::subscribe_alarm))
        .route("/gb28181/devices/{id}/alarm/reset", post(alarm::reset_alarm))
        // ── 报警记录更新 ──
        .route("/gb28181/alarms/{id}", put(alarm::handle_alarm))
        // ── 移动位置订阅 ──
        .route("/gb28181/devices/{id}/mobile_position/query", post(alarm::query_mobile_position))
        .route("/gb28181/devices/{id}/mobile_position/unsubscribe", post(alarm::unsubscribe_mobile_position))
        // ── 录像 / 回放 / 下载 ──
        .route("/gb28181/devices/{id}/records/query", post(playback::query_records))
        .route("/gb28181/devices/{id}/playback/start", post(playback::start_playback))
        .route("/gb28181/devices/{id}/playback/control", post(playback::playback_control))
        .route("/gb28181/devices/{id}/record/control", post(playback::record_control))
        .route("/gb28181/devices/{id}/download/start", post(playback::start_download))
        // ── 广播 / 对讲 ──
        .route("/gb28181/devices/{id}/broadcast/invite", post(broadcast::broadcast_invite))
        .route("/gb28181/devices/{id}/broadcast/accept", post(broadcast::broadcast_accept))
        .route("/gb28181/devices/{id}/broadcast/stop", post(broadcast::broadcast_stop))
        .route("/gb28181/devices/{id}/talkback/start", post(broadcast::start_talkback))
        // ── 会话控制 ──
        .route("/gb28181/sessions", post(sessions::invite))
        .route("/gb28181/sessions/{call_id}", delete(sessions::hangup))
        // ── 网络校时 ──
        .route("/gb28181/devices/{id}/time_sync", post(admin::time_sync))
        .route("/gb28181/devices/{id}/sync_time", post(admin::sync_time))
        // ── 配置管理 ──
        .route("/gb28181/devices/{id}/config", post(admin::query_config))
        // ── 看守位控制 ──
        .route("/gb28181/devices/{id}/guard/control", post(admin::guard_control))
        .route("/gb28181/devices/{id}/guard/info", post(admin::guard_info))
        .route("/gb28181/devices/{id}/guard/basic", post(admin::guard_basic))
        // ── 预置位控制 ──
        .route("/gb28181/devices/{id}/preset/goto", post(admin::goto_preset))
        .route("/gb28181/devices/{id}/preset/set", post(admin::set_preset))
        // ── 巡航控制 ──
        .route("/gb28181/devices/{id}/cruise/start", post(admin::start_cruise))
        .route("/gb28181/devices/{id}/cruise/stop", post(admin::stop_cruise))
        .route("/gb28181/devices/{id}/cruise/list", post(admin::cruise_list))
        .route("/gb28181/devices/{id}/cruise_track", post(admin::cruise_track))
        // ── 扩展设备控制 ──
        .route("/gb28181/devices/{id}/make_key_frame", post(admin::make_key_frame))
        .route("/gb28181/devices/{id}/zoom/in", post(admin::zoom_in))
        .route("/gb28181/devices/{id}/zoom/out", post(admin::zoom_out))
        .route("/gb28181/devices/{id}/ptz_precise", post(admin::ptz_precise))
        .route("/gb28181/devices/{id}/ptz_precise_status", post(admin::ptz_precise_status))
        .route("/gb28181/devices/{id}/target_track", post(admin::target_track))
        .route("/gb28181/devices/{id}/storage/format", post(admin::storage_format))
        .route("/gb28181/devices/{id}/storage/status", post(admin::storage_status))
        .route("/gb28181/devices/{id}/playback_ctrl", post(admin::playback_ctrl))
        // ── 设备分组写操作 ──
        .route("/gb28181/groups", post(group::create_group))
        .route("/gb28181/groups/{id}", put(group::update_group))
        .route("/gb28181/groups/{id}", delete(group::delete_group))
        .route("/gb28181/groups/{id}/members", post(group::add_members))
        .route("/gb28181/groups/{id}/members/{did}", delete(group::remove_member))
        // ── 注册审核写操作 ──
        .route("/gb28181/register_audit/{id}/approve", post(audit::approve))
        .route("/gb28181/register_audit/{id}/reject", post(audit::reject))
        .route("/gb28181/register_audit/{id}", delete(audit::delete_audit))
        .route("/gb28181/register_audit/auto_approve", post(audit::auto_approve))
        .with_state(db.clone())
        .layer(SaTokenLayer::new(sa_state.clone()))
        .layer(SaCheckLoginLayer::new());

    // 合并路由（仅保留版本化 v1 路由）
    Router::new()
        .nest("/api/v1", open)
        .nest("/api/v1", readonly)
        .nest("/api/v1", write)
}
