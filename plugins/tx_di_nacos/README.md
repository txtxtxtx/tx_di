# tx_di_nacos — 配置中心 + 服务注册 + 应用启动循环

**非组件 crate**：不导出任何 `#[derive(Component)]` 结构、不注册进 DI 容器，以普通函数/宏的方式由应用入口使用。基于**官方 `nacos-sdk`（nacos-group/nacos-sdk-rust 0.8）**。

## 能力

1. **配置中心**：启动时从 Nacos 拉取远程配置，与本地 TOML 融合（远程覆盖本地），组件按融合后配置初始化。
2. **配置变更 → 优雅重启**：监听主配置（data_id）变更，优雅关闭当前 App（**进程不退出**），用新配置重启。
3. **服务注册**：收集 HTTP/gRPC 端点注册到 Nacos（SDK 自动心跳，无需手写心跳）。

## 快速开始

`Cargo.toml`：

```toml
tx_di_nacos = { path = "plugins/tx_di_nacos" }
```

`main.rs`（替换原来的 `BuildContext::new(...)` + `ins_run()` + `waiting_exit()`；须在 `#[tokio::main]` 中使用）：

```rust
#[tokio::main]
async fn main() -> AppResult<()> {
    tx_di_nacos::app_loop! {
        config = r"config/config.toml",
        startup = |app: std::sync::Arc<tx_di_core::App>| -> tx_di_core::RIE<()> {
            // ins_run 完成后的业务初始化（job handler 注册、事件订阅等）
            Ok(())
        },
    }
}
```

## 配置（`config.toml` 的 `[registry_config]` 节）

```toml
[registry_config]
enabled = true                 # 主开关：true 启用配置中心 + 服务注册
nacos_addr = "http://127.0.0.1:8848"
namespace = "public"
group = "DEFAULT_GROUP"
service_name = "tx-admin"
auto_register = true
username = "nacos"             # Nacos 登录账号（服务端开启鉴权时必填）
password = "******"            # Nacos 登录密码
config_data_id = "tx-admin.toml"   # 主配置 data_id（默认 "{service_name}.toml"）
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
| `config_data_id` | `Option<String>` | `None`（用 `{service_name}.toml`） |
| `username` | `Option<String>` | `None` |
| `password` | `Option<String>` | `None` |

> **鉴权**：Nacos 服务端开启鉴权后（`nacos.core.auth.enabled=true`）必须配置 `username`/`password`，
> 否则配置发布/拉取会被静默拒绝（返回成功但无数据）。

## 公共 API

| API | 说明 |
|-----|------|
| `app_loop! { config = ..., startup = ... }` | 应用启动循环（拉配置 → 启动 → 注册端点 → 等待 → 优雅关闭 → 重启） |
| `NacosClient::connect(&RegistryConfig)` | 构建单连接客户端（配置中心 + 服务注册） |
| `NacosClient::connect_if_enabled` | 按 `enabled` 开关连接，`false` → `None` |
| `NacosClient::pull_config(data_id)` | 拉远程配置（不存在 → `None`） |
| `NacosClient::merge_config(local, remote)` | TOML 深合并，远程覆盖本地 |
| `NacosClient::watch_config(data_id)` | 监听主配置变更，返回 `watch::Receiver<u64>` |
| `NacosClient::dynamic_config::<T>(data_id)` | 纯查询型业务参数热更新（不触发重启） |
| `NacosClient::register_service(endpoints)` | 注册 http/gRPC 端点，返回 `instance_id` |
| `NacosClient::deregister(instance_id)` | 注销服务实例 |
| `register_endpoints(provider)` | 插件侧声明端点（HTTP/gRPC 插件在 `app_async_init` 中调用） |
| `take_endpoints()` | 取走并清空端点注册表（启动后由宏调用） |
| `load_bootstrap(path)` / `load_local_toml(path)` | 读取本地配置（bootstrap 层） |

**trait 抽象**：`ServiceRegistry`（`register`/`update`/`deregister`/`discover`/`subscribe`）、`ConfigCenter`（`get_config`/`publish_config`/`remove_config`/`listen_config`）、`EndpointProvider`（`get_endpoints()`）。

## 端点注册

HTTP/gRPC 插件在 `app_async_init` 中调用 `register_endpoints` 声明端点；`app_loop!` 在 App 启动完成后通过 `take_endpoints` 统一收集并注册到 Nacos。

```rust
use tx_di_nacos::{register_endpoints, EndpointProvider, Protocol, ServiceEndpoint};

struct MyEndpoints;
impl EndpointProvider for MyEndpoints {
    fn get_endpoints(&self) -> Vec<ServiceEndpoint> {
        vec![ServiceEndpoint {
            protocol: Protocol::Http,
            ip: "0.0.0.0".into(),
            port: 8080,
            metadata: Default::default(),
        }]
    }
}
register_endpoints(Arc::new(MyEndpoints));
```

## 行为语义

| 场景 | 行为 |
|------|------|
| `enabled = true` + Nacos 正常 | 启动拉取 + 合并远程配置 → 启动 → 监听配置变更 → 优雅重启 |
| `enabled = false` | 退化为「本地配置启动一次 + 传统退出」，与旧行为一致 |
| Nacos 启动不可达 | 降级纯本地启动，warn，不阻塞 |
| 远程配置缺失 / 解析失败 | 用本地配置，warn |
| 新配置启动失败 | 宏返回错误，进程退出（由进程管理器拉起，本地 bootstrap 兜底） |
| 重启循环 | 每次先 `graceful_shutdown`（不退出进程）再构建新 App |

## 与 tx-di-core 的配合

- `BuildContext::with_config(toml)` / `AppAllConfig::from_toml_value`：从内存配置构建（宏内部使用）。
- `App::graceful_shutdown()`：优雅关闭当前实例，不退出进程（宏内部调用）。
- `App::wait_exit_signal()`：跨平台退出信号等待（宏内部 select）。

## 注意事项

1. **注册 IP**：容器/多网卡场景通过 `SERVICE_IP` 环境变量指定（默认 127.0.0.1 仅单机调试）。
2. **单连接**：`NacosClient` 在 App 外创建一次，App 启停不影响；App 内部不再创建配置中心连接。
3. **动态配置**：基础设施配置（端口/DB/Nacos 地址）改动走「优雅重启生效」；纯查询型业务参数（限流阈值/开关）用 `dynamic_config` 热更新。
4. **端点注册表为进程级静态**（`ENDPOINT_PROVIDERS`），`take_endpoints` 取走清空，重启不累积。
5. nacos-sdk 写操作语义：服务端不可用时 `register` 等会阻塞，生产需配合快速失败 + 后台重试。
