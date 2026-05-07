//! SSE 实时事件推送

use axum::response::sse::{Event, Sse};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;

use crate::device::DeviceManager;

/// GET /api/gb_cams/events — SSE 端点
pub async fn handler() -> Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>> {
    let mgr = DeviceManager::instance();
    let rx = mgr.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            result.ok().map(|ev| {
                let json = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string());
                Ok(Event::default().data(json))
            })
        });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}
