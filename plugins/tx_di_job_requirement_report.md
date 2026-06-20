# tx_di_job 定时任务插件 - 需求分析报告

## 一、项目概述

### 1.1 插件名称
`tx_di_job` - 基于 `tx_di` 框架的定时任务调度插件

### 1.2 核心功能
- 支持 Cron 表达式的定时任务调度
- 多种任务执行器：内部函数、Shell 脚本、Python 脚本
- 任务执行日志记录和查询
- 任务重试机制和超时监控
- 基于 Toasty ORM 的数据库持久化

### 1.3 依赖插件
- `tx_di_toasty` - 数据库操作（必须）
- `tx_di_log` - 日志记录（推荐）

---

## 二、数据库设计

### 2.1 现有表结构分析

#### 表1: `infrust_job` - 定时任务表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGINT | PK | 任务编号 |
| name | VARCHAR(30) | NOT NULL | 任务名称 |
| status | INTEGER | NOT NULL | 任务状态（0=暂停，1=运行中） |
| handler_name | VARCHAR(200) | NOT NULL | 处理器名称或 URL |
| handler_param | VARCHAR(900) | - | 处理器参数（JSON） |
| cron_expression | VARCHAR(30) | NOT NULL | Cron 表达式 |
| retry_count | INTEGER | DEFAULT 0 | 重试次数 |
| retry_interval | INTEGER | DEFAULT 0 | 重试间隔（秒） |
| monitor_timeout | INTEGER | DEFAULT 0 | 监控超时时间（秒） |
| creator | VARCHAR(100) | - | 创建者 |
| create_time | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updater | VARCHAR(100) | - | 更新者 |
| update_time | TIMESTAMPTZ | NOT NULL | 更新时间 |
| deleted | INTEGER | NOT NULL | 是否删除（软删除） |

#### 表2: `infrust_job_log` - 定时任务日志表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INT8 | PK | 日志编号 |
| job_id | INT8 | NOT NULL | 任务编号（外键） |
| handler_name | VARCHAR(64) | NOT NULL | 处理器名称 |
| handler_param | VARCHAR(30) | DEFAULT '' | 处理器参数 |
| execute_index | INT2 | DEFAULT 1 | 第几次执行 |
| begin_time | TIMESTAMP | NOT NULL | 开始执行时间 |
| end_time | TIMESTAMP | - | 结束执行时间 |
| duration | INT4 | - | 执行时长（毫秒） |
| status | INT2 | NOT NULL | 执行状态（0=失败，1=成功） |
| result | VARCHAR(4000) | DEFAULT '' | 结果数据 |
| creator | VARCHAR(64) | DEFAULT '' | 创建者 |
| create_time | TIMESTAMP | NOT NULL | 创建时间 |
| updater | VARCHAR(64) | DEFAULT '' | 更新者 |
| update_time | TIMESTAMP | NOT NULL | 更新时间 |
| deleted | INT2 | DEFAULT 0 | 是否删除 |

### 2.2 Toasty 模型设计

#### 2.2.1 枚举定义

```rust
/// 任务状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JobStatus {
    Paused = 0,   // 暂停
    Running = 1,   // 运行中
}

/// 执行状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Failed = 0,    // 失败
    Success = 1,    // 成功
    Timeout = 2,    // 超时
    Retrying = 3,   // 重试中
}
```

#### 2.2.2 Toasty 模型定义

