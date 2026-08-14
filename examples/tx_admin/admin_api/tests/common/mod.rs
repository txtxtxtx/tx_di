//! E2E 回归测试公共基座：App 工厂 + HTTP 客户端 + 登录辅助
//!
//! ## 约定
//! - 每个测试文件独立进程；单文件内所有用例共享**一个** App（`OnceCell` 缓存），
//!   避免 `ROUTER_REGISTRY` / `LAYER_REGISTRY` / sa-token 全局状态等进程级单例冲突。
//! - 测试配置：内存 SQLite（`sqlite://memory`，单连接）+ 关闭 Nacos + 随机端口。
//! - 配置节名使用宏实际读取的 key（`[toasty]`/`[log]`/`[file]`/`[job]`/`[sa_token]`/
//!   `[web_config]`），而非 `[xxx_config]` 后缀形式。
#![allow(dead_code)]

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::OnceCell;
use tx_di_core::App;

// 空导入：触发 admin_api lib 链接（AdminPlugin 等组件 linkme 注册）
#[allow(unused_imports)]
use admin_api;

/// 测试服务器句柄
pub struct TestServer {
    /// HTTP 基地址（如 `http://127.0.0.1:53271`）
    pub base_url: String,
    /// gRPC 基地址（如 `http://127.0.0.1:53272`），供 gRPC 客户端连接
    pub grpc_url: String,
    /// 已启动的 App（init + async_init 已完成，async_run 后台运行）
    pub app: Arc<App>,
}

static SERVER: OnceCell<TestServer> = OnceCell::const_new();

/// 获取共享测试服务器实例（进程内全局唯一，只启动一次）
pub async fn server() -> &'static TestServer {
    SERVER
        .get_or_try_init(|| start_server())
        .await
        .expect("E2E 测试服务器启动失败")
}

/// 启动完整 App：内存 SQLite + 关闭 Nacos + 随机 Web/gRPC 端口
///
/// **关键设计**：App 的 `async_run`（含 Web 服务器的 `axum::serve`）由
/// `tokio::spawn` 启动，绑定在调用方 runtime 上。若在每个 `#[tokio::test]`
/// 各自的 runtime 中启动，测试结束 runtime 销毁会连带杀掉后台 server，
/// 导致后续用例连接被拒（ConnectionRefused）。
/// 因此必须用**独立常驻 runtime** 执行 `ins_run`，该 runtime 与任何测试
/// 的生命周期无关，跨测试持续存活。实现：后台线程 + 泄漏的 `Runtime`。
async fn start_server() -> anyhow::Result<TestServer> {
    let web_port = pick_free_port().await?;
    let grpc_port = pick_free_port().await?;
    // AdminPlugin 通过 GRPC_PORT 环境变量覆盖 gRPC 端口（优先级最高）
    // safety: edition 2024 下 set_var 为 unsafe；测试进程内单次设置，无并发读。
    unsafe {
        std::env::set_var("GRPC_PORT", grpc_port.to_string());
    }

    let mut cfg: toml::Value = toml::from_str(TEST_CONFIG).expect("测试配置解析失败");
    if let Some(web) = cfg.get_mut("web_config").and_then(|v| v.as_table_mut()) {
        web.insert("port".to_string(), toml::Value::Integer(web_port as i64));
    }

    // 常驻 runtime：泄漏的 Runtime 保证其内部 async_run 任务（Web server）跨测试存活。
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("创建常驻 runtime 失败");
    let runtime: &'static tokio::runtime::Runtime = Box::leak(Box::new(runtime));

    let app = tokio::task::spawn_blocking(move || {
        runtime.block_on(async move {
            tx_di_core::BuildContext::with_config(cfg)?
                .build()?
                .ins_run()
                .await
        })
    })
    .await??;

    let base_url = format!("http://127.0.0.1:{web_port}");
    let grpc_url = format!("http://127.0.0.1:{grpc_port}");
    wait_for_ready(&base_url).await?;
    Ok(TestServer {
        base_url,
        grpc_url,
        app,
    })
}

