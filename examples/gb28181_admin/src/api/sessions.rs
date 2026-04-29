//! 会话（点播/回放）相关 API

use axum::{
    extract::{Path, Json as ExtJson},
    response::IntoResponse,
};
use serde::Deserialize;
use tx_di_axum::R;
use tx_di_core::ApiR;
use tx_di_gb28181::Gb28181Server;

use crate::dto::SessionDto;

/// GET /api/gb28181/sessions — 活跃会话列表
pub async fn list() -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    let sessions: Vec<SessionDto> = srv.active_sessions().into_iter().map(SessionDto::from).collect();
    R::from(ApiR::success(sessions))
}

/// 点播请求体
#[derive(Deserialize)]
pub struct InviteReq {
    pub device_id: String,
    pub channel_id: String,
}

/// POST /api/gb28181/sessions — 发起实时点播
pub async fn invite(ExtJson(req): ExtJson<InviteReq>) -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    match srv.invite(&req.device_id, &req.channel_id).await {
        Ok((call_id, urls)) => R::from(ApiR::success(serde_json::json!({
            "call_id": call_id,
            "hls":  urls.hls,
            "rtsp": urls.rtsp,
            "rtmp": urls.rtmp,
        }))),
        Err(e) => R::from(ApiR::<serde_json::Value>::error_with_data(
            -1, e.to_string(), serde_json::Value::Null,
        )),
    }
}

/// DELETE /api/gb28181/sessions/:call_id — 挂断
pub async fn hangup(Path(call_id): Path<String>) -> impl IntoResponse {
    let srv = Gb28181Server::instance();
    match srv.hangup(&call_id).await {
        Ok(_) => R::from(ApiR::success("会话已挂断".to_string())),
        Err(e) => R::from(ApiR::<String>::error_with_data(-1, e.to_string(), String::new())),
    }
}