**infrust_job 模型**：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrustJob {
    #[toasty(primary_key)]
    pub id: i64,
    pub name: String,        // VARCHAR(30)
    pub status: i32,        // INTEGER (使用枚举: JobStatus)
    pub handler_name: String, // VARCHAR(200)
    pub handler_param: Option<String>, // VARCHAR(900)
    pub cron_expression: String, // VARCHAR(30)
    pub retry_count: i32,   // INTEGER DEFAULT 0
    pub retry_interval: i32, // INTEGER DEFAULT 0
    pub monitor_timeout: i32, // INTEGER DEFAULT 0
    pub creator: Option<String>, // VARCHAR(100)
    pub create_time: DateTime<Utc>, // TIMESTAMPTZ
    pub updater: Option<String>, // VARCHAR(100)
    pub update_time: DateTime<Utc>, // TIMESTAMPTZ
    pub deleted: i32,       // INTEGER DEFAULT 0
}
```

**infrust_job_log 模型**：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrustJobLog {
    #[toasty(primary_key)]
    pub id: i64,
    pub job_id: i64,       // 外键关联 infrust_job.id
    pub handler_name: String, // VARCHAR(64)
    pub handler_param: Option<String>, // VARCHAR(30)
    pub execute_index: i16, // INT2 DEFAULT 1
    pub begin_time: DateTime<Utc>, // TIMESTAMP
    pub end_time: Option<DateTime<Utc>>, // TIMESTAMP
    pub duration: Option<i32>, // INT4 (毫秒)
    pub status: i32,        // INT2 (使用枚举: ExecutionStatus)
    pub result: Option<String>, // VARCHAR(4000)
    pub creator: Option<String>, // VARCHAR(64)
    pub create_time: DateTime<Utc>, // TIMESTAMP
    pub updater: Option<String>, // VARCHAR(64)
    pub update_time: DateTime<Utc>, // TIMESTAMP
    pub deleted: i16,       // INT2 DEFAULT 0
}
```

#### 2.2.3 使用 Embed 优化

Toasty 支持 `Embed`，可以将重复的字段组合提取出来：

```rust
/// 审计字段（创建者、创建时间、更新者、更新时间）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFields {
    pub creator: Option<String>,
    pub create_time: DateTime<Utc>,
    pub updater: Option<String>,
    pub update_time: DateTime<Utc>,
}

/// 软删除字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftDelete {
    pub deleted: i32,
}

/// 优化后的 infrust_job 模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrustJob {
    #[toasty(primary_key)]
    pub id: i64,
    pub name: String,
    pub status: i32,
    pub handler_name: String,
    pub handler_param: Option<String>,
    pub cron_expression: String,
    pub retry_count: i32,
    pub retry_interval: i32,
    pub monitor_timeout: i32,
    #[toasty(embed)]
    pub audit: AuditFields,
    #[toasty(embed)]
    pub soft_delete: SoftDelete,
}
```

---

## 三、任务执行器设计

### 3.1 执行器接口

定义统一的任务执行器 trait：

```rust
/// 任务执行器 trait
#[async_trait]
pub trait JobExecutor: Send + Sync {
    /// 执行任务
    async fn execute(&self, job: &InfrustJob, param: Option<&str>) -> JobResult;
}

/// 任务执行结果
pub struct JobResult {
    pub status: ExecutionStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}
```

### 3.2 内置执行器

#### 3.2.1 内部函数执行器

执行 Rust 异步函数：

```rust
/// 内部函数执行器
pub struct InternalJobExecutor {
    /// 注册的函数映射表
    handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(Option<&str>) -> JobResult + Send + Sync>>>>,
}

impl InternalJobExecutor {
    /// 注册任务处理器
    pub fn register<F>(&self, name: &str, handler: F)
    where
        F: Fn(Option<&str>) -> JobResult + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().unwrap();
        handlers.insert(name.to_string(), Box::new(handler));
    }
}

impl JobExecutor for InternalJobExecutor {
    async fn execute(&self, job: &InfrustJob, param: Option<&str>) -> JobResult {
        let handlers = self.handlers.read().unwrap();
        match handlers.get(&job.handler_name) {
            Some(handler) => handler(param),
            None => JobResult {
                status: ExecutionStatus::Failed,
                result: None,
                error: Some(format!("未找到处理器: {}", job.handler_name)),
            },
        }
    }
}
```

**使用示例**：

```rust
// 注册任务处理器
let executor = ctx.inject::<JobPlugin>().internal_executor();
executor.register("send_email", |param| {
    // 发送邮件逻辑
    JobResult {
        status: ExecutionStatus::Success,
        result: Some("邮件发送成功".to_string()),
        error: None,
    }
});
```

