//! API 路由汇总（版本化 v1，已移除旧版 /api/gb28181 兼容路由）
//!
//! 路由结构：
//!   POST /api/v1/auth/login       — 登录
//!   POST /api/v1/auth/logout      — 登出
//!   GET  /api/v1/auth/info       — 当前用户信息（需认证）
//!   GET  /api/v1/users           — 用户列表
//!   POST /api/v1/users           — 创建用户
//!   GET  /api/v1/users/:id       — 用户详情
//!   PUT  /api/v1/users/:id       — 更新用户
//!   DEL  /api/v1/users/:id       — 删除用户
//!   GET  /api/v1/gb28181/stats          — 统计概要
//!   GET  /api/v1/gb28181/devices        — 设备列表
//!   GET  /api/v1/gb28181/devices/:id    — 设备详情
//!   POST /api/v1/gb28181/devices/:id/catalog    — 目录查询
//!   POST /api/v1/gb28181/devices/:id/info       — 设备信息查询
//!   POST /api/v1/gb28181/devices/:id/status     — 设备状态查询
//!   POST /api/v1/gb28181/devices/:id/ptz        — PTZ 控制
//!   POST /api/v1/gb28181/devices/:id/teleboot   — 远程重启
//!   POST /api/v1/gb28181/devices/:id/alarm_reset — 报警复位
//!   POST /api/v1/gb28181/devices/:id/alarm/*     — 报警订阅/复位
//!   GET|PUT /api/v1/gb28181/alarms/*             — 报警记录 CRUD
//!   POST /api/v1/gb28181/devices/:id/mobile_position/* — 移动位置
//!   GET|POST /api/v1/gb28181/devices/:id/records/*   — 录像查询
//!   POST /api/v1/gb28181/devices/:id/playback/*  — 历史回放+控制
//!   POST /api/v1/gb28181/devices/:id/download/*   — 录像下载
//!   POST /api/v1/gb28181/devices/:id/broadcast/* — 语音广播
//!   POST /api/v1/gb28181/devices/:id/talkback     — 语音对讲
//!   GET  /api/v1/gb28181/sessions              — 会话列表
//!   POST /api/v1/gb28181/sessions              — 发起点播
//!   DEL  /api/v1/gb28181/sessions/:call_id      — 挂断
//!   GET  /api/v1/gb28181/events                — SSE 实时事件
//!
//! ## 中间件架构（sa-token-rust 三层）
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
use toasty::Db;
use tx_di_toasty::ToastyDb;

