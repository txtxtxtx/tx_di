use std::{fs, panic};
use std::sync::{Arc, OnceLock};
use anyhow::{anyhow, Context};
use log::{debug, error};
use tracing::info;
use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tx_di_core::{tx_comp, BuildContext, CompInit, RIE};
use crate::LogConfig;

// 全局变量存储 日志 guard
static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();
#[derive(Clone,Debug)]
#[tx_comp(init)]
pub struct LogPlugins{
    /// 日志配置
    pub config: Arc<LogConfig>,
}

impl CompInit for LogPlugins{
    fn inner_init(&mut self, _: &mut BuildContext) -> RIE<()>{
        if !self.config.dir.exists() {
            fs::create_dir_all(&self.config.dir)?;
        }
        // 选项        作用
        // with_thread_ids(true) 显示线程ID
        // with_thread_names(true) 显示线程名称
        // with_target(true) 显示日志位置（模块路径）
        // with_file(true) 显示文件名
        // with_line_number(true) 显示行号
        // with_level(false) 不显示日志级别
        // with_timer(...) 设置时间格式
        // with_ansi(false) // 文件中禁用ANSI颜色
        // 按天滚动的文件输出
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix(&self.config.prefix)
            .filename_suffix(".log")
            .max_log_files(self.config.retention_days)
            .build(&self.config.dir)
            .map_err(|e| anyhow::Error::new(e))?;

        let (non_blocking_appender, guard) = NonBlocking::new(file_appender);

        LOG_GUARD.set(guard)
            .map_err(|_| anyhow!("日志全局守卫已被初始化，不允许重复设置"))?;

        let timer = self.config.time_format.to_timer();
        let file_layer = fmt::layer()
            .with_writer( non_blocking_appender)
            .with_ansi(false)
            .with_thread_ids(true)
            .with_thread_names( true)
            .with_level(true)
            .with_file( true)
            .with_line_number( true)
            .with_target( true)
            .with_timer(timer.clone())
            .compact();

        // 控制台输出层 - 美化格式，便于开发查看
        let console_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true) // 控制台启用颜色
            .with_timer(timer)
            .with_level(true)
            .with_target(true) // 显示模块路径
            .with_file(true) // 显示文件名
            .with_line_number(true) // 显示行号
            .pretty();
        // 环境过滤器
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&self.config.level.as_str().to_lowercase()));

        // 注册全局订阅者
        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer);
        if self.config.console_output {
            subscriber
                .with(console_layer)
                .init();
        }else {
            subscriber.init();
        }

        // 设置 panic hook
        panic::set_hook(Box::new(|panic_info| {
            error!("程序异常终止: {}", panic_info);
        }));
        info!("日志初始化完成");
        Ok(())
    }

    /// 插件初始化排序,
    fn init_sort() -> i32 {
        i32::MIN
    }
}