//! 后端事件 -> 前端转发

use tauri::{AppHandle, Emitter};
use tx_di_can::{CanEvent, CanPlugin};

/// 将后端 `CanEvent` 事件总线转发到前端。
///
/// 前端通过 `@tauri-apps/api` 的 `listen("can://event", cb)` 订阅。
/// 必须在任何事件产生之前（即在 `CanPlugin::connect` 之前）注册。
pub fn register_event_forwarder(app: &AppHandle) {
    let handle = app.clone();
    CanPlugin::on_event(move |ev: CanEvent| {
        let h = handle.clone();
        async move {
            // emit 失败（如前端尚未就绪）仅忽略，不阻断链路
            let _ = h.emit("can://event", ev);
            Ok(())
        }
    });
}