#### 3.2.2 Shell 脚本执行器

执行 Shell 脚本：

```rust
/// Shell 脚本执行器
pub struct ShellJobExecutor {
    pub timeout: Duration,
}

impl JobExecutor for ShellJobExecutor {
    async fn execute(&self, job: &InfrustJob, param: Option<&str>) -> JobResult {
        let script_path = &job.handler_name; // handler_name 存储脚本路径

        // 构建命令
        let mut cmd = Command::new("bash");
        cmd.arg(script_path);
        if let Some(p) = param {
            cmd.arg(p);
        }

        // 设置超时
        let output = match tokio::time::timeout(self.timeout, cmd.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return JobResult {
                    status: ExecutionStatus::Failed,
                    result: None,
                    error: Some(format!("执行脚本失败: {}", e)),
                };
            }
            Err(_) => {
                return JobResult {
                    status: ExecutionStatus::Timeout,
                    result: None,
                    error: Some("执行超时".to_string()),
                };
            }
        };

        if output.status.success() {
            JobResult {
                status: ExecutionStatus::Success,
                result: Some(String::from_utf8_lossy(&output.stdout).to_string()),
                error: None,
            }
        } else {
            JobResult {
                status: ExecutionStatus::Failed,
                result: None,
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
            }
        }
    }
}
```

#### 3.2.3 Python 脚本执行器

执行 Python 脚本：

```rust
/// Python 脚本执行器
pub struct PythonJobExecutor {
    pub python_path: PathBuf, // Python 解释器路径
    pub timeout: Duration,
}

impl JobExecutor for PythonJobExecutor {
    async fn execute(&self, job: &InfrustJob, param: Option<&str>) -> JobResult {
        let script_path = &job.handler_name; // handler_name 存储脚本路径

        // 构建命令
        let mut cmd = Command::new(&self.python_path);
        cmd.arg(script_path);
        if let Some(p) = param {
            cmd.arg(p);
        }

        // 设置超时（与 Shell 执行器类似）
        // ...
    }
}
```

---

## 四、调度器设计

### 4.1 调度器架构

```
┌─────────────────────────────────────────────────┐
│              JobScheduler（调度器）              │
├─────────────────────────────────────────────────┤
│ - 解析 Cron 表达式                             │
│ - 计算下次执行时间                             │
│ - 管理任务队列                                 │
│ - 触发任务执行                                 │
└─────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────┐
│         JobExecutorPool（执行器池）             │
├─────────────────────────────────────────────────┤
│ - InternalJobExecutor（内部函数）               │
│ - ShellJobExecutor（Shell 脚本）                │
│ - PythonJobExecutor（Python 脚本）              │
└─────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────┐
│           JobRepository（数据访问）              │
├─────────────────────────────────────────────────┤
│ - 查询任务列表                                 │
│ - 创建任务                                     │
│ - 更新任务状态                                 │
│ - 记录执行日志                                 │
└─────────────────────────────────────────────────┘
```

### 4.2 Cron 表达式解析

使用 `cron` crate 解析 Cron 表达式：

```rust
use cron::Schedule;

/// 计算任务的下次执行时间
pub fn next_execution_time(cron_expr: &str) -> RIE<DateTime<Utc>> {
    let schedule = Schedule::from_str(cron_expr)
        .map_err(|e| anyhow::anyhow!("无效的 Cron 表达式: {}", e))?;
    
    match schedule.upcoming(Utc).next() {
        Some(next_time) => Ok(next_time),
        None => Err(anyhow::anyhow!("无法计算下次执行时间")),
    }
}
```

### 4.3 调度循环

```rust
/// 调度器主循环
async fn scheduler_loop(
    ctx: Arc<App>,
    token: CancellationToken,
) -> RIE<()> {
    let plugin = ctx.inject::<JobPlugin>();
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                // 1. 查询待执行的任务
                let now = Utc::now();
                let jobs = plugin.get_due_jobs(now).await?;
                
                // 2. 执行任务
                for job in jobs {
                    plugin.execute_job(job).await?;
                }
            }
            _ = token.cancelled() => {
                info!("调度器收到关闭信号，正在停止...");
                break;
            }
        }
    }
    
    Ok(())
}
```

