# tx_di_gb28181 — AI 插件速览

> 面向 AI/维护者的快速理解文档。描述本插件的能力边界、公开 API、内部架构与集成要点。
> 权威信息以源码为准（`plugins/tx_di_gb28181/src/`）及公共库 `common/tx_gb28181`。

## 1. 定位

基于 `tx_di_sip` 构建的 **GB28181-2022 上级平台完整实现**。实现设备注册/心跳/目录/查询、实时点播/历史回放/回放控制、PTZ 控制、报警、语音广播/对讲、抓拍、级联等全套国标能力，并通过事件总线向业务层广播。

- 位置：`plugins/tx_di_gb28181/`
- 关键依赖：`tx_di_sip`（SIP 能力）、`tx_gb28181`（公共库：数据类型 `GbDevice`/`ItemType`、XML 解析/构造、SDP 工具、摘要认证、事件/命令枚举）、`tokio`、`dashmap`、`serde_json`、`reqwest`（媒体 HTTP API）
- 编译状态：`cargo check -p tx_di_gb28181` 零警告。

## 2. 模块地图

```
src/
├── lib.rs             # 模块声明 + 公开再导出
├── config.rs          # Gb28181ServerConfig / MediaConfig / CascadeConfig
├── plugin.rs          # Gb28181Server 主体 + SessionInfo + app_async_init + 心跳看门狗
├── plugin_tail.rs     # Gb28181Server 全部主动 API（查询/点播/PTZ/广播/对讲/抓拍/级联…）
├── handlers.rs        # 入站 SIP 处理器注册（REGISTER/MESSAGE/NOTIFY/SUBSCRIBE/OPTIONS）
├── auth.rs            # Gb28181AuthMiddleware：REGISTER 摘要认证 + ACL
├── device_registry.rs # DeviceRegistry：并发设备表
├── event.rs           # re-export tx_gb28181::event（Gb28181Event / emit / subscribe）
├── cascade.rs         # CascadeLower：下级平台向上级注册/心跳/目录推送
├── crypto.rs          # re-export tx_gb28181::utils（nonce/MD5/验证）
├── media/             # 统一流媒体后端抽象（trait + ZLM / MediaMTX / Null 实现）
├── sdp.rs             # re-export tx_gb28181::sdp（SDP 构造/解析）
└── xml.rs             # re-export tx_gb28181::xml（GB28181 XML 构造/解析）
```

> 注意：`sdp.rs` / `xml.rs` 本身**只是 re-export**（SDP/XML 逻辑已下沉到公共库 `tx_gb28181`），修改应去公共库。

## 3. 公开类型与 API（`pub use`，见 `lib.rs`）

### 3.1 核心类型
| 类型 | 角色 |
|------|------|
| `Gb28181Server` | 门面组件（DI 单例），所有主动操作入口 |
| `Gb28181ServerConfig` / `MediaConfig` / `CascadeConfig` | 配置（`#[component(conf)]`，TOML `[gb28181_server_config]`） |
| `DeviceRegistry` | 并发设备注册表（`Arc<DashMap>`，GbDevice 节点） |
| `Gb28181Event` | 27 种事件枚举；`subscribe` / `on_event` 订阅 |
| `Gb28181AuthMiddleware` | REGISTER 摘要认证 + ACL 中间件 |
| `SessionInfo` | 活跃媒体会话（含 `dialog: ClientInviteDialog` 用于真 BYE） |
| `Gb28181CmdType` | MESSAGE CmdType 枚举（re-export） |
| `media::{MediaBackend, ZlmBackend, MediaMtxBackend, NullBackend, OpenRtpRequest, RtpServerHandle, PlayUrls, MediaStreamInfo, StreamProxyHandle, TcpMode, BackendType, MediaBackendConfig, build_backend}` | 流媒体抽象 |
| `sdp::{parse_sdp_ssrc, AudioCodec, AudioSessionInfo, SnapshotInfo}` | SDP 工具 |
| `xml::{...}` | 大量 GB28181 XML 构造/解析函数与类型（PtzCommand / PlaybackControl / ConfigType / CruiseInfo / GuardMode / …） |

### 3.2 `Gb28181Server` 主动 API 分组（plugin_tail.rs）

**查询类**
- `query_catalog(device_id)` — 目录查询
- `query_device_info / query_device_status / query_config / query_preset_list / query_cruise_list / query_guard_info`
- `query_record_info(device, channel, start, end, record_type)`
- `query_cruise_track / query_ptz_precise_status / query_storage_status`
- `time_sync(device_id)` / `sync_time_to_device(device_id)`
- `query_mobile_position(device_id, interval: Option<u32>)` / `unsubscribe_mobile_position`

