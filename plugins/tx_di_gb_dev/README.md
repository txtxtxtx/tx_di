# tx_di_gb_dev — GB28181 设备端插件使用文档

GB/T 28181 **设备端（UAC，向上级平台注册的设备/模拟设备）插件**，对应分层架构的 `L2b` 层。

## 用途

- 把"设备向上级平台注册 + 周期心跳保活 + 应答平台下发的查询/控制 + 点播 INVITE/BYE 响应"封装为 tx_di 组件，**业务零改造**。
- 设备侧只需实现 `DeviceHandler` trait 回调，由 `Gb28181Device` 统一完成 REGISTER、Keepalive、目录/设备信息/状态查询响应、PTZ 控制、校时响应、实时流点播（INVITE/BYE）。
- 供模拟多通道摄像机、平台级联下级（CascadeLower）复用。

> **依赖关系澄清**：本插件依赖 `common/tx_gb28181`（纯国标协议库）与 `tx_di_sip`，**不依赖** `plugins/tx_di_gb28181`（服务端插件）。

## 启用

`Cargo.toml`：

```toml
tx_di_gb_dev = { path = "plugins/tx_di_gb_dev" }
tx_di_sip    = { path = "plugins/tx_di_sip" }   # 必需
```

## 配置

TOML 节名为 `[gb_dev]`：

```toml
[gb_dev]
platform_uri   = "sip:34020000002000000001@192.168.1.1:5060"
device_id      = "34020000001320000001"
username       = "34020000001320000001"
password       = "12345678"
realm          = "3402000000"
register_ttl   = 3600
heartbeat_secs = 60
version        = "v2022"     # 或 "v2016"
enabled        = true
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `platform_uri` | `String` | `""`（上级平台 SIP URI，身兼注册服务器与出网 MESSAGE 目标） |
| `device_id` | `String` | `""` |
| `username` | `String` | `""` |
| `password` | `String` | `""` |
| `realm` | `Option<String>` | `None`（接受任意挑战） |
| `register_ttl` | `u32` | `3600` |
| `heartbeat_secs` | `u32` | `60`（运行时取 `max(5, ...)`） |
| `version` | `GbVersion`（`v2016`/`v2022`） | `V2022` |
| `enabled` | `bool` | `false` |

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `GbDevConfig` | `conf = "gb_dev"` | 设备端配置 |
| `Gb28181Device` | `app_async_init`, `app_async_run`, `shutdown`, `init_sort = 30000` | 设备门面：注册/心跳/MESSAGE/INVITE/BYE 响应 |

非 Component 类型：`DeviceHandler` trait（所有方法默认空实现）、`NoopDeviceHandler`（未注入 handler 时兜底）。

## 使用方式

```rust
use tx_di_gb_dev::{Gb28181Device, DeviceHandler};
use async_trait::async_trait;
use tx_di_core::{BuildContext, app, Component};

app! { AppModule }

struct MyCam;
#[async_trait]
impl DeviceHandler for MyCam {
    async fn on_catalog(&self, _sn: u32) -> Vec<(String, String)> {
        vec![("34020000001320000001".into(), "前门摄像机".into())]
    }
}

#[derive(Component)]
pub struct CamService { pub dev: std::sync::Arc<Gb28181Device> }

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    // TOML 配置 [gb_dev] enabled = true，且 tx_di_sip 已注册
    let app = BuildContext::new::<std::path::PathBuf>(Some("configs/gb_dev.toml"))
        .build_and_run().await?;
    Ok(())
}
```

运行后组件自动：注册 → 每 `heartbeat_secs` 续期+心跳 → 收到平台 MESSAGE/INVITE/BYE 时回调 `DeviceHandler` 并回网 → 退出时注销。

## 注意事项

1. **`enabled = false` 是默认**，且配置段可整体缺省（退化为 `enabled=false` 不执行注册/心跳，但处理器仍注册）。务必显式 `enabled = true`。
2. **`platform_uri` 身兼两职**：取 `@` 之后作为注册服务器，并作为出网 MESSAGE 目标 URI，格式必须正确。
3. 心跳间隔下限 5 秒；初次注册失败不致命，后续周期重试。
4. MESSAGE 先回 200 OK 再异步处理业务；空 body / 无 `CmdType` 直接忽略。
5. PTZ 控制不回网（仅触发 `on_ptz` 副作用）。
6. 校时响应自动生成，不回调 `on_device_status`。
7. INVITE 为事务级响应（骨架级），不含完整 dialog 状态机；媒体流需业务侧另行实现。
8. **出网编码随 `version` 变化**：`v2016`→GB2312/GBK，`v2022`→GB18030。
9. 未注入 `DeviceHandler` 时用 `NoopDeviceHandler` 兜底（目录/信息返回空）——通常需要实现 `on_catalog` 才有意义。