/// 测试配置模板（port 由运行时注入）
const TEST_CONFIG: &str = r#"
[log]
level = "info"
console_output = true
dir = "./logs"
prefix = "tx_admin_e2e"
retention_days = 1
time_format = "local"

[sa_token]
token_name = "Authorization"
timeout = 86400
is_concurrent = true
# is_share=true 时同一账号(admin)的所有登录共享同一个 token，
# 并行运行下 logout 用例会连带失效 admin_token() 的共享 token，
# 导致 user_info_with_token_ok 偶发 401。测试中关闭共享，
# 使各用例独立 token 互不干扰（与设计文档示例 is_share:false 一致）。
is_share = false
token_style = "simple-uuid"
is_read_header = true
is_read_cookie = false
enable_refresh_token = true

[toasty]
database_url = "sqlite://memory"
auto_schema = true
migrate_on_start = false
max_pool_size = 1

[file]
backend = "local"
base_path = "./uploads"
base_url = "http://127.0.0.1:8080/files"
max_file_size = 10485760

[job]
enabled = false

[registry_config]
enabled = false

[grpc_config]
port = 50052

[web_config]
enable_cors = true
host = "127.0.0.1"
max_body_size = 104857600
timeout_secs = 30
layers = [[5, "timeout"], [10, "api_log"], [100, "compression"], [10000, "cors"]]
"#;

/// 选择一个空闲 TCP 端口（绑定后立即释放，随后由服务器绑定）
async fn pick_free_port() -> anyhow::Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    drop(listener);
    Ok(addr.port())
}

/// 轮询健康接口直到服务就绪（默认 15s 超时）
async fn wait_for_ready(base_url: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        if client
            .get(format!("{base_url}/health/live"))
            .send()
            .await
            .is_ok()
        {
            return Ok(());
        }
        if std::time::Instant::now() > deadline {
            anyhow::bail!("服务在 15s 内未就绪: {base_url}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

/// 登录并返回 token（seed 账号 admin/admin123）
pub async fn login(client: &reqwest::Client, base_url: &str) -> anyhow::Result<String> {
    let resp = client
        .post(format!("{base_url}/api/v1/auth/login"))
        .json(&json!({
            "username": "admin",
            "password": "admin123",
            "loginIp": "127.0.0.1",
        }))
        .send()
        .await?;
    let body: Value = resp.json().await?;
    assert_eq!(body["code"], 200, "登录失败: {body}");
    let token = body["data"]["token"]
        .as_str()
        .expect("登录响应缺少 token")
        .to_string();
    Ok(token)
}

static ADMIN_TOKEN: OnceCell<String> = OnceCell::const_new();

/// 登录限流桶容量只有 5，`OnceCell::get_or_try_init` 在多个测试并行进入时会
/// 并发执行多次登录请求（初始化不保证只跑一次），瞬间耗尽配额触发 429。
/// 因此用互斥锁串行化初始化，保证进程内只真正发起一次登录。
static ADMIN_TOKEN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 获取共享 admin token（进程内只真正登录一次，其余等待复用）。
pub async fn admin_token() -> &'static str {
    // 先快速路径：已初始化直接返回，避免锁竞争
    if let Some(token) = ADMIN_TOKEN.get() {
        return token.as_str();
    }
    let _guard = ADMIN_TOKEN_LOCK.lock().await;
    // double-check：锁内可能已被其他任务初始化
    ADMIN_TOKEN
        .get_or_try_init(|| async {
            let srv = server().await;
            let client = reqwest::Client::new();
            login(&client, &srv.base_url).await
        })
        .await
        .expect("获取 admin token 失败")
}

/// 创建携带认证头的客户端（sa-token 默认从 `Authorization` 头读取裸 token）
pub fn authed_client(token: &str) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(token).expect("非法 token"),
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("创建 HTTP 客户端失败")
}