---

## 五、插件接口设计

### 5.1 配置结构体

```rust
/// Job 插件配置
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
pub struct JobConfig {
    /// 是否启用调度器
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// 调度器轮询间隔（秒）
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    
    /// Shell 脚本执行超时时间（秒）
    #[serde(default = "default_shell_timeout")]
    pub shell_timeout_secs: u64,
    
    /// Python 脚本执行超时时间（秒）
    #[serde(default = "default_python_timeout")]
    pub python_timeout_secs: u64,
    
    /// Python 解释器路径
    #[serde(default = "default_python_path")]
    pub python_path: PathBuf,
    
    /// 任务执行线程池大小
    #[serde(default = "default_thread_pool_size")]
    pub thread_pool_size: usize,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            poll_interval_secs: default_poll_interval(),
            shell_timeout_secs: default_shell_timeout(),
            python_timeout_secs: default_python_timeout(),
            python_path: default_python_path(),
            thread_pool_size: default_thread_pool_size(),
        }
    }
}
```

### 5.2 组件结构体

```rust
/// Job 插件组件
#[derive(Clone, Debug)]
#[tx_comp(init)]
pub struct JobPlugin {
    /// 配置
    pub config: Arc<JobConfig>,
    
    /// 数据库访问
    pub db: Arc<ToastyPlugin>,
    
    /// 内部函数执行器
    pub internal_executor: Arc<InternalJobExecutor>,
    
    /// 调度器句柄
    #[tx_cst(OnceLock::new())]
    pub scheduler_handle: OnceLock<JoinHandle<()>>,
}

impl JobPlugin {
    /// 注册内部任务处理器
    pub fn register_handler<F>(&self, name: &str, handler: F)
    where
        F: Fn(Option<&str>) -> JobResult + Send + Sync + 'static,
    {
        self.internal_executor.register(name, handler);
    }
    
    /// 手动触发任务执行
    pub async fn trigger_job(&self, job_id: i64) -> RIE<()> {
        let job = self.get_job_by_id(job_id).await?;
        self.execute_job(job).await
    }
    
    /// 查询待执行的任务
    async fn get_due_jobs(&self, now: DateTime<Utc>) -> RIE<Vec<InfrustJob>> {
        // 使用 Toasty 查询数据库
        // ...
    }
    
    /// 执行任务
    async fn execute_job(&self, job: InfrustJob) -> RIE<()> {
        // 1. 创建执行日志
        // 2. 根据 handler_name 选择执行器
        // 3. 执行任务
        // 4. 记录结果
        // 5. 处理重试
        // ...
    }
}
```

### 5.3 公共 API

```rust
impl JobPlugin {
    /// 创建任务
    pub async fn create_job(&self, req: CreateJobRequest) -> RIE<InfrustJob> {
        // ...
    }
    
    /// 更新任务
    pub async fn update_job(&self, job_id: i64, req: UpdateJobRequest) -> RIE<InfrustJob> {
        // ...
    }
    
    /// 删除任务（软删除）
    pub async fn delete_job(&self, job_id: i64) -> RIE<()> {
        // ...
    }
    
    /// 暂停任务
    pub async fn pause_job(&self, job_id: i64) -> RIE<()> {
        // ...
    }
    
    /// 恢复任务
    pub async fn resume_job(&self, job_id: i64) -> RIE<()> {
        // ...
    }
    
    /// 查询任务列表
    pub async fn list_jobs(&self, page: i64, page_size: i64) -> RIE<Vec<InfrustJob>> {
        // ...
    }
    
    /// 查询任务执行日志
    pub async fn get_job_logs(&self, job_id: i64, page: i64, page_size: i64) -> RIE<Vec<InfrustJobLog>> {
        // ...
    }
}
```

