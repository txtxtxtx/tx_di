# tx_di_gb28181 — GB28181 国标视频监控服务端插件使用文档

基于 `tx_di_sip` 构建的 **GB/T 28181 国标视频监控完整服务端（上级平台）插件**。

## 用途

- 实现国标 SIP 信令中心：设备 REGISTER/注销/心跳、目录/设备信息/状态/录像查询、实时点播（INVITE s=Play）、历史回放（s=Playback）及回放控制、PTZ 云台控制、布撤防/看守位、报警订阅与接收、媒体状态通知、移动位置上报。
- 通过 `MediaBackend` 抽象联动 ZLMediaServer / MediaMTX 完成 RTP 收流与播放 URL 生成。
- 支持上下级平台级联（Cascade）。
- 通过 `Gb28181Event` 27 种事件总线向上层业务暴露。

> **依赖 `tx_di_sip`**：必须同时在 App 中启用 `tx_di_sip` 插件。

## 启用

`Cargo.toml`：

```toml
tx_di_gb28181 = { path = "plugins/tx_di_gb28181" }
tx_di_sip     = { path = "plugins/tx_di_sip" }
```

## 配置

TOML 节名为 `[gb28181_server_config]`：

```toml
[gb28181_server_config]
platform_id = "34020000002000000001"
realm = "3402000000"
sip_ip = "127.0.0.1"
heartbeat_timeout_secs = 120
register_ttl = 3600
enable_auth = false
auth_password = "12345678"

[gb28181_server_config.media]
local_ip = "0.0.0.0"
rtp_port_start = 30000
rtp_port_end = 30500
nat_external_ip = "203.0.113.10"

[gb28181_server_config.media_backend]
backend_type = "zlm"
[gb28181_server_config.media_backend.zlm]
base_url = "http://127.0.0.1:8080"
secret = "035c73f7-bb6b-4889-a715-d9eb2d1925cc"

[gb28181_server_config.cascade]
enable_upper = true
enable_lower = false
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `platform_id` | `String` | `"34020000002000000001"` |
| `realm` | `String` | `"3402000000"` |
| `sip_ip` | `String` | `"127.0.0.1"` |
| `heartbeat_timeout_secs` | `u64` | `120` |
| `register_ttl` | `u32` | `3600` |
| `enable_auth` | `bool` | `false` |
| `auth_password` | `String` | `"12345678"` |
| `device_passwords` | `HashMap<String,String>` | `{}` |
| `allowed_device_ids` | `Vec<String>` | `[]`（白名单） |
| `blocked_device_ids` | `Vec<String>` | `[]`（黑名单，优先） |
| `default_version` | `GbVersion` | `V2022` |
| `device_versions` | `HashMap<String, GbVersion>` | `{}` |
| `media.local_ip` | `String` | `"0.0.0.0"` |
| `media.rtp_port_start` | `u16` | `30000` |
| `media.rtp_port_end` | `u16` | `30500` |
| `media.nat_external_ip` | `Option<String>` | `None` |
| `cascade.enable_upper` | `bool` | `true` |
| `cascade.enable_lower` | `bool` | `false` |
| `media_backend.backend_type` | `BackendType`（`zlm`/`mediamtx`/`null`） | `Zlm` |

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `Gb28181ServerConfig` | `conf` | 平台配置（realm/认证/ACL/媒体/级联） |
| `Gb28181Server` | `app_async_init`, `init_sort = 10001` | 服务端门面：`query_catalog`/`invite`/`ptz_control`/`hangup`/`on_event`/`restore_devices` |
| `Gb28181AuthMiddleware` | `as_trait = dyn SipMiddleware` | REGISTER 摘要认证 + ACL（洋葱链最外层 sort=10） |

其他重要类型：`DeviceRegistry`（并发设备表）、`Gb28181Event`（事件总线，`on_event` 订阅）、`MediaBackend` trait。

## 使用方式

```rust
use tx_di_gb28181::{Gb28181Server, Gb28181Event};
use tx_di_core::{BuildContext, app};

app! { AppModule }

// 必须在 build 之前订阅事件
fn sub() {
    Gb28181Server::on_event(|event| async move {
        match event {
            Gb28181Event::DeviceRegistered { device_id, .. } =>
                tracing::info!("设备上线: {}", device_id),
            _ => {}
        }
        Ok(())
    });
}

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    sub();
    let mut ctx = BuildContext::new::<std::path::PathBuf>(Some("configs/gb28181-server.toml"));
    let app = ctx.build_and_run().await?;

    let server = app.inject::<Gb28181Server>();
    let (call_id, urls) = server.invite("34020000001320000001", "34020000001320000001").await?;
    println!("HLS: {}", urls.hls);
    server.hangup(&call_id).await?;
    Ok(())
}
```

设备通过 SIP REGISTER 自动注册到 `DeviceRegistry`，上层通过 `DeviceRegistered` 事件感知。axum handler 中可用 `DiComp<Gb28181Server>` 注入。

## 注意事项

1. **init_sort = 10001**：`Gb28181Server` 依赖 `SipPlugin`(10000) 已运行，确保 SIP 路由器先就绪。
2. **认证中间件仅拦截 REGISTER**：INVITE/MESSAGE 不受摘要认证保护（依赖 REGISTER 时身份绑定）；`enable_auth=false` 直接放行。
3. **密码模型简化**：默认共用 `auth_password`，可用 `device_passwords` 按 device_id 覆盖（生产建议替换为按库查密码）。
4. **ACL 优先级**：黑名单 > 白名单 > 其余放行。
5. **媒体端口范围**默认 `30000~30500`；`port=0` 交 ZLM 自动分配。NAT 用 `nat_external_ip` 覆盖 SDP 出网媒体地址。
6. **`media_backend` 优先于旧 `zlm` 配置**：统一用 `[gb28181_server_config.media_backend]`。
7. **协议版本影响字符集**：`V2016`→GB2312，`V2022`→GB18030。
8. **事件订阅时机**：`Gb28181Server::on_event` 必须在 `build()` 之前调用，否则不生效。
9. `hangup(call_id)` 发送真实 SIP BYE 并释放 RTP 端口。
