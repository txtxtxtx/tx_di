# tx_di_toasty

基于 [Toasty ORM](https://github.com/tokio-rs/toasty) 的 tx-di 数据库插件。自动管理数据库连接、连接池、Schema 推送，通过 DI 注入到业务组件。

## 支持数据库

| Feature | 数据库 | 连接串格式 |
|---------|--------|-----------|
| `sqlite`（默认） | SQLite | `sqlite://path/to/db.db` 或 `sqlite://memory` |
| `postgresql` | PostgreSQL | `postgresql://user:pass@host:port/database` |
| `mysql` | MySQL | `mysql://user:pass@host:port/database` |
| `dynamodb` | DynamoDB | `dynamodb://endpoint/region/table_prefix` |

## 安装

```toml
[dependencies]
tx_di_toasty = { path = "plugins/tx_di_toasty", features = ["sqlite"] }
```

## 配置

```toml
# configs/app.toml
[toasty_config]
database_url = "sqlite://app.db"
auto_schema = true              # 启动时自动创建/更新表
max_pool_size = 10              # 连接池大小（默认 num_cpus * 2）
table_name_prefix = ""          # 表名前缀
pool_pre_ping = false           # 取连接前先 ping
```

### 完整配置项

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `database_url` | String | `sqlite://gb28181.db` | 数据库连接串 |
| `auto_schema` | bool | `true` | 启动时自动推送 Schema |
| `max_pool_size` | usize | `num_cpus * 2` | 连接池最大连接数 |
| `table_name_prefix` | String | 无 | 表名前缀 |
| `pool_wait_timeout_secs` | u64 | 无限 | 获取连接最大等待时间 |
| `pool_create_timeout_secs` | u64 | 无 | 建立新连接最大时间 |
| `pool_health_check_interval_secs` | u64 | 60 | 连接健康检查间隔（0 禁用） |
| `pool_max_connection_lifetime_secs` | u64 | 无 | 连接最大存活时间 |
| `pool_max_connection_idle_time_secs` | u64 | 无 | 连接最大空闲时间 |
| `pool_pre_ping` | bool | `false` | 取连接前 ping |
| `default_admin_password` | String | `admin123` | 空库时自动创建的管理员密码 |

## 使用

### 1. 注册模型（build 之前）

```rust
use tx_di_toasty::ToastyPlugin;

// 可多次调用，模型会合并
ToastyPlugin::register_models(toasty::models!(User, Device));
ToastyPlugin::register_models(toasty::models!(AuditLog));
```

### 2. 启动应用

```rust
use tx_di_core::BuildContext;

#[tokio::main]
async fn main() {
    let ctx = BuildContext::new(Some("configs/app.toml"));
    let app = ctx.build().unwrap().ins_run().await.unwrap();

    // 获取数据库实例
    let plugin = app.inject::<ToastyPlugin>();
    let db = plugin.db();
}
```

### 3. 在组件中注入

```rust
use tx_di_core::{tx_comp, App, RIE, CancellationToken};
use tx_di_toasty::{ToastyPlugin, ToastyDb};

#[tx_comp(init)]
pub struct UserService {
    pub toasty: Arc<ToastyPlugin>,
}

impl CompInit for UserService {
    tx_di_core::async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let plugin = ctx.inject::<ToastyPlugin>();
            let db = plugin.db();

            // 使用 db 进行查询
            let users = toasty::stmt!(User::find_all()).collect(db).await?;
            Ok(())
        }
    );

    fn init_sort() -> i32 { 100 }
}
```

### 4. 测试中手动构建

```rust
use tx_di_toasty::{ToastyPlugin, ToastyConfig};

let config = ToastyConfig {
    database_url: "sqlite://:memory:".into(),
    auto_schema: true,
    ..Default::default()
};

let db = ToastyPlugin::build_db_with_models(
    toasty::models!(User),
    &config,
).await?;
```

## API

### ToastyPlugin

| 方法 | 说明 |
|------|------|
| `register_models(models)` | 注册模型（build 前调用，可多次） |
| `db() -> &ToastyDb` | 获取数据库实例（async_init 后） |
| `try_db() -> Option<&ToastyDb>` | 安全获取 |
| `build_db_with_models(models, config)` | 手动构建（测试用） |
| `build_schema(models)` | 仅构建 Schema（迁移工具用） |

### ToastyConfig

从 TOML `[toasty_config]` 自动加载的配置组件。

### ToastyDb

`toasty::Db` 的类型别名，Toasty ORM 的核心数据库实例。