/// 构建 /api/v1/ 路由树
///
/// 返回 `Router`（即 `Router<()>`），State 已在函数内部通过
/// `.with_state(db)` 注入，调用方无需再处理 State。
///
/// # 参数
/// - `db`: toasty 数据库连接（Clone，线程安全）
/// - `sa_state`: sa-token 认证状态（Clone，内部用 Arc<SaTokenManager>）
pub fn router(db: ToastyDb, sa_state: SaTokenState) -> Router {
    // ════════════════════════════════
    //  公开路由（无需认证）
    //  使用 State<Db>，通过 .with_state(db) 注入
    // ════════════════════════════════
    let public = Router::new()
        // ── 认证 ──
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        // ── GB28181 设备操作 ──
        .route("/gb28181/stats", get(devices::stats))
        .route("/gb28181/devices", get(devices::list))
        .route("/gb28181/devices/{id}", get(devices::detail))
        .route("/gb28181/devices/{id}/catalog", post(devices::query_catalog))
        .route("/gb28181/devices/{id}/info", post(devices::query_info))
        .route("/gb28181/devices/{id}/status", post(devices::query_status))
        .route("/gb28181/devices/{id}/ptz", post(devices::ptz))
        .route("/gb28181/devices/{id}/teleboot", post(devices::teleboot))
        .route("/gb28181/devices/{id}/alarm_reset", post(devices::alarm_reset))
        // ── 报警订阅/复位 ──
        .route("/gb28181/devices/{id}/alarm/subscribe", post(alarm::subscribe_alarm))
        .route("/gb28181/devices/{id}/alarm/reset", post(alarm::reset_alarm))
        // ── 报警记录 CRUD（DB）──
        .route("/gb28181/alarms", get(alarm::list_alarms))
        .route("/gb28181/alarms/{id}", get(alarm::get_alarm))
        .route("/gb28181/alarms/{id}", put(alarm::handle_alarm))
        // ── 移动位置 ──
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
        // ── 会话管理 ──
        .route("/gb28181/sessions", get(sessions::list))
        .route("/gb28181/sessions", post(sessions::invite))
        .route("/gb28181/sessions/{call_id}", delete(sessions::hangup))
        // ── SSE 事件流 ──
        .route("/gb28181/events", get(sse::handler))
        // ── 管理能力增强（Task #13）──
        // 统计仪表盘
        .route("/gb28181/dashboard", get(admin::dashboard))
        // 网络校时
        .route("/gb28181/devices/{id}/time_sync", post(admin::time_sync))
        .route("/gb28181/devices/{id}/sync_time", post(admin::sync_time))
        // 配置管理
        .route("/gb28181/devices/{id}/config", post(admin::query_config))
        // 看守位控制
        .route("/gb28181/devices/{id}/guard/control", post(admin::guard_control))
        .route("/gb28181/devices/{id}/guard/info", post(admin::guard_info))
        .route("/gb28181/devices/{id}/guard/basic", post(admin::guard_basic))
        // 预置位控制
        .route("/gb28181/devices/{id}/preset/goto", post(admin::goto_preset))
        .route("/gb28181/devices/{id}/preset/set", post(admin::set_preset))
        // 巡航控制
        .route("/gb28181/devices/{id}/cruise/start", post(admin::start_cruise))
        .route("/gb28181/devices/{id}/cruise/stop", post(admin::stop_cruise))
        .route("/gb28181/devices/{id}/cruise/list", post(admin::cruise_list))
        .route("/gb28181/devices/{id}/cruise_track", post(admin::cruise_track))
        // 扩展设备控制
        .route("/gb28181/devices/{id}/make_key_frame", post(admin::make_key_frame))
        .route("/gb28181/devices/{id}/zoom/in", post(admin::zoom_in))
        .route("/gb28181/devices/{id}/zoom/out", post(admin::zoom_out))
        .route("/gb28181/devices/{id}/ptz_precise", post(admin::ptz_precise))
        .route("/gb28181/devices/{id}/ptz_precise_status", post(admin::ptz_precise_status))
        .route("/gb28181/devices/{id}/target_track", post(admin::target_track))
        .route("/gb28181/devices/{id}/storage/format", post(admin::storage_format))
        .route("/gb28181/devices/{id}/storage/status", post(admin::storage_status))
        .route("/gb28181/devices/{id}/playback_ctrl", post(admin::playback_ctrl))
        // 审计日志
        .route("/gb28181/audit_logs", get(admin::list_audit_logs))
        .route("/gb28181/audit_logs/{id}", get(admin::get_audit_log))
        // ── 设备分组管理 ──
        .route("/gb28181/groups", get(group::list_groups))
        .route("/gb28181/groups", post(group::create_group))
        .route("/gb28181/groups/{id}", get(group::get_group))
        .route("/gb28181/groups/{id}", put(group::update_group))
        .route("/gb28181/groups/{id}", delete(group::delete_group))
        .route("/gb28181/groups/{id}/members", get(group::list_members))
        .route("/gb28181/groups/{id}/members", post(group::add_members))
        .route("/gb28181/groups/{id}/members/{did}", delete(group::remove_member))
        // ── 注册审核 ──
        .route("/gb28181/register_audit", get(audit::list_audits))
        .route("/gb28181/register_audit/{id}", get(audit::get_audit))
        .route("/gb28181/register_audit/{id}/approve", post(audit::approve))
        .route("/gb28181/register_audit/{id}/reject", post(audit::reject))
        .route("/gb28181/register_audit/{id}", delete(audit::delete_audit))
        .route("/gb28181/register_audit/auto_approve", post(audit::auto_approve))
        .with_state(db.clone());

    // ════════════════════════════════
    //  需要认证的路由
    //
    //  中间件栈（从外到内）：
    //  1. SaTokenLayer          — 从请求头解析 token，注入 extensions
    //  2. SaCheckLoginLayer     — 检查 extensions 中是否有 login_id
    //  3. Handler               — 通过 LoginIdExtractor 获取 login_id
    // ════════════════════════════════
    let protected = Router::new()
        // 用户信息（从 token 提取）
        .route("/auth/info", get(auth::get_info))
        // 用户 CRUD（管理员）
        .route("/users", get(auth::list_users))
        .route("/users", post(auth::create_user))
        .route("/users/{id}", get(auth::get_user))
        .route("/users/{id}", put(auth::update_user))
        .route("/users/{id}", delete(auth::delete_user))
        // 注入 Db State
        .with_state(db.clone())
        // 第1层：解析 token（注入 extensions）
        .layer(SaTokenLayer::new(sa_state.clone()))
        // 第2层：检查是否已登录
        .layer(SaCheckLoginLayer::new());

    // 合并路由（仅保留版本化 v1 路由）
    Router::new()
        .nest("/api/v1", public)
        .nest("/api/v1", protected)
}
