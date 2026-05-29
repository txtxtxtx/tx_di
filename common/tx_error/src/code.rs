use std::fmt;

/// 归一化错误码值类型。
///
/// 纯值类型，无堆分配，无虚表，可 `Copy`/`Clone`。
/// 错误身份由 `domain + code` 唯一决定，`message` 仅用于展示。
#[derive(Debug, Copy, Clone)]
pub struct AppErrCode {
    pub domain: &'static str,
    pub code: u16,
    pub message: &'static str,
}

impl AppErrCode {
    #[inline]
    pub const fn new(domain: &'static str, code: u16, message: &'static str) -> Self {
        Self { domain, code, message }
    }
}

/// 错误身份相等：domain + code 唯一决定错误身份，message 不参与比较
impl PartialEq for AppErrCode {
    fn eq(&self, other: &Self) -> bool {
        self.domain == other.domain && self.code == other.code
    }
}

// 为 AppErrCode 类型实现 Eq trait
impl Eq for AppErrCode {}

impl fmt::Display for AppErrCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}] {}", self.domain, self.code, self.message)
    }
}

/// 错误码转换 trait — 连接业务错误枚举与统一 `AppError` 的桥梁。
///
/// 通过宏自动生成实现，业务层无需手写。
pub trait CodeMsg: fmt::Debug + fmt::Display + Copy + Sync + Send + 'static {
    /// 将自身转换为归一化的 `AppErrCode` 值
    fn err_code(self) -> AppErrCode;

    #[inline]
    fn domain(self) -> &'static str {
        self.err_code().domain
    }

    #[inline]
    fn code(self) -> u16 {
        self.err_code().code
    }

    #[inline]
    fn message(self) -> &'static str {
        self.err_code().message
    }
}
