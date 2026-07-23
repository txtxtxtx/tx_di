# tx_di_sa_token — 认证授权插件使用文档

基于 [sa-token-rust](https://github.com/sa-tokens/sa-token-rust) 的登录认证、权限/角色控制、分布式 Session 插件，集成进 `tx-di`。

## 用途

- 登录/登出、Token 校验、权限与角色校验（`StpUtil`）。
- 分布式 Session 后端：内存 / Redis / 数据库（编译 feature 互斥）。
- 与 `tx_di_axum` 集成，提供 axum 路由层级的鉴权 Layer（`SaTokenLayer` 等）。

## 启用

`Cargo.toml`：

```toml
tx_di_sa_token = { path = "plugins/tx_di_sa_token", features = ["memory"] }  # memory/redis/database 三选一
tx_di_axum     = { path = "plugins/tx_di_axum" }   # 硬依赖（提供 Layer 类型）
```

- **storage 后端由编译 feature 决定**：`memory`（默认）/ `redis` / `database` 互斥。
- **不依赖 `tx_di_cache`**：Token 存储由 sa-token 自身根据 feature 提供。

## 配置

TOML 节名为 `[sa_token]`：

```toml
[sa_token]
token_name = "Authorization"
timeout = 86400            # -1 永不过期
active_timeout = -1
auto_renew = false
is_concurrent = true
is_share = true
token_style = "uuid"       # uuid/simple-uuid/random-32/.../jwt
is_read_header = true
is_read_cookie = true
jwt_secret_key = "your-secret"  # token_style="jwt" 时必填
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `token_name` | `String` | `"Authorization"` |
| `timeout` | `i64` | `86400` |
| `active_timeout` | `i64` | `-1` |
| `auto_renew` | `bool` | `false` |
| `is_concurrent` | `bool` | `true` |
| `is_share` | `bool` | `true` |
| `token_style` | `String` | `"uuid"` |
| `is_read_body` | `bool` | `false` |
| `is_read_header` | `bool` | `true` |
| `is_read_cookie` | `bool` | `true` |
| `token_prefix` | `Option<String>` | `None` |
| `is_log` | `bool` | `false` |
| `jwt_secret_key` | `Option<String>` | `None` |
| `jwt_algorithm` | `Option<String>` | `None` |
| `enable_nonce` | `bool` | `false` |
| `nonce_timeout` | `i64` | `-1` |
| `enable_refresh_token` | `bool` | `false` |
| `refresh_token_timeout` | `i64` | `604800` |

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `SaTokenConf` | `conf = "sa_token"`, `init`, `init_sort = i32::MIN + 1` | 配置载体 |
| `SaTokenPlugin` | `init`, `init_sort = i32::MIN + 1` | 门面；`state()`/`build_layer()`/`check_login_layer()`/`check_permission_layer(perm)` |

重导出 `StpUtil`、`SaTokenLayer`、`SaCheckLoginLayer`、`SaCheckPermissionLayer` 等可直接从 `tx_di_sa_token` 引入。

## 使用方式

```rust
use tx_di_sa_token::{SaTokenPlugin, StpUtil};
use tx_di_core::RIE;

// 在异步初始化/业务代码中：
let sa = app.inject::<SaTokenPlugin>();
let state = sa.state();                 // init 之后才可调用

let token = StpUtil::login("user_10001", state)?;
let _is_login = StpUtil::is_login(state);
let user_id = StpUtil::get_login_id(state)?;
StpUtil::set_permission_list(&["user:read", "user:write"], state)?;
StpUtil::check_permission("user:read", state)?;   // 无权限抛错
StpUtil::logout(state)?;
```

Axum 路由层鉴权：

```rust
use axum::Router;
let app = Router::new()
    .route("/api/user/info", get(user_info))
    .layer(sa.check_login_layer())
    .layer(sa.build_layer());
```

## 注意事项

1. **存储后端由 feature 决定**：`redis`/`database` 后端通过环境变量 `REDIS_URL`（默认 `redis://127.0.0.1:6379`）/`DATABASE_URL`（默认 `sqlite://sa_token.db`）读取连接串，**尚未从 TOML 读取**（源码有 TODO）。
2. `SaTokenState` 延迟初始化（`OnceLock`）：`init` 阶段构建，在此之前调用 `state()` 会 panic，可用 `try_state()` 安全检查。
3. **拦截器未自动挂载**：插件不会自动把 Layer 挂到 axum，必须手动在 `Router` 上 `.layer(...)`。
4. `token_style = "jwt"` 时必须设置 `jwt_secret_key`，否则无法签发/校验。
5. `init_sort = i32::MIN + 1`，保证 SaToken 在其他业务组件之前最早初始化。
