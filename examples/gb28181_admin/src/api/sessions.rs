//! 会话（点播/回放）相关 API

use axum::{
    extract::{Path, Json as ExtJson},
    response::IntoResponse,
};
use serde::Deserialize;
use tx_di_axum::R;
use tx_di_axum::DiComp;
use tx_di_gb28181::Gb28181Server;

use crate::dto::SessionDto;

/// GET /api/gb28181/sessions — 活跃会话列表
pub async fn list(srv: DiComp<Gb28181Server>) -> impl IntoResponse {
    let sessions: Vec<SessionDto> = srv.active_sessions().into_iter().map(SessionDto::from).collect();
    R::ok(sessions)
}

/// 点播请求体
#[derive(Deserialize)]
pub struct InviteReq {
    pub device_id: String,
    pub channel_id: String,
}

/// POST /api/gb28181/sessions — 发起实时点播
pub async fn invite(srv: DiComp<Gb28181Server>, ExtJson(req): ExtJson<InviteReq>) -> impl IntoResponse {
    match srv.invite(&req.device_id, &req.channel_id).await {
        Ok((call_id, urls)) => R::ok(serde_json::json!({
            "call_id": call_id,
            "urls": urls,
        })),
        Err(e) => R::fail(e.to_string()),
    }
}

/// DELETE /api/gb28181/sessions/:call_id — 挂断
pub async fn hangup(Path(call_id): Path<String>, srv: DiComp<Gb28181Server>) -> impl IntoResponse {
    match srv.hangup(&call_id).await {
        Ok(_) => R::ok("会话已挂断".to_string()),
        Err(e) => R::fail(e.to_string()),
    }
}
