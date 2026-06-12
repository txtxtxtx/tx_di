# tx_di_sa_token

基于 [sa-token-rust](https://github.com/sa-tokens/sa-token-rust) 的 tx-di 认证授权插件。提供登录认证、权限/角色控制、分布式 Session，与 axum 无缝集成。

## 安装

```toml
[dependencies]
tx_di_sa_token = { path = "plugins/tx_di_sa_token", features = ["memory"] }
```

### 存储后端

| Feature | 说明 | 适用场景 |
|---------|------|---------|
| `memory`（默认） | 内存存储 | 开发、测试 |
| `redis` | Redis 存储 | 生产环境，分布式部署 |
| `database` | 数据库存储 | 需要持久化 |

三个 feature 互斥，只能选一个。

## 配置

```toml
# configs/app.toml
[sa_token_config]
token_name = "Authorization"     # Token 键名（Header/Cookie/Parameter）
timeout = 86400                  # Token 有效期（秒），-1 永不过期
active_timeout = -1              # 最低活跃频率（秒），-1 不限制
auto_renew = false               # 是否自动续签
is_concurrent = true             # 是否允许同一账号并发登录
is_share = true                  # 多人登录同一账号时是否共享 Token
token_style = "uuid"             # Token 风格
is_read_header = true            # 从 Header 读取 Token
is_read_cookie = true            # 从 Cookie 读取 Token
is_read_body = false             # 从请求体读取 Token
token_prefix = ""                # Token 前缀，如 "Bearer "
is_log = false                   # 是否输出操作日志
jwt_secret_key = ""              # JWT 密钥（token_style = "jwt" 时使用）
jwt_algorithm = "HS256"          # JWT 算法
enable_nonce = false             # 是否启用防重放攻击
enable_refresh_token = false     # 是否启用 Refresh Token
refresh_token_timeout = 604800   # Refresh Token 有效期（秒，默认 7 天）
```

### Token 风格

| 值 | 说明 |
|----|------|
| `uuid` | UUID 格式（默认） |
| `simple-uuid` | 简化 UUID（无横线） |
| `random-32` | 32 位随机字符串 |
| `random-64` | 64 位随机字符串 |
| `random-128` | 128 位随机字符串 |
| `jwt` | JWT 格式 |
| `hash` | 哈希值 |
| `tik` | tik 风格 |
| `timestamp` | 时间戳风格 |

## 使用

### 登录与鉴权

```rust
use tx_di_sa_token::{SaTokenPlugin, StpUtil};

#[tx_comp(init)]
pub struct AuthService {
    pub sa: Arc<SaTokenPlugin>,
}

impl CompInit for AuthService {
    tx_di_core::async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let sa = ctx.inject::<SaTokenPlugin>();
            let state = sa.state();

            // 登录
            let token = StpUtil::login("user_10001", state)?;

            // 检查登录
            let is_login = StpUtil::is_login(state);

            // 获取登录 ID
            let user_id = StpUtil::get_login_id(state)?;

            // 注销
            StpUtil::logout(state)?;
            Ok(())
        }
    );
    fn init_sort() -> i32 { 100 }
}
```

### 权限与角色

```rust
use tx_di_sa_token::StpUtil;

// 设置权限（登录后）
StpUtil::set_permission_list(&["user:read", "user:write"], state)?;

// 设置角色
StpUtil::set_role_list(&["admin", "manager"], state)?;

// 检查权限
StpUtil::check_permission("user:read", state)?;    // 无权限则抛错
StpUtil::has_permission("user:read", state);        // 返回 bool

// 检查角色
StpUtil::check_role("admin", state)?;
StpUtil::has_role("admin", state);

// 批量检查
StpUtil::check_permission_and(&["user:read", "user:write"], state)?;  // 全部满足
StpUtil::check_permission_or(&["user:read", "user:delete"], state)?;  // 满足其一
```

### Axum 路由鉴权

```rust
use tx_di_sa_token::SaTokenPlugin;
use axum::{Router, routing::get, middleware};

#[tx_comp(init)]
pub struct WebServer {
    pub sa: Arc<SaTokenPlugin>,
}

impl CompInit for WebServer {
    tx_di_core::async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let sa = ctx.inject::<SaTokenPlugin>();

            let app = Router::new()
                // 全局登录检查
                .route("/api/user/info", get(user_info))
                .layer(sa.check_login_layer())
                // 特定路由权限检查
                .route("/api/admin/users", get(list_users))
                .layer(sa.check_permission_layer("admin:user:list"))
                // 或使用 SaTokenLayer 做整体鉴权
                .layer(sa.build_layer());

            Ok(())
        }
    );
    fn init_sort() -> i32 { 200 }
}
```

### 路径鉴权配置

```rust
use tx_di_sa_token::sa_token_plugin_axum::sa_token_core::router::PathAuthConfig;

let path_auth = PathAuthConfig::new()
    .add_include_pattern("/api/**")          // 需要登录的路径
    .add_exclude_pattern("/api/public/**");  // 排除的路径

let layer = sa.build_layer_with_path_auth(path_auth);
```

## API

### SaTokenPlugin

| 方法 | 说明 |
|------|------|
| `state()` | 获取 `SaTokenState`（async_init 后） |
| `try_state()` | 安全获取 |
| `build_layer()` | 构建 Axum `SaTokenLayer` |
| `build_layer_with_path_auth(config)` | 带路径鉴权的 Layer |
| `check_login_layer()` | 登录检查 Layer |
| `check_permission_layer(perm)` | 权限检查 Layer |

### StpUtil（重导出）

| 方法 | 说明 |
|------|------|
| `login(id, state)` | 登录，返回 Token |
| `logout(state)` | 注销 |
| `is_login(state)` | 是否已登录 |
| `get_login_id(state)` | 获取登录 ID |
| `check_permission(perm, state)` | 检查权限（无权限抛错） |
| `has_permission(perm, state)` | 是否有权限（返回 bool） |
| `check_role(role, state)` | 检查角色 |
| `set_permission_list(list, state)` | 设置权限列表 |
| `set_role_list(list, state)` | 设置角色列表 |