**点播/回放类**
- `invite(device_id, channel_id) -> (call_id, PlayUrls)` — 实时点播
- `invite_playback(device_id, channel_id, start, end) -> (call_id, PlayUrls)` — 历史回放
- `invite_download(device_id, channel_id, download_speed)` — 录像下载（INVITE s=Download）
- `hangup(call_id)` — **真 BYE**：发 SIP BYE + 释放 RTP 端口 + 触发 `SessionEnded`
- `snapshot(device_id, channel_id) -> image_url` — 抓拍（INVITE s=SnapShot）
- `playback_control(device_id, PlaybackControl)` — 暂停/继续/快放/拖动
- `active_sessions() -> Vec<SessionInfo>`

**控制类**
- `ptz_control(device_id, channel_id, PtzCommand)`
- `ptz_precise_control(device_id, channel_id, PtzPreciseParam)`
- `ptz_lock / ptz_unlock`
- `record_control / guard_control / guard_control_v2 / teleboot / alarm_reset / make_video_record`
- `zoom_in / zoom_out(ZoomRect)`、`target_track`、`storage_format`
- `goto_preset / set_preset / start_cruise / stop_cruise`
- `snapshot_control`、`push_config(device_id, ConfigType, &[(String,String)])`

**语音类**
- `broadcast_invite(device_id)` / `broadcast_accept(device_id, audio_port)` / `broadcast_stop(device_id)`
- `audio_talkback(device_id, channel_id, audio_port, codec) -> (call_id, device_ip, device_audio_port)` — 双向对讲

**报警订阅**
- `subscribe_alarm(device_id, alarm_type, expire)`

**媒体查询**
- `is_streaming(channel_id) / get_play_urls(channel_id) -> PlayUrls`

**注册表查询**
- `get_device / online_devices / device_count / online_count / get_channels`

**事件订阅（plugin.rs）**
- `Gb28181Server::on_event(handler)` — 须在 `ctx.build()` 之前调用
- `Gb28181Server::restore_devices(&app, Vec<GbDevice>)` — 启动时从 DB 恢复设备状态

### 3.3 事件总线（Gb28181Event，部分列举）
`DeviceRegistered / DeviceUnregistered / DeviceOnline / DeviceOffline / Keepalive / CatalogReceived / DeviceInfoReceived / DeviceStatusReceived / RecordInfoReceived / AlarmReceived / MediaStatusNotify / MobilePosition / ConfigDownloaded / PresetListReceived / CruiseListReceived / CruiseTrackReceived / GuardInfoReceived / PtzPreciseStatusReceived / SnapshotTaken / SessionStarted / SessionEnded / AudioTalkbackStarted / AudioTalkbackEnded / BroadcastInviteReceived / BroadcastSessionStarted / BroadcastSessionEnded / TimeSyncResult` 等。

## 4. 配置（`[gb28181_server_config]`）

| 字段 | 默认 | 说明 |
|------|------|------|
| `platform_id` | `"34020000002000000001"` | 本平台 20 位 ID |
| `realm` | `"3402000000"` | 认证域（摘要认证） |
| `sip_ip` | `"127.0.0.1"` | 对外 SIP 服务 IP（构造 URI） |
| `heartbeat_timeout_secs` | `120` | 心跳超时（秒），超时标记离线 |
| `register_ttl` | `3600` | 注册有效期（回写 200 OK Contact Expires） |
| `enable_auth` | `false` | 是否开启 REGISTER 摘要认证 |
| `auth_password` | `"12345678"` | 全局认证密码 |
| `device_passwords` | `{}` | 按 device_id 的独立密码（优先于全局） |
| `allowed_device_ids` / `blocked_device_ids` | `[]` | ACL 白/黑名单（黑名单优先） |
| `media` | — | `MediaConfig { local_ip, rtp_port_start, rtp_port_end }` |
| `cascade` | — | `CascadeConfig { enable_upper(default true), enable_lower, upper_platform_sip, upper_platform_id, upper_auth_password }` |
| `media_backend` | ZLM | `MediaBackendConfig { backend_type, zlm{...}, mediamtx{...} }` |

## 5. 内部架构要点

### 5.1 生命周期
`Gb28181Server` 用 `#[component(app_async_init, init_sort = 10001)]`：
- `app_async_init`：`build_backend(&config.media_backend)` 构建流媒体后端；`register_server_handlers(self)` 注册 SIP 处理器；若 `cascade.enable_lower` 则启动下级级联任务；启动心跳看门狗后台任务。

