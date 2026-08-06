//! 配置路径解析工具
//!
//! 生产部署时，进程的工作目录（CWD）往往与服务化/容器化启动方式不一致，
//! 依赖相对路径的配置（数据库文件、上传目录、日志目录、静态资源目录）
//! 会静默失效。本模块提供统一的相对路径解析规则：
//!
//! 1. 绝对路径 → 原样返回
//! 2. 设置环境变量 `APP_HOME` → 返回 `APP_HOME/<path>`
//! 3. 未设置 → 原样返回（保持与 CWD 相对的既有开发行为，向后兼容）

use std::path::Path;

/// 将配置中的相对路径解析为实际路径，规则见模块文档。
pub fn resolve_data_path(path: &str) -> String {
    let p = Path::new(path);
    if p.is_absolute() {
        return path.to_string();
    }
    if let Ok(home) = std::env::var("APP_HOME") {
        let home = home.trim();
        if !home.is_empty() {
            return Path::new(home).join(path).to_string_lossy().into_owned();
        }
    }
    path.to_string()
}

/// 解析 SQLite 连接串（`sqlite:path` 或 `sqlite://path`）中的相对数据库路径。
///
/// - `sqlite://memory` / `sqlite:memory` 等内存库不解析
/// - 绝对路径不解析
/// - 其余相对路径按 [`resolve_data_path`] 规则锚定
pub fn resolve_sqlite_url(url: &str) -> String {
    let (scheme, rest) = if let Some(rest) = url.strip_prefix("sqlite://") {
        ("sqlite://", rest)
    } else if let Some(rest) = url.strip_prefix("sqlite:") {
        ("sqlite:", rest)
    } else {
        // 非 SQLite URL（postgres/mysql/dynamodb 等）不处理
        return url.to_string();
    };

    if rest.is_empty() || rest == "memory" || Path::new(rest).is_absolute() {
        return url.to_string();
    }
    format!("{}{}", scheme, resolve_data_path(rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn absolute_path_unchanged() {
        assert_eq!(resolve_data_path("/data/app.db"), "/data/app.db");
        assert_eq!(resolve_data_path(r"C:\data\app.db"), r"C:\data\app.db");
    }

    #[test]
    fn sqlite_memory_unchanged() {
        assert_eq!(resolve_sqlite_url("sqlite://memory"), "sqlite://memory");
        assert_eq!(resolve_sqlite_url("sqlite:memory"), "sqlite:memory");
    }

    #[test]
    fn sqlite_absolute_unchanged() {
        assert_eq!(
            resolve_sqlite_url("sqlite:///opt/data/app.db"),
            "sqlite:///opt/data/app.db"
        );
    }

    #[test]
    fn app_home_prepends_relative() {
        // 模拟 APP_HOME 环境变量
        unsafe {
            std::env::set_var("APP_HOME", "/app");
        }
        // 平台无关比较：直接用 PathBuf 语义验证（Windows 下分隔符为 \）
        let expected = Path::new("/app").join("data/app.db");
        assert_eq!(PathBuf::from(resolve_data_path("data/app.db")), expected);

        // SQLite URL：前缀保留原 scheme，尾部为解析后的路径
        let url = resolve_sqlite_url("sqlite:data/app.db");
        assert!(url.starts_with("sqlite:"));
        assert!(url.ends_with("data/app.db"));

        let url2 = resolve_sqlite_url("sqlite://data/app.db");
        assert!(url2.starts_with("sqlite://"));
        assert!(url2.ends_with("data/app.db"));

        unsafe {
            std::env::remove_var("APP_HOME");
        }
    }

    #[test]
    fn no_app_home_keeps_relative() {
        unsafe {
            std::env::remove_var("APP_HOME");
        }
        assert_eq!(resolve_data_path("data/app.db"), "data/app.db");
        assert_eq!(resolve_sqlite_url("sqlite:data/app.db"), "sqlite:data/app.db");
    }
}