---

## 六、初始化流程

### 6.1 模型注册

在 `main.rs` 中，在 `build()` 之前注册模型：

```rust
use tx_di_toasty::ToastyPlugin;
use tx_di_job::models;

// 注册 Job 插件模型
ToastyPlugin::register_models(toasty::models!(
    InfrustJob,
    InfrustJobLog
));

let app = BuildContext::new(Some("config.toml")).build()?.ins_run().await?;
```

### 6.2 插件初始化

```rust
impl CompInit for JobPlugin {
    /// 构建时初始化
    fn inner_init(&mut self, _: &InnerContext) -> RIE<()> {
        info!("JobPlugin: 初始化开始");
        
        // 验证配置
        // 检查数据库表是否存在（Toasty auto_schema 会自动创建）
        
        Ok(())
    }
    
    /// 异步初始化
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            info!("JobPlugin: 异步初始化开始");
            
            // 从数据库加载所有运行中的任务
            let plugin = ctx.inject::<JobPlugin>();
            plugin.load_jobs_from_db().await?;
            
            info!("JobPlugin: 异步初始化完成");
            Ok(())
        }
    );
    
    /// 启动调度器
    async_method!(
        fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
            info!("JobPlugin: 启动调度器");
            
            let plugin = ctx.inject::<JobPlugin>();
            
            // 启动调度器主循环
            let handle = tokio::spawn(scheduler_loop(ctx.clone(), token));
            
            // 保存句柄
            plugin.scheduler_handle.set(handle).map_err(|_| anyhow::anyhow!("调度器已启动"))?;
            
            // 等待关闭信号
            token.cancelled().await;
            info!("JobPlugin: 收到关闭信号，正在优雅关闭...");
            
            Ok(())
        }
    );
    
    fn init_sort() -> i32 {
        i32::MAX - 10 // 在 Web 插件之前启动
    }
}
```

---

## 七、配置文件示例

### 7.1 configs/di-config.toml

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

---

## 八、使用场景示例

### 8.1 场景1：执行内部函数

```rust
// main.rs
let app = BuildContext::new(Some("config.toml")).build()?.ins_run().await?;

// 注册任务处理器
let job_plugin = app.inject::<JobPlugin>();
job_plugin.register_handler("cleanup_logs", |param| {
    info!("清理日志开始");
    // 清理逻辑
    JobResult {
        status: ExecutionStatus::Success,
        result: Some("清理了 100 条日志".to_string()),
        error: None,
    }
});

// 通过 API 创建任务
// POST /api/jobs
// {
//     "name": "清理日志",
//     "handler_name": "cleanup_logs",
//     "cron_expression": "0 0 2 * * ?",  // 每天凌晨2点
//     "retry_count": 3,
//     "retry_interval": 60
// }
```

### 8.2 场景2：执行 Shell 脚本

```rust
// 创建任务时，handler_name 设置为脚本路径
// POST /api/jobs
// {
//     "name": "备份数据库",
//     "handler_name": "/opt/scripts/backup.sh",
//     "handler_param": "--compress",
//     "cron_expression": "0 0 3 * * ?",  // 每天凌晨3点
//     "monitor_timeout": 3600  // 1小时超时
// }
```

### 8.3 场景3：执行 Python 脚本

```rust
// 创建任务时，handler_name 设置为脚本路径
// POST /api/jobs
// {
//     "name": "数据分析",
//     "handler_name": "/opt/scripts/analyze.py",
//     "handler_param": "{\"date\": \"2024-01-01\"}",
//     "cron_expression": "0 0 4 * * ?",  // 每天凌晨4点
//     "retry_count": 2,
//     "retry_interval": 300
// }
```

---

## 九、错误处理

### 9.1 错误类型定义

