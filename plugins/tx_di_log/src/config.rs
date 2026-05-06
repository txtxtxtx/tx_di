use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fmt};
use time::format_description::well_known;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime, LocalTime, UtcTime};
use tx_di_core::{CompInit, tx_comp};

/// 日志配置结构体
///
/// 用于配置应用程序的日志级别，通过 TOML 配置文件自动反序列化。
///
/// # 配置文件示例
///
/// ```toml
/// [log_config]
/// level = "info"  # 可选值: "off", "error", "warn", "info", "debug", "trace"（不区分大小写）
/// ```
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
pub struct LogConfig {
    /// 全局日志级别过滤器
    ///
    /// 控制日志输出的详细程度。支持的值为：
    /// - `"off"`: 完全禁用日志
    /// - `"error"`: 仅记录错误
    /// - `"warn"`: 记录警告和错误
    /// - `"info"`: 记录信息、警告和错误（默认值）
    /// - `"debug"`: 记录调试信息及更高级别
    /// - `"trace"`: 记录所有日志，包括最详细的跟踪信息
    ///
    /// 该字段在 TOML 配置文件中对应 `log_config.level`，
    /// 字符串值不区分大小写。
    #[serde(default = "default_level")]
    pub level: log::LevelFilter,

    /// 模块级别的日志覆盖配置
    /// Key: 模块路径 (如 "my_crate::module")
    /// Value: 日志级别 (如 "debug", "trace")
    #[serde(default)]
    pub modules: HashMap<String, log::LevelFilter>,

    /// 日志输出格式
    ///
    /// 默认为 `"{date} {time} [{level}] {target}: {message}"`。
    ///
    /// 该字段在 TOML 配置文件中对应 `log_config.format`。
    #[serde(default = "default_format")]
    pub format: String,

    /// 日志输出位置
    ///
    /// 默认为 `"stdout"`。
    ///
    /// 该字段在 TOML 配置文件中对应 `log_config.target`。
    #[serde(default = "default_dir")]
    pub dir: PathBuf,

    /// 日志保留天数，默认为 90 2400 天 6年多
    #[serde(default = "default_retention_days")]
    pub retention_days: usize,

    /// 是否输出到控制台，默认为 false
    #[serde(default)]
    pub console_output: bool,

    /// 日志文件前缀
    #[serde(default = "default_prefix")]
    pub prefix: String,

    /// 时间格式类型
    ///
    /// 支持两种模式：
    /// - `"utc"`: 使用 UTC 时间（协调世界时），格式如 `2026-04-22T06:30:45.123456789Z`
    /// - `"local"`: 使用本地时间（系统时区），格式如 `2026-04-22T14:30:45.123456789+08:00`
    ///
    /// 默认为 `"utc"`。
    ///
    /// 该字段在 TOML 配置文件中对应 `log_config.time_format`。
    #[serde(default)]
    pub time_format: TimeFormat,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_level(),
            modules: HashMap::new(),
            format: default_format(),
            dir: default_dir(),
            retention_days: default_retention_days(),
            console_output: false,
            prefix: default_prefix(),
            time_format: TimeFormat::default(),
        }
    }
}

impl CompInit for LogConfig {
    fn init_sort() -> i32 {
        i32::MIN
    }
}

#[derive(Debug, Clone, Deserialize,Default)]
#[serde(rename_all = "lowercase")]
pub enum TimeFormat {
    /// UTC 时间（协调世界时）
    Utc,
    /// 本地时间（系统时区）
    #[default]
    Local,
}

/// 定时器包装器，统一 UTC 和本地时间
#[derive(Debug, Clone)]
pub enum TimerWrapper {
    Utc(UtcTime<well_known::Rfc3339>),
    Local(LocalTime<well_known::Rfc3339>),
}

impl TimeFormat {
    /// 将 TimeFormat 转换为对应的计时器实例
    pub fn to_timer(&self) -> TimerWrapper {
        match self {
            TimeFormat::Utc => TimerWrapper::Utc(UtcTime::rfc_3339()),
            TimeFormat::Local => TimerWrapper::Local(LocalTime::rfc_3339()),
        }
    }
}

impl FormatTime for TimerWrapper {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        match self {
            TimerWrapper::Utc(timer) => timer.format_time(w),
            TimerWrapper::Local(timer) => timer.format_time(w),
        }
    }
}
impl fmt::Display for TimeFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeFormat::Utc => write!(f, "utc"),
            TimeFormat::Local => write!(f, "local"),
        }
    }
}

/// 提供默认的日志级别
fn default_level() -> log::LevelFilter {
    log::LevelFilter::Info
}

/// 提供默认的日志格式
fn default_format() -> String {
    "".to_string()
}

/// 默认的日志目录
fn default_dir() -> PathBuf {
    // 获取可执行文件所在目录
    if let Ok(exe_path) = env::current_exe()
        && let Some(parent) = exe_path.parent()
    {
        return parent.join("logs");
    }

    // 降级方案：使用当前工作目录
    PathBuf::from("./logs")
}

fn default_retention_days() -> usize {
    90
}

fn default_prefix() -> String {
    "tx_di".to_string()
}
