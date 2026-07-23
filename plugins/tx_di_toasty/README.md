# tx_di_toasty — 数据库 ORM 插件使用文档

基于 [Toasty ORM](https://github.com/tokio-rs/toasty) 的 `tx-di` 数据库插件，封装连接、连接池、模型注册与 Schema 推送。

## 用途

- 通过 DI 自动完成数据库初始化；支持 **SQLite / PostgreSQL / MySQL / DynamoDB**（由 `database_url` scheme 自动选驱动）。
- 业务组件注入 `Arc<ToastyPlugin>` 或 `ToastyDb` 即可使用 Toasty 的 Repository/查询 API。

## 启用

`Cargo.toml`（按需开启数据库 feature）：

```toml
tx_di_toasty = { path = "plugins/tx_di_toasty", features = ["sqlite"] }
# 可选: postgresql / mysql / dynamodb（可多选）
```

## 配置

TOML 节名为 `[toasty]`：

```toml
[toasty]
database_url = "sqlite://gb28181.db"
auto_schema = true               # 启动时自动 push_schema 建表
max_pool_size = 10
table_name_prefix = "app_"
pool_pre_ping = false
default_admin_password = "admin123"
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `database_url` | `String` | `"sqlite://gb28181.db"` |
| `auto_schema` | `bool` | `true` |
| `max_pool_size` | `Option<usize>` | `None`（驱动默认） |
| `table_name_prefix` | `Option<String>` | `None` |
| `pool_wait_timeout_secs` | `Option<u64>` | `None` |
| `pool_create_timeout_secs` | `Option<u64>` | `None` |
| `pool_health_check_interval_secs` | `Option<u64>` | `None`（默认 60） |
| `pool_max_connection_lifetime_secs` | `Option<u64>` | `None` |
| `pool_max_connection_idle_time_secs` | `Option<u64>` | `None` |
| `pool_pre_ping` | `bool` | `false` |
| `default_admin_password` | `String` | `"admin123"` |

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `ToastyConfig` | `conf`, `init`, `init_sort = i32::MIN + 2` | 配置载体 |
| `ToastyPlugin` | `app_async_init`, `init_sort = i32::MIN + 2` | 数据库门面 |

`ToastyPlugin` 方法：`register_models(ModelSet)`（build 前，静态合并）、`db() -> &ToastyDb`（async_init 后，未就绪会 panic）、`try_db() -> Option<&ToastyDb>`、`build_db_with_models(...)`（测试）。`type ToastyDb = toasty::Db`。

## 使用方式

```rust
use tx_di_core::{BuildContext, tx_comp};
use tx_di_toasty::{ToastyPlugin, ToastyDb};

// build 之前注册模型
ToastyPlugin::register_models(toasty::models!(User, Device));

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    let app = BuildContext::new::<std::path::PathBuf>(Some("configs/app.toml"))
        .build()?.ins_run().await?;

    let plugin = app.inject::<ToastyPlugin>();
    let db: &ToastyDb = plugin.db();

    let users = toasty::stmt!(User::find_all()).collect(db).await?;
    Ok(())
}
```

## 注意事项

1. **模型注册时序**：必须在 `BuildContext::build()` / `run()` **之前**调用 `ToastyPlugin::register_models(...)`，否则 async_init 时模型为空。可多次调用合并。
2. **`auto_schema` 会回写配置文件**：推送 Schema 成功后会把 `auto_schema = true` 改为 `false`（按行替换、保留注释）。若想每次启动都建表需手动改回/重新部署配置。
3. `db()` 在 async_init 完成前调用会 panic；用 `try_db()` 安全检查。业务组件 `init_sort` 应大于 `i32::MIN + 2` 确保数据库先就绪。
4. **feature 是编译期开关**：启用哪个数据库 feature 只是开启驱动编译；真正选哪种库由运行时 `database_url` scheme 决定。未启用对应 feature 而用该 scheme 会编译/运行时错误。
5. `default_admin_password` 当前**未被实际使用**（未实现空库自动建 admin 逻辑），不应依赖。
6. `auto_schema=true` 即开发期自动 `push_schema()`，并非版本化迁移；生产建议设 `false` 自行管理。
