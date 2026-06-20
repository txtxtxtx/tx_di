# tx_di_job

基于 `tx_di` 框架的定时任务调度插件，支持 Cron 表达式、内部函数、Shell 脚本和 Python 脚本执行。

## 功能特性

- **Cron 表达式调度**：支持标准的 Cron 表达式，精确到秒
- **多种任务执行器**：
  - 内部函数执行器：执行 Rust 异步函数
  - Shell 脚本执行器：执行 Shell 脚本
  - Python 脚本执行器：执行 Python 脚本
- **任务重试机制**：支持配置重试次数和重试间隔
- **超时监控**：支持配置任务执行超时时间
- **执行日志记录**：记录每次任务执行的详细信息
- **任务管理 API**：提供创建、更新、删除、暂停、恢复等 API

## 快速开始

### 1. 添加依赖

```toml
# Cargo.toml
tx_di_job = { path = "plugins/tx_di_job" }
```

### 2. 配置

在 `configs/di-config.toml` 中添加配置：

```toml
[toasty_config]
database_url = "postgresql://user:pass@localhost/tx_di"
auto_schema = true

[job_config]
enabled = true
poll_interval_secs = 1
shell_timeout_secs = 300
python_timeout_secs = 300
python_path = "/usr/bin/python3"
thread_pool_size = 4
```

### 3. 注册模型

在 `main.rs` 中，在 `build()` 之前注册模型：

```rust,ignore
use tx_di_toasty::ToastyPlugin;
use tx_di_job::models;

// 注册 Job 插件模型
ToastyPlugin::register_models(models::register_models());

let app = BuildContext::new(Some("config.toml")).build()?.ins_run().await?;
```

### 4. 注册任务处理器

```rust,ignore
let app = BuildContext::new(Some("config.toml")).build()?.ins_run().await?;

// 注册内部任务处理器
let job_plugin = app.inject::<JobPlugin>();
job_plugin.register_handler("cleanup_logs", |param| {
    tracing::info!("清理日志开始");
    // 清理逻辑
    JobResult {
        status: ExecutionStatus::Success,
        result: Some("清理了 100 条日志".to_string()),
        error: None,
    }
});
```

### 5. 创建任务

通过 API 创建任务：

```json
POST /api/jobs
{
    "name": "清理日志",
    "handler_name": "cleanup_logs",
    "cron_expression": "0 0 2 * * ?",
    "retry_count": 3,
    "retry_interval": 60
}
```

## 配置说明

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | `bool` | `true` | 是否启用调度器 |
| `poll_interval_secs` | `u64` | `1` | 调度器轮询间隔（秒） |
| `shell_timeout_secs` | `u64` | `300` | Shell 脚本执行超时时间（秒） |
| `python_timeout_secs` | `u64` | `300` | Python 脚本执行超时时间（秒） |
| `python_path` | `PathBuf` | `/usr/bin/python3` | Python 解释器路径 |
| `thread_pool_size` | `usize` | `4` | 任务执行线程池大小 |

## 使用场景示例

### 场景 1：执行内部函数

```rust,ignore
// 注册任务处理器
job_plugin.register_handler("send_email", |param| {
    // 发送邮件逻辑
    JobResult {
        status: ExecutionStatus::Success,
        result: Some("邮件发送成功".to_string()),
        error: None,
    }
});

// 创建任务
// POST /api/jobs
// {
//     "name": "发送邮件",
//     "handler_name": "send_email",
//     "cron_expression": "0 0 9 * * ?",  // 每天上午9点
//     "retry_count": 3
// }
```

### 场景 2：执行 Shell 脚本

```json
POST /api/jobs
{
    "name": "备份数据库",
    "handler_name": "/opt/scripts/backup.sh",
    "handler_param": "--compress",
    "cron_expression": "0 0 3 * * ?",  // 每天凌晨3点
    "monitor_timeout": 3600  // 1小时超时
}
```

### 场景 3：执行 Python 脚本

```json
POST /api/jobs
{
    "name": "数据分析",
    "handler_name": "/opt/scripts/analyze.py",
    "handler_param": "{\"date\": \"2024-01-01\"}",
    "cron_expression": "0 0 4 * * ?",  // 每天凌晨4点
    "retry_count": 2,
    "retry_interval": 300
}
```

## API 文档

### JobPlugin 公共方法

