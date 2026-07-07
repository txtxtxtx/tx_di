// CAN 诊断上位机 — Tauri 应用入口
//
// 架构：Rust 后端（tx_di_can）通过 Tauri command 暴露给前端，
// 后端的 CanEvent 事件总线通过 `app.emit("can://event", ev)` 转发到前端，
// 前端用 `@tauri-apps/api` 的 `listen("can://event", ...)` 订阅。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod events;

use tx_di_can::CanPlugin;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 1) 注册事件转发：后端 CanEvent -> 前端 listen("can://event")
            events::register_event_forwarder(app.handle());

            // 2) 默认以仿真总线启动（无需硬件即可运行；用户可在 UI 中切换真实适配器）
            let cfg = CanPlugin::default_config();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = CanPlugin::connect(cfg).await {
                    eprintln!("[can-host] 默认连接失败: {e}");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::default_config,
            commands::get_config,
            commands::is_connected,
            commands::connect,
            commands::disconnect,
            commands::send_frame,
            commands::send_fd_frame,
            commands::read_data,
            commands::write_data,
            commands::session_control,
            commands::ecu_reset,
            commands::tester_present,
            commands::security_access,
            commands::read_dtc,
            commands::flash,
            commands::get_bus_stats,
            commands::reset_stats,
            commands::set_frame_filter,
            commands::get_frame_filter,
            commands::send_isotp,
            commands::get_desc_dids,
            commands::get_desc_dtcs,
            commands::sim_ecu_status,
            commands::record_csv,
            commands::replay_csv,
            commands::load_dbc,
            commands::decode_dbc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
