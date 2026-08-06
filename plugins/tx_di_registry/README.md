# tx_di_registry — 服务注册发现与配置中心插件使用文档

提供统一的**服务注册/发现**（`ServiceRegistry` trait）与**配置中心**（`ConfigCenter` trait）抽象，Nacos 后端基于**官方 `nacos-sdk`（nacos-group/nacos-sdk-rust 0.8）**实现。

## 用途

- 服务实例注册、**自动心跳保活**（nacos-sdk gRPC 双工长连接，无需手写心跳）、发现、订阅。
- 配置热更新监听（`listen_config`）。
- HTTP/gRPC 双协议端点自动收集并注册到注册中心。
- 内置 `DynamicConfig<T>` 通用配置容器（基于 `tokio::sync::watch` 热更新）。

> ✅ **Nacos 后端已实现（官方 nacos-sdk 0.8）**：`NacosServiceRegistry`（`register`/`deregister`/`discover`/`subscribe`，注册后自动心跳）、`NacosConfigCenter`（`get_config`/`publish_config`/`remove_config`/`listen_config`）均已接入真实能力；`ConfigWatcher` 负责监听远端配置变更（变更回调反序列化→`DynamicConfig` 热更新链路的回调落点已预留）。

## 启用

`Cargo.toml`：

```toml
tx_di_registry = { path = "plugins/tx_di_registry" }            # 默认已含 nacos feature
# tx_di_registry = { path = "plugins/tx_di_registry", default-features = false } # 关闭 nacos
```

## 配置

TOML 节名为 `[registry_config]`：

```toml
[registry_config]
enabled = true                 # 主开关，默认 false
nacos_addr = "http://127.0.0.1:8848"
namespace = "public"
group = "DEFAULT_GROUP"
service_name = "my-service"
auto_register = true           # 是否自动注册本地端点
heartbeat_secs = 5             # 心跳间隔(秒)
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `enabled` | `bool` | `false` |
| `nacos_addr` | `String` | `"http://127.0.0.1:8848"` |
| `namespace` | `String` | `"public"` |
| `group` | `String` | `"DEFAULT_GROUP"` |
| `service_name` | `String` | `"unknown-service"` |
| `auto_register` | `bool` | `true` |
| `heartbeat_secs` | `u64` | `5` |

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `RegistryConfig` | `conf`, `init`, `init_sort = i32::MIN` | 配置载体 |
| `RegistryPlugin` | `app_async_init`, `app_async_run`, `shutdown`, `init_sort = i32::MAX - 50` | 注册/配置中心门面 |

**trait 抽象**：`ServiceRegistry`（`register`/`update`/`deregister`/`discover`/`subscribe`）、`ConfigCenter`（`get_config`/`publish_config`/`remove_config`/`listen_config`）、`EndpointProvider`（`get_endpoints()`）。

**数据模型**：`Protocol`（`Http`/`Grpc`）、`ServiceEndpoint { protocol, ip, port, metadata }`、`ServiceInstance { service_name, instance_id, endpoints, healthy, metadata }`。

`RegistryPlugin` 方法：`get_registry() -> Option<&Arc<dyn ServiceRegistry>>`、`get_config_center() -> Option<&Arc<dyn ConfigCenter>>`。

## 使用方式

```rust
use std::sync::Arc;
use tx_di_core::{BuildContext, App};
use tx_di_registry::{RegistryPlugin, ServiceRegistry, ConfigCenter};

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    let ctx = BuildContext::new::<std::path::PathBuf>(Some("configs/registry_config.toml"));
    let app = Arc::new(ctx.build()?);
    let app = app.ins_run().await?;

    let plugin = app.inject::<RegistryPlugin>();
    if let Some(reg) = plugin.get_registry() {
        let instances = reg.discover("other-service").await?;
        println!("发现实例数: {}", instances.len());
    }
    if let Some(cc) = plugin.get_config_center() {
        if let Some(cfg) = cc.get_config("my-service.yaml", "DEFAULT_GROUP").await? {
            println!("当前配置: {}", cfg);
        }
    }
    Ok(())
}
```

端点注册（HTTP/gRPC 插件侧，在 `app_async_init` 阶段调用）：

```rust
use std::sync::Arc;
use tx_di_registry::{register_endpoints, EndpointProvider, ServiceEndpoint, Protocol};

struct MyEndpoints;
impl EndpointProvider for MyEndpoints {
    fn get_endpoints(&self) -> Vec<ServiceEndpoint> {
        vec![ServiceEndpoint { protocol: Protocol::Http, ip: "0.0.0.0".into(), port: 8080, metadata: Default::default() }]
    }
}
register_endpoints(Arc::new(MyEndpoints));
```

## 注意事项

1. `enabled = false` 时所有回调提前返回，`get_registry()`/`get_config_center()` 返回 `None`。
2. `shutdown()` 已实现显式 `deregister`（`tokio::runtime::Handle::block_on` 异步注销；无 runtime 上下文时降级为心跳超时剔除）。
3. 心跳保活由 nacos-sdk 的 gRPC 双工长连接自动完成，插件侧无额外心跳。
4. 端点注册表为进程级全局静态（`ENDPOINT_PROVIDERS`），跨 App 实例共享。
5. `init_sort` 顺序：`RegistryConfig = i32::MIN`（最早），`RegistryPlugin = i32::MAX - 50`（很晚，确保端点已注册才收集）。
6. `DynamicConfig<T>` 可独立使用 `update`/`subscribe`；与 Nacos 监听的接线点在 `ConfigWatcher`（`listen_config` 回调目前打印日志，反序列化→`DynamicConfig.update` 待业务接入）。
7. **nacos-sdk 写操作语义**：服务端不可用时 `register` 等会阻塞（SDK 正确行为），生产需配合快速失败 + 后台重试。
8. **注册 IP**：容器/多网卡场景通过 `SERVICE_IP` 环境变量指定（默认 127.0.0.1 仅单机调试）。
