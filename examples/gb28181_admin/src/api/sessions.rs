//! 会话（点播/回放）相关 API
//!
//! 所有 handler 使用 `DiComp<Gb28181Server>` 从 DI 容器提取 GB28181 服务实例。
//! 返回类型统一为 `R<T>`（而非 `impl IntoResponse`），确保 axum 能正确推断类型。

use axum::{
    extract::{Path, Json as ExtJson},
};
use serde::{Deserialize, Serialize};
use tx_di_axum::{DiComp, R};
use tx_di_gb28181::{Gb28181Server, PlayUrls};

use crate::dto::SessionDto;

/// GET /api/v1/gb28181/sessions — 活跃会话列表
pub async fn list(srv: DiComp<Gb28181Server>) -> R<Vec<SessionDto>> {
    let sessions: Vec<SessionDto> = srv
        .active_sessions()
        .into_iter()
        .map(SessionDto::from)
        .collect();
    R::ok(sessions)
}

/// 点播请求体
#[derive(Deserialize)]
pub struct InviteReq {
    pub device_id: String,
    pub channel_id: String,
}

/// 点播响应体
#[derive(Serialize)]
pub struct InviteResp {
    pub call_id: String,
    pub urls: PlayUrls,
}

/// POST /api/v1/gb28181/sessions — 发起实时点播
pub async fn invite(
    srv: DiComp<Gb28181Server>,
    ExtJson(req): ExtJson<InviteReq>,
) -> R<InviteResp> {
    match srv.invite(&req.device_id, &req.channel_id).await {
        Ok((call_id, urls)) => R::ok(InviteResp { call_id, urls }),
        Err(e) => R::fail(e.to_string()),
    }
}

/// DELETE /api/v1/gb28181/sessions/:call_id — 挂断
pub async fn hangup(
    Path(call_id): Path<String>,
    srv: DiComp<Gb28181Server>,
) -> R<String> {
    match srv.hangup(&call_id).await {
        Ok(_) => R::ok("会话已挂断".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}