```rust,ignore
impl JobPlugin {
    /// 注册内部任务处理器
    pub fn register_handler<F>(&self, name: &str, handler: F);
    
    /// 注销内部任务处理器
    pub fn unregister_handler(&self, name: &str);
    
    /// 手动触发任务执行
    pub async fn trigger_job(&self, job_id: i64) -> RIE<()>;
    
    /// 创建任务
    pub async fn create_job(&self, name: &str, handler_name: &str, cron_expression: &str) -> RIE<InfrustJob>;
    
    /// 更新任务
    pub async fn update_job(&self, job_id: i64, name: Option<&str>, handler_name: Option<&str>, cron_expression: Option<&str>) -> RIE<InfrustJob>;
    
    /// 删除任务（软删除）
    pub async fn delete_job(&self, job_id: i64) -> RIE<()>;
    
    /// 暂停任务
    pub async fn pause_job(&self, job_id: i64) -> RIE<()>;
    
    /// 恢复任务
    pub async fn resume_job(&self, job_id: i64) -> RIE<()>;
    
    /// 查询任务列表
    pub async fn list_jobs(&self) -> RIE<Vec<InfrustJob>>;
    
    /// 查询任务执行日志
    pub async fn get_job_logs(&self, job_id: i64, limit: i64) -> RIE<Vec<InfrustJobLog>>;
}
```

## 数据库表结构

### infrust_job - 定时任务表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | `BIGINT` | PK | 任务编号 |
| `name` | `VARCHAR(30)` | NOT NULL | 任务名称 |
| `status` | `INTEGER` | NOT NULL | 任务状态（0=暂停，1=运行中） |
| `handler_name` | `VARCHAR(200)` | NOT NULL | 处理器名称或路径 |
| `handler_param` | `VARCHAR(900)` | - | 处理器参数（JSON） |
| `cron_expression` | `VARCHAR(30)` | NOT NULL | Cron 表达式 |
| `retry_count` | `INTEGER` | DEFAULT 0 | 重试次数 |
| `retry_interval` | `INTEGER` | DEFAULT 0 | 重试间隔（秒） |
| `monitor_timeout` | `INTEGER` | DEFAULT 0 | 监控超时时间（秒） |
| `creator` | `VARCHAR(100)` | - | 创建者 |
| `create_time` | `TIMESTAMPTZ` | NOT NULL | 创建时间 |
| `updater` | `VARCHAR(100)` | - | 更新者 |
| `update_time` | `TIMESTAMPTZ` | NOT NULL | 更新时间 |
| `deleted` | `INTEGER` | NOT NULL | 是否删除（软删除） |

### infrust_job_log - 定时任务日志表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | `INT8` | PK | 日志编号 |
| `job_id` | `INT8` | NOT NULL | 任务编号（外键） |
| `handler_name` | `VARCHAR(64)` | NOT NULL | 处理器名称 |
| `handler_param` | `VARCHAR(30)` | DEFAULT '' | 处理器参数 |
| `execute_index` | `INT2` | DEFAULT 1 | 第几次执行 |
| `begin_time` | `TIMESTAMP` | NOT NULL | 开始执行时间 |
| `end_time` | `TIMESTAMP` | - | 结束执行时间 |
| `duration` | `INT4` | - | 执行时长（毫秒） |
| `status` | `INT2` | NOT NULL | 执行状态（0=失败，1=成功，2=超时，3=重试中） |
| `result` | `VARCHAR(4000)` | DEFAULT '' | 结果数据 |
| `creator` | `VARCHAR(64)` | DEFAULT '' | 创建者 |
| `create_time` | `TIMESTAMP` | NOT NULL | 创建时间 |
| `updater` | `VARCHAR(64)` | DEFAULT '' | 更新者 |
| `update_time` | `TIMESTAMP` | NOT NULL | 更新时间 |
| `deleted` | `INT2` | DEFAULT 0 | 是否删除 |

## 枚举类型

### JobStatus - 任务状态

```rust
pub enum JobStatus {
    Paused = 0,   // 暂停
    Running = 1,   // 运行中
}
```

### ExecutionStatus - 执行状态

```rust
pub enum ExecutionStatus {
    Failed = 0,    // 失败
    Success = 1,    // 成功
    Timeout = 2,    // 超时
    Retrying = 3,   // 重试中
}
```

## 错误处理

插件定义了 `JobErr` 错误类型，包括：

- `JobNotFound(i64)` - 任务不存在
- `InvalidCronExpression(String)` - 无效的 Cron 表达式
- `ExecutionFailed(String)` - 任务执行失败
- `ExecutionTimeout` - 任务执行超时
- `HandlerNotFound(String)` - 未找到处理器
- `DatabaseError(toasty::Error)` - 数据库错误
- `JsonError(serde_json::Error)` - JSON 序列化错误
- `IoError(std::io::Error)` - IO 错误
- `Other(anyhow::Error)` - 其他错误

## 许可证

[MIT License](LICENSE) 或 [Apache-2.0 License](LICENSE)

## 贡献

欢迎提交 Issue 和 Pull Request！

## 更新历史

- 2026-06-20: 初始版本，支持基本定时任务调度功能
