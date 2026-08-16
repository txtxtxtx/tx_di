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
///
/// # 注释与字符串
///
/// 按 TOML 词法区分注释与字符串上下文：
/// - **注释中的 `${...}` 不展开**（如文档示例 `# ${VAR}`），注释文本被丢弃
///   （保留换行，使后续 TOML 解析的行号不偏移）
/// - 字符串值中的 `#` 不会被当作注释起始
/// - 字符串（基础 `"..."`、字面 `'...'`、多行 `"""..."""` / `'''...'''`）中的
///   `${...}` 会正常展开
pub fn interpolate_env(input: &str) -> Result<String, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let n = chars.len();

    #[derive(Clone, Copy, PartialEq)]
    enum Ctx {
        /// 普通上下文（表头 / 键 / 空白）
        Normal,
        /// 基础字符串 `"..."`
        Basic,
        /// 字面字符串 `'...'`
        Literal,
        /// 多行基础字符串 `"""..."""`
        MultilineBasic,
        /// 多行字面字符串 `'''...'''`
        MultilineLiteral,
        /// 注释（`#` 到行尾）
        Comment,
    }

    let mut ctx = Ctx::Normal;

    while i < n {
        let c = chars[i];

        // 占位符展开：仅非注释上下文（字符串值 / 表头 / 裸值）
        if ctx != Ctx::Comment && c == '$' && chars.get(i + 1) == Some(&'{') {
            let body_start = i + 2;
            let Some(rel_end) = chars[body_start..].iter().position(|&x| x == '}') else {
                return Err("配置中存在未闭合的占位符 `${`（缺少 `}`）".to_string());
            };
            let end = body_start + rel_end;
            let inner: String = chars[body_start..end].iter().collect();

            // 支持 `${VAR:-default}` 语法
            let (name, default) = match inner.split_once(":-") {
                Some((n, d)) => (n.trim(), Some(d.to_string())),
                None => (inner.trim(), None),
            };
            if name.is_empty() {
                return Err("配置占位符 `${}` 缺少环境变量名".to_string());
            }

            let value = match std::env::var(name) {
                Ok(v) => v,
                Err(_) => match default {
                    Some(d) => d,
                    None => {
                        return Err(format!(
                            "配置占位符 `${{{inner}}}` 引用的环境变量 `{name}` 未定义"
                        ))
                    }
                },
            };
            out.push_str(&value);
            i = end + 1;
            continue;
        }

        match ctx {
            Ctx::Normal => {
                if c == '#' {
                    ctx = Ctx::Comment;
                } else if c == '"' {
                    if chars.get(i + 1) == Some(&'"') && chars.get(i + 2) == Some(&'"') {
                        out.push_str("\"\"\"");
                        ctx = Ctx::MultilineBasic;
                        i += 3;
                        continue;
                    }
                    out.push('"');
                    ctx = Ctx::Basic;
                } else if c == '\'' {
                    if chars.get(i + 1) == Some(&'\'') && chars.get(i + 2) == Some(&'\'') {
                        out.push_str("'''");
                        ctx = Ctx::MultilineLiteral;
                        i += 3;
                        continue;
                    }
                    out.push('\'');
                    ctx = Ctx::Literal;
                } else {
                    out.push(c);
                }
                i += 1;
            }
            Ctx::Basic => {
                if c == '\\' {
                    // 转义：连同下一个字符原样保留
                    out.push(c);
                    if let Some(&next) = chars.get(i + 1) {
                        out.push(next);
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else if c == '"' {
                    out.push(c);
                    ctx = Ctx::Normal;
                    i += 1;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            Ctx::Literal => {
                if c == '\'' {
                    out.push(c);
                    ctx = Ctx::Normal;
                } else {
                    out.push(c);
                }
                i += 1;
            }
            Ctx::MultilineBasic => {
                if c == '"'
                    && chars.get(i + 1) == Some(&'"')
                    && chars.get(i + 2) == Some(&'"')
                {
                    out.push_str("\"\"\"");
                    ctx = Ctx::Normal;
                    i += 3;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            Ctx::MultilineLiteral => {
                if c == '\''
                    && chars.get(i + 1) == Some(&'\'')
                    && chars.get(i + 2) == Some(&'\'')
                {
                    out.push_str("'''");
                    ctx = Ctx::Normal;
                    i += 3;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            Ctx::Comment => {
                // 丢弃注释内容，仅保留换行（维持后续 TOML 解析的行号）
                if c == '\n' {
                    out.push('\n');
                    ctx = Ctx::Normal;
                }
                i += 1;
            }
        }
    }

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

    #[test]
    fn comment_placeholder_ignored() {
        // 注释中的 `${VAR}` 是文档示例，不应展开、不应报错
        let s = "# 语法示例：${VAR} 与 ${VAR:-default}\n[web_config]\nport = 8888\n";
        let out = interpolate_env(s).unwrap();
        assert!(out.contains("port = 8888"), "保留有效配置: {out}");
        assert!(!out.contains("${VAR}"), "注释内容应被丢弃: {out}");
    }

    #[test]
    fn inline_comment_placeholder_ignored() {
        // 行内注释（# 后）中的占位符不展开，且不会因未闭合而报错
        const VAR: &str = "TX_DI_TEST_INLINE";
        unsafe { std::env::set_var(VAR, "v1") };
        let s = "x = \"${TX_DI_TEST_INLINE}\" # 示例 ${UNCLOSED\n";
        let out = interpolate_env(s).unwrap();
        assert!(out.contains("x = \"v1\""), "值应正常展开: {out}");
        assert!(!out.contains("UNCLOSED"), "注释内容应被丢弃: {out}");
        assert!(!out.contains("${"), "注释占位符不应残留: {out}");
        unsafe { std::env::remove_var(VAR) };
    }

    #[test]
    fn hash_inside_string_not_comment() {
        // 字符串值里的 `#` 不是注释起始，后面的内容原样保留
        let s = "greeting = \"hello#world\"\n";
        let out = interpolate_env(s).unwrap();
        assert_eq!(out, s);
    }

    #[test]
    fn multiline_string_hash_kept() {
        // 多行字符串中的 `#` 与 `${}` 语义
        const VAR: &str = "TX_DI_TEST_ML";
        unsafe { std::env::set_var(VAR, "ml-value") };
        let s = "desc = \"\"\"line1 # not-comment\n${TX_DI_TEST_ML}\"\"\"\n";
        let out = interpolate_env(s).unwrap();
        assert!(out.contains("line1 # not-comment"), "多行字符串中的 # 保留: {out}");
        assert!(out.contains("ml-value"), "多行字符串中的占位符展开: {out}");
        unsafe { std::env::remove_var(VAR) };
    }
}
