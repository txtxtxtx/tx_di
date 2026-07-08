# tx_di_registry — 服务注册发现与配置中心插件使用文档

提供统一的**服务注册/发现**（`ServiceRegistry` trait）与**配置中心**（`ConfigCenter` trait）抽象，默认后端为 Nacos。

## 用途

- 服务实例注册、心跳保活、发现、订阅。
- 配置热更新监听（`listen_config`）。
- HTTP/gRPC 双协议端点自动收集并注册到注册中心。
- 内置 `DynamicConfig<T>` 通用配置容器（基于 `tokio::sync::watch` 热更新）。

> ⚠️ **当前 Nacos 后端为 TODO 占位实现**：`NacosServiceRegistry` / `NacosConfigCenter` 的方法体仅打印日志，并未真正接入 `nacos_rust_client`。`discover` 恒返回空、`get_config` 恒返回 `None`、`listen_config` 内部 `pending()` 会永久阻塞订阅任务。生产接入前需补全实现。

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

1. **Nacos 后端为占位**：见上方用途警告，当前不要依赖其真实能力。
2. `enabled = false` 时所有回调提前返回，`get_registry()`/`get_config_center()` 返回 `None`。
3. `shutdown()` 仅打印日志，未实现真正的 `deregister`。
4. 端点注册表为进程级全局静态（`ENDPOINT_PROVIDERS`），跨 App 实例共享。
5. `init_sort` 顺序：`RegistryConfig = i32::MIN`（最早），`RegistryPlugin = i32::MAX - 50`（很晚，确保端点已注册才收集）。
6. `DynamicConfig<T>` 是独立通用工具，目前与 Nacos 监听未接线，可单独使用其 `update`/`subscribe`。