### 5.2 认证中间件化（auth.rs）
- `Gb28181AuthMiddleware` 实现 `SipMiddleware`，`#[component(as_trait = dyn SipMiddleware)]`，`sort()=10`（最外层前置）。
- 仅对 `REGISTER` 拦截：ACL 黑名单/白名单 → 403；无 `Authorization` → 401 质询（下发 `WWW-Authenticate: Digest` + nonce）；有则校验 MD5 response，失败 403，成功放行并清除 nonce。
- 非 REGISTER 方法直接放行。nonce 存于 `NonceStore`（`DashMap`，随中间件单例常驻）。

### 5.3 真 BYE（迭代已落地）
- `SessionInfo.dialog: ClientInviteDialog`（`rsipstack`，`#[derive(Clone)]` 且 thread-safe，可存 `DashMap` 跨任务）。因其未实现 `Debug`，`SessionInfo` 手写 `Debug` 跳过 dialog。
- `hangup(call_id)`：先 `sessions.remove` → 关闭 RTP 端口 → `sender.bye(&dialog)` 真发 SIP BYE → 触发 `SessionEnded`。
- `invite_internal/audio_talkback` 的 `DialogState::Terminated` 分支用 `sessions_clone.contains_key` 去重，避免 hangup 后 Terminated 重复清理/事件。

### 5.4 流媒体抽象（media/）
- `MediaBackend` trait（`Send+Sync+'static`）：`open_rtp_server / close_rtp_server / is_stream_online / get_play_urls / list_streams / add_stream_proxy / remove_stream_proxy / backend_name / health_check`。
- 实现：`ZlmBackend`（ZLMediaServer HTTP API，默认）、`MediaMtxBackend`（MediaMTX）、`NullBackend`（测试）。
- `build_backend(&MediaBackendConfig) -> Arc<dyn MediaBackend>` 工厂。点播流程：分配 RTP 端口 → 构造 SDP → INVITE → 用 `get_play_urls(stream_id)` 返回多协议播放地址。

### 5.5 级联（cascade.rs）
- `CascadeLower`：`enable_lower=true` 时，本平台作为**下级**向上级注册（含 401 质询 → 提取 nonce → 带 Authorization 重试）、定期续约、注册成功后推送目录 XML。
- `enable_upper=true`（默认）时本平台作为**上级**天然接收下级/设备 REGISTER（由 `handlers.rs` + 认证中间件处理）。

### 5.6 错误处理
`GbErr`（`CodeMsg`，前缀 `"GB"`）：`DeviceNotFound(-1)`、`InvalidUri(-2)`、`InviteFailed(-3)`、`RegisterFailed(-4)`、`UnregisterFailed(-5)`、`MessageSendFailed(-6)`、`MediaApi*（-7~-9）`、`RtpPortFailed(-10)`、`NoAvailablePort(-11)`、`UnsupportedOperation(-12)`。

## 6. 集成要点（给上层业务/示例）

1. 依赖 `tx_di_gb28181`，TOML 配置 `[gb28181_server_config]`（含 `media_backend`）。
2. **订阅事件须在 `ctx.build()` 之前**：`Gb28181Server::on_event(|e| async { ... })`.
3. 启动后从 DI 取 `app.inject::<Gb28181Server>()`，调用 `invite/query_*/ptz_control/...` 等主动 API。
4. 点播拿到 `call_id` 与 `PlayUrls`（`rtsp/rtmp/hls/flv/webrtc` 等），收流用 `RtpServerHandle.port`；结束时 `server.hangup(&call_id)` 释放资源。
5. **消费方注意**：`SessionInfo` 新增字段（如 `dialog`）属于插件内部状态，业务侧通常只关心 `call_id`；不要直接构造 `SessionInfo`/`Gb28181Server`。
6. 大部分 GB28181 XML/SDP 构造与解析逻辑位于公共库 `tx_gb28181`，本插件 `xml.rs`/`sdp.rs` 仅为 re-export——改协议细节应改公共库并同步两个消费方。

## 7. 已知边界
- 认证密码默认全局共享（`auth_password`），生产建议用 `device_passwords` 按设备隔离。
- `handle_keepalive` 对未注册设备心跳仅告警忽略（todo：可触发重新注册）。
- `invite_download` 当前实现与 `invite_playback` 路径相同（未单独区分 Download SDP），属待完善项。
