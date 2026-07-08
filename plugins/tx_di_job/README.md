# tx_di_job — 定时任务调度插件使用文档

基于 `tx-di` 的**定时任务调度插件**，通过轮询数据库中的任务行实现 Cron 调度。

## 用途

- 轮询式调度器：每隔 `poll_interval_secs` 秒扫描"运行中"任务，按 **5 字段 Cron**（精确到分钟）判断是否命中时间槽。
- 三类执行器：**内部 Rust 函数**、**Shell 脚本**、**Python 脚本**。
- 重试机制、执行超时监控、执行日志落库、任务 CRUD / 暂停 / 恢复 / 手动触发。

> 强依赖 `tx_di_toasty`（任务与日志存于数据库），需配置 `[toasty_config]` 且数据库可达。

## 启用

`Cargo.toml`：

```toml
tx_di_job    = { path = "plugins/tx_di_job" }
tx_di_toasty = { path = "plugins/tx_di_toasty", features = ["sqlite"] }
```

`JobPlugin` 与 `JobConfig` 自带 `#[component]` 标注，依赖本 crate 即自动注册，**无需手动 register**。

## 配置

TOML 节名为 `[job_config]`：

```toml
[job_config]
enabled = true
poll_interval_secs = 1
shell_timeout_secs = 300
python_timeout_secs = 300
python_path = "/usr/bin/python3"
thread_pool_size = 4
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | `bool` | `true` | 调度器主循环开关 |
| `poll_interval_secs` | `u64` | `1` | 轮询间隔秒，≤0 时重置为 1 |
| `shell_timeout_secs` | `u64` | `300` | Shell 执行超时 |
| `python_timeout_secs` | `u64` | `300` | Python 执行超时 |
| `python_path` | `PathBuf` | `/usr/bin/python3` | Python 解释器路径 |
| `thread_pool_size` | `usize` | `4` | 预留字段（当前未实际使用） |

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `JobConfig` | `conf`, `init`, `init_sort = i32::MIN + 1` | 配置载体 |
| `JobPlugin` | `app_async_init`, `app_async_run`, `init_sort = i32::MAX - 10` | 调度器门面 |

`JobPlugin` 方法：`register_handler(name, handler)` / `trigger_job(id)` / `create_job(...)` / `update_job` / `delete_job`（软删）/ `pause_job` / `resume_job` / `list_jobs(page)` / `get_job_logs(id, page)`。

## 使用方式

```rust
use tx_di_core::{BuildContext, Component};
use tx_di_job::{JobPlugin, JobResult, ExecutionStatus};

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    let app = BuildContext::new::<std::path::PathBuf>(Some("configs/di-config.toml"))
        .build()?.ins_run().await?;

    let job_plugin = app.inject::<JobPlugin>();

    // 注册内部函数处理器（闭包签名 Fn(Option<&str>) -> JobResult）
    job_plugin.register_handler("cleanup_logs", |param: Option<&str>| {
        tracing::info!("清理日志, param={:?}", param);
        JobResult { status: ExecutionStatus::Success, result: Some("ok".into()), error: None }
    });

    // 创建任务（每天 02:00，5 字段 Cron）
    let job = job_plugin.create_job("清理日志", "cleanup_logs", "0 2 * * *").await?;

    job_plugin.pause_job(job.id).await?;
    job_plugin.trigger_job(job.id).await?;
    Ok(())
}
```

- Shell/Python 任务：无需 `register_handler`，`handler_name` 传 `.sh`/`.py` 绝对路径即可（由 `ExecutorType::from_handler_name` 识别）。
- **模型注册（重要）**：`ToastyPlugin::register_models(...)` 是**实例方法**，需在 `build()` 之后、`run()` 之前取得 `ToastyPlugin` 实例再调用：
  ```rust
  let app = ctx.build()?;
  app.inject::<tx_di_toasty::ToastyPlugin>()
     .register_models(tx_di_job::models::register_models());
  let app = app.ins_run().await?;
  ```

## 注意事项

1. **Cron 为 5 字段（分 时 日 月 周），精确到分钟**，不支持秒级，不识别 `?`。`"0 0 2 * * ?"`（6 字段）会被 `cron_matches` 判为不匹配、**永不触发**，应使用 `"0 2 * * *"`。
2. **同一分钟只触发一次**（去重）；错过的时间窗口不会补执行（无 misfire 策略）。
3. 调度器单任务顺序执行，长任务会阻塞后续判断。
4. `thread_pool_size` 当前未使用。
5. 重试执行 1 + retry_count 次，每次独立写一条 `InfrustJobLog`。
6. 数据库强依赖（`auto_schema` 自动建表 `infrust_job`/`infrust_job_log`）。
