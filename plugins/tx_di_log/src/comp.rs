use crate::LogConfig;
use std::sync::{Arc, OnceLock};
use std::{fs, panic};
use tracing::{debug,error};
use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};
use tx_di_core::{Component, DepsTuple, RIE, Store};

// 全局变量存储 日志 guard
static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

/// 检查日志系统是否已经初始化过
fn is_log_initialized() -> bool {
    LOG_GUARD.get().is_some()
}

#[derive(Component)]
#[component(init, init_sort = i32::MIN)]
pub struct LogPlugins{
    /// 日志配置
    pub config: Arc<LogConfig>,
}

/// #[component(init)] 回调函数：在 build 之后、inner_init 阶段执行
fn init(this: &mut LogPlugins, _store: &Store) -> RIE<()> {
    // 如果全局守卫已经初始化（例如并行测试场景），跳过重复设置
    if is_log_initialized() {
        debug!("日志系统已初始化，跳过重复初始化");
        return Ok(());
    }

    if !this.config.dir.exists() {
        fs::create_dir_all(&this.config.dir)?;
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
        .filename_prefix(&this.config.prefix)
        .filename_suffix("log")
        .max_log_files(this.config.retention_days)
        .build(&this.config.dir)
        .map_err(|e| anyhow::Error::new(e))?;

    let (non_blocking_appender, guard) = NonBlocking::new(file_appender);

    if LOG_GUARD.set(guard).is_err() {
        // 竞态条件：另一个线程在我们检查后完成了初始化，直接返回
        debug!("日志全局守卫已被其他线程设置，跳过");
        return Ok(());
    }

    let timer = this.config.time_format.to_timer(&this.config.time_format_str)?;
    let file_layer = fmt::layer()
        .with_writer( non_blocking_appender)
        .with_ansi(false)
        .with_thread_ids(true)
        .with_thread_names( true)
        .with_level(true)
        .with_file( true)
        .with_line_number( true)
        .with_target( false)
        .with_timer(timer.clone())
        .compact();

    // 控制台输出层 - 美化格式，便于开发查看
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true) // 控制台启用颜色
        .with_timer(timer)
        .with_level(true)
        .with_target(false) // 显示模块路径
        .with_file(true) // 显示文件名
        .with_line_number(true) // 显示行号
        .compact(); // pretty json compact
    // 环境过滤器
    // let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&self.config.level.as_str().to_lowercase()));
    // 构建环境过滤器，支持模块级别的日志覆盖
    let env_filter = if this.config.modules.is_empty() {
        // 如果没有模块级别的配置，使用全局级别
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&this.config.level.as_str().to_lowercase()))
    } else {
        // 从全局级别开始
        let mut filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&this.config.level.as_str().to_lowercase()));

        // 添加模块级别的覆盖配置
        for (module, level) in &this.config.modules {
            let directive_str = format!("{}={}", module, level.as_str().to_lowercase());
            match directive_str.parse() {
                Ok(directive) => {
                    filter = filter.add_directive(directive);
                }
                Err(e) => {
                    error!("无效的日志指令 '{}': {}，已跳过该模块配置", directive_str, e);
                }
            }
        }

        filter
    };
    // 注册全局订阅者（使用 try_init 避免重复初始化 panic）
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer);
    if this.config.console_output {
        let _ = subscriber
            .with(console_layer)
            .try_init();
    }else {
        let _ = subscriber.try_init();
    }

    // 设置 panic hook：仅记录日志，不强制退出进程。
    // 强制退出会破坏测试环境（catch_unwind 无法捕获）与嵌入式场景的正常流程。
    panic::set_hook(Box::new(|panic_info| {
        error!("程序异常终止: {}", panic_info);
    }));
    debug!("日志初始化完成");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use tx_di_core::BuildContext;

    #[test]
    fn test_log_plugins_build() {
        // LogPlugins 可以正常构建，inner_init 设置 tracing
        let ctx = BuildContext::new::<&str>(None);
        let plugin = ctx.inject::<LogPlugins>();

        // 验证配置已注入
        assert_eq!(plugin.config.level, log::LevelFilter::Info);
        assert!(plugin.config.console_output, "console_output 默认应为 true");

        // 日志子系统已初始化，写入测试日志
        tracing::info!("[test] 日志插件测试消息");
        tracing::warn!("[test] 这是一条警告消息");
        tracing::error!("[test] 这是一条错误消息");

        // 验证日志文件已生成
        assert!(
            plugin.config.dir.exists(),
            "日志目录应已创建: {:?}",
            plugin.config.dir
        );

        // 到达此处说明构建和初始化未 panic
    }

    #[test]
    fn test_log_init_sort() {
        assert_eq!(<LogPlugins as Component>::init_sort(), i32::MIN);
    }
}
