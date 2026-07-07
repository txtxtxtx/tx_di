# CAN 诊断上位机（前端）

基于 Tauri 2 + Vue 3 + TypeScript + Vite 的桌面端 UI，后端为 `tx_di_can` Rust 引擎。

## 目录结构
- `src-tauri/` — Rust 后端（Tauri 应用入口、命令桥接、事件转发）
- `src/` — Vue 前端（Trace / UDS / Flash / Config 四视图 + `api/can.ts` 调用封装 + `store.ts` 状态）

## 开发
```bash
npm install
npm run tauri dev      # 启动 dev server + 桌面窗口
```

## 构建
```bash
npm run tauri build    # 产出平台安装包
```

## 后端命令（`src-tauri/src/commands.rs`）
`connect` / `disconnect` / `is_connected` / `get_config` / `default_config` /
`send_frame` / `send_fd_frame` / `read_data` / `write_data` / `session_control` /
`ecu_reset` / `tester_present` / `security_access` / `read_dtc` / `flash`

后端事件经 `app.emit("can://event", ev)` 转发，前端用 `listen("can://event", ...)` 订阅
（见 `src/api/can.ts` 与 `src/store.ts`）。
