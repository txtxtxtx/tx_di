use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fmt};
use time::format_description;
use time::format_description::OwnedFormatItem;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime, LocalTime, UtcTime};
use tx_di_core::Component;

/// 日志配置结构体
///
/// 通过 TOML 配置文件自动反序列化（配置键：`log_config`）。
///
/// # 示例
///
/// ```toml
/// [log_config]
/// level = "info"
/// ```
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf = "log")]
pub struct LogConfig {
    /// 全局日志级别过滤器
    #[serde(default = "default_level")]
    pub level: log::LevelFilter,

    /// 模块级别的日志覆盖配置
    #[serde(default)]
    pub modules: HashMap<String, log::LevelFilter>,

    /// 日志输出格式
    #[serde(default = "default_format")]
    pub format: String,

    /// 日志文件输出目录
    #[serde(default = "default_dir")]
    pub dir: PathBuf,

    /// 日志保留天数
    #[serde(default = "default_retention_days")]
    pub retention_days: usize,

    /// 是否输出到控制台，默认为 true（开发时直观，生产环境可配置关闭）
    #[serde(default = "default_console_output")]
    pub console_output: bool,

    /// 日志文件前缀
    #[serde(default = "default_prefix")]
    pub prefix: String,

    /// 时间格式类型
    #[serde(default)]
    pub time_format: TimeFormat,

    #[serde(default = "time_format_str")]
    pub time_format_str: String,
}

fn time_format_str() -> String {
    "[hour]:[minute]:[second].[subsecond digits:3]".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
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
    Utc(UtcTime<OwnedFormatItem>),
    Local(LocalTime<OwnedFormatItem>),
}

impl TimeFormat {
    pub fn to_timer(&self, format: &str) -> Result<TimerWrapper, anyhow::Error> {
        let format = format_description::parse_owned::<2>(format)
            .map_err(|e| anyhow::anyhow!(e))?;
        match self {
            TimeFormat::Utc => Ok(TimerWrapper::Utc(UtcTime::new(format))),
            TimeFormat::Local => Ok(TimerWrapper::Local(LocalTime::new(format))),
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

fn default_level() -> log::LevelFilter {
    log::LevelFilter::Info
}

fn default_format() -> String {
    String::new()
}

fn default_dir() -> PathBuf {
    if let Ok(exe_path) = env::current_exe()
        && let Some(parent) = exe_path.parent()
    {
        return parent.join("logs");
    }
    PathBuf::from("./logs")
}

fn default_retention_days() -> usize {
    90
}

fn default_prefix() -> String {
    "tx_di".to_string()
}

fn default_console_output() -> bool {
    true
}
