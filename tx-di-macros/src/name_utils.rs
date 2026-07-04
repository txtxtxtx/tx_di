//! 命名转换工具
//!
//! 提供驼峰命名与蛇形/大写蛇形命名之间的转换，
//! 用于生成静态变量名和配置 key。

/// 将驼峰命名法字符串转换为蛇形命名法。
///
/// 在大写字母前插入下划线，并将所有字符转换为小写。
/// 第一个字符前不插入下划线。
///
/// # 示例
///
/// `DbPool` -> `db_pool`
pub fn camel_to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        // 在非首字符的大写字母前插入下划线
        if ch.is_uppercase() && i != 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

/// 将驼峰命名法字符串转换为大写蛇形命名法（SCREAMING_SNAKE_CASE）。
///
/// 先转换为蛇形命名法，再将所有字符转为大写。
/// 常用于生成常量名或静态变量名。
pub fn camel_to_screaming_snake(s: &str) -> String {
    camel_to_snake(s).to_uppercase()
}
