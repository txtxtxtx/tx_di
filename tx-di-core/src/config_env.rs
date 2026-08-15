//! 配置文件环境变量插值
//!
//! 支持在 TOML 配置文件中使用 `${VAR}` / `${VAR:-default}` 占位符，
//! 在 TOML 解析前替换为环境变量值（`.env` 由 [`ensure_dotenv`] 加载进进程环境）。
//!
//! ```toml
//! [registry_config]
//! password = "${NACOS_PASSWORD}"          # 未定义时报错
//! host = "${SERVICE_HOST:-127.0.0.1}"     # 未定义时用默认值
//! ```

use std::sync::OnceLock;

/// 幂等加载 `.env` 文件（仅首次调用生效）
///
/// 加载路径优先级：
/// 1. 环境变量 `DOTENV_PATH` 指定的文件路径
/// 2. 当前工作目录下的 `.env`
///
/// `.env` 不存在或读取失败时静默忽略（纯环境变量部署是合法场景）。
/// 已存在于进程环境中的变量不会被 `.env` 覆盖。
pub fn ensure_dotenv() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        // `dotenv()` 返回 Result<PathBuf, _>，`from_path` 返回 Result<(), _>，
        // 统一为 Result<(), _> 后忽略结果（.env 不存在是正常情况）。
        let result: Result<(), dotenvy::Error> = match std::env::var("DOTENV_PATH") {
            Ok(p) if !p.trim().is_empty() => dotenvy::from_path(p.trim()),
            _ => dotenvy::dotenv().map(|_| ()),
        };
        if let Ok(()) = result {
            eprintln!("[di] 已加载环境变量文件（dotenv）");
        }
    });
}

/// 将配置文本中的 `${VAR}` / `${VAR:-default}` 替换为环境变量值
///
/// - `${VAR}`：取环境变量 `VAR`，未定义则返回错误
/// - `${VAR:-default}`：取环境变量 `VAR`，未定义则用 `default`
///
/// 其它 `$`（非 `${` 前缀，如 `$5`）原样保留。
pub fn interpolate_env(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(idx) = rest.find("${") {
        let (head, tail) = rest.split_at(idx);
        out.push_str(head);

        // tail 以 "${" 开头，找匹配的 "}"
        let body = &tail[2..];
        let Some(end) = body.find('}') else {
            return Err("配置中存在未闭合的占位符 `${`（缺少 `}`）".to_string());
        };
        let inner = &body[..end];

        // 支持 `${VAR:-default}` 语法
        let (name, default) = match inner.split_once(":-") {
            Some((n, d)) => (n.trim(), Some(d)),
            None => (inner.trim(), None),
        };
        if name.is_empty() {
            return Err("配置占位符 `${}` 缺少环境变量名".to_string());
        }

        let value = match std::env::var(name) {
            Ok(v) => v,
            Err(_) => match default {
                Some(d) => d.to_string(),
                None => {
                    return Err(format!(
                        "配置占位符 `${{{inner}}}` 引用的环境变量 `{name}` 未定义"
                    ))
                }
            },
        };
        out.push_str(&value);
        rest = &body[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_placeholder_unchanged() {
        let s = "hello = \"world\"\nprice = \"$5\"";
        assert_eq!(interpolate_env(s).unwrap(), s);
    }

    #[test]
    fn simple_var_replaced() {
        const VAR: &str = "TX_DI_TEST_SIMPLE";
        unsafe { std::env::set_var(VAR, "secret-123") };
        let out = interpolate_env("password = \"${TX_DI_TEST_SIMPLE}\"").unwrap();
        assert_eq!(out, "password = \"secret-123\"");
        unsafe { std::env::remove_var(VAR) };
    }

    #[test]
    fn default_used_when_missing() {
        const VAR: &str = "TX_DI_TEST_MISSING";
        unsafe { std::env::remove_var(VAR) };
        let out = interpolate_env("host = \"${TX_DI_TEST_MISSING:-127.0.0.1}\"").unwrap();
        assert_eq!(out, "host = \"127.0.0.1\"");
    }

    #[test]
    fn default_ignored_when_set() {
        const VAR: &str = "TX_DI_TEST_SET";
        unsafe { std::env::set_var(VAR, "10.0.0.1") };
        let out = interpolate_env("host = \"${TX_DI_TEST_SET:-127.0.0.1}\"").unwrap();
        assert_eq!(out, "host = \"10.0.0.1\"");
        unsafe { std::env::remove_var(VAR) };
    }

    #[test]
    fn missing_var_without_default_errors() {
        const VAR: &str = "TX_DI_TEST_UNDEF";
        unsafe { std::env::remove_var(VAR) };
        let err = interpolate_env("x = \"${TX_DI_TEST_UNDEF}\"").unwrap_err();
        assert!(err.contains(VAR), "错误信息应包含变量名: {err}");
    }

    #[test]
    fn unclosed_placeholder_errors() {
        let err = interpolate_env("x = \"${UNCLOSED").unwrap_err();
        assert!(err.contains("未闭合"), "错误信息应提示未闭合: {err}");
    }
}
