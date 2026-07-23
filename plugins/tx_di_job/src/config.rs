use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tx_di_core::{Component, RIE, Store};

/// Job 插件配置
#[derive(Debug, Clone, Serialize, Deserialize, Component)]
#[component(conf, init, init_sort = i32::MIN + 1)]
pub struct JobConfig {
    /// 是否启用调度器 默认 启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// 调度器轮询间隔（秒） 默认 1s
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    
    /// Shell 脚本执行超时时间（秒） 300s 5 分钟
    #[serde(default = "default_shell_timeout")]
    pub shell_timeout_secs: u64,
    
    /// Python 脚本执行超时时间（秒）
    #[serde(default = "default_python_timeout")]
    pub python_timeout_secs: u64,
    
    /// Python 解释器路径 /usr/bin/python3
    #[serde(default = "default_python_path")]
    pub python_path: PathBuf,
    
    /// 任务执行线程池大小 4
    #[serde(default = "default_thread_pool_size")]
    pub thread_pool_size: usize,

    /// 内部函数执行器超时（秒） 300s 5 分钟
    /// 若内部处理器阻塞超过此时间，将强制中止并标记超时
    #[serde(default = "default_internal_timeout")]
    pub internal_timeout_secs: u64,
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
            internal_timeout_secs: default_internal_timeout(),
        }
    }
}

/// `#[component(init)]` 回调：验证配置
fn init(this: &mut JobConfig, _store: &Store) -> RIE<()> {
    // 验证配置
    if this.poll_interval_secs <= 0 {
        tracing::warn!("poll_interval_secs 不能为 0，已重置为默认值 1");
        this.poll_interval_secs = default_poll_interval();
    }
    Ok(())
}

// 默认值函数
fn default_enabled() -> bool {
    true
}

fn default_poll_interval() -> u64 {
    1
}

fn default_shell_timeout() -> u64 {
    300 // 5 分钟
}

fn default_python_timeout() -> u64 {
    300 // 5 分钟
}

fn default_python_path() -> PathBuf {
    PathBuf::from("/usr/bin/python3")
}

fn default_thread_pool_size() -> usize {
    4
}

fn default_internal_timeout() -> u64 {
    300 // 5 分钟
}

impl JobConfig {
    /// 获取 Shell 执行超时
    pub fn shell_timeout(&self) -> Duration {
        Duration::from_secs(self.shell_timeout_secs)
    }
    
    /// 获取 Python 执行超时
    pub fn python_timeout(&self) -> Duration {
        Duration::from_secs(self.python_timeout_secs)
    }
    
    /// 获取轮询间隔
    pub fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.poll_interval_secs)
    }

    /// 获取内部函数执行超时
    pub fn internal_timeout(&self) -> Duration {
        Duration::from_secs(self.internal_timeout_secs)
    }
}