```rust
/// Job 插件错误
#[derive(Debug, thiserror::Error)]
pub enum JobErr {
    #[error("任务不存在: {0}")]
    JobNotFound(i64),
    
    #[error("无效的 Cron 表达式: {0}")]
    InvalidCronExpression(String),
    
    #[error("任务执行失败: {0}")]
    ExecutionFailed(String),
    
    #[error("任务执行超时")]
    ExecutionTimeout,
    
    #[error("未找到处理器: {0}")]
    HandlerNotFound(String),
    
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] ToastyErr),
}
```

### 9.2 重试机制

```rust
/// 执行任务（带重试）
async fn execute_job_with_retry(
    &self,
    job: &InfrustJob,
    executor: &dyn JobExecutor,
) -> JobResult {
    let mut retry_count = 0;
    let max_retries = job.retry_count as u32;
    let retry_interval = Duration::from_secs(job.retry_interval as u64);
    
    loop {
        let result = executor.execute(job, job.handler_param.as_deref()).await;
        
        if result.status == ExecutionStatus::Success || retry_count >= max_retries {
            return result;
        }
        
        warn!(
            job_id = job.id,
            retry_count = retry_count + 1,
            "任务执行失败，准备重试"
        );
        
        retry_count += 1;
        tokio::time::sleep(retry_interval).await;
    }
}
```

---

## 十、监控和日志

### 10.1 执行日志

每次任务执行都会在 `infrust_job_log` 表中创建一条记录：

```rust
/// 记录任务执行日志
async fn log_execution(
    &self,
    job_id: i64,
    handler_name: &str,
    handler_param: Option<&str>,
    execute_index: i16,
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    duration: i32,
    status: ExecutionStatus,
    result: Option<&str>,
) -> RIE<()> {
    let log = InfrustJobLog {
        id: self.generate_id().await?,
        job_id,
        handler_name: handler_name.to_string(),
        handler_param: handler_param.map(|s| s.to_string()),
        execute_index,
        begin_time,
        end_time: Some(end_time),
        duration: Some(duration),
        status: status as i32,
        result: result.map(|s| s.to_string()),
        creator: Some("system".to_string()),
        create_time: Utc::now(),
        updater: Some("system".to_string()),
        update_time: Utc::now(),
        deleted: 0,
    };
    
    self.db().insert(log).await?;
    Ok(())
}
```

### 10.2 超时监控

```rust
/// 执行任务（带超时监控）
async fn execute_with_timeout(
    &self,
    executor: &dyn JobExecutor,
    job: &InfrustJob,
    timeout_secs: i32,
) -> JobResult {
    if timeout_secs <= 0 {
        return executor.execute(job, job.handler_param.as_deref()).await;
    }
    
    let timeout = Duration::from_secs(timeout_secs as u64);
    
    match tokio::time::timeout(timeout, executor.execute(job, job.handler_param.as_deref())).await {
        Ok(result) => result,
        Err(_) => JobResult {
            status: ExecutionStatus::Timeout,
            result: None,
            error: Some("任务执行超时".to_string()),
        },
    }
}
```

---

## 十一、总结

### 11.1 功能清单

- [x] Cron 表达式调度
- [x] 内部函数执行器
- [x] Shell 脚本执行器
- [x] Python 脚本执行器
- [x] 任务重试机制
- [x] 超时监控
- [x] 执行日志记录
- [x] 任务管理 API（创建、更新、删除、暂停、恢复）
- [x] 枚举支持
- [x] Embed 支持

### 11.2 技术栈

- **调度器**：`cron` crate（Cron 表达式解析）
- **数据库**：`toasty` ORM（通过 `tx_di_toasty` 插件）
- **异步运行时**：`tokio`
- **进程管理**：`tokio::process::Command`（执行 Shell/Python 脚本）

### 11.3 下一步

请确认以上需求分析报告是否符合您的预期。确认后，我将立即生成完整的插件代码，包括：

1. `Cargo.toml`
2. `src/lib.rs`
3. `src/config.rs`
4. `src/comp.rs`
5. `src/models.rs`（数据库模型）
6. `src/executors/` （执行器实现）
7. `src/repository.rs`（数据访问层）
8. `README.md`

---

**请确认报告内容，如有修改建议请告知。**
