//! 认证领域错误码
//!
//! | 错误码 | 说明 |
//! |--------|------|
//! | AUTH-2001 | 用户名或密码错误 |
//! | AUTH-2002 | 用户已被禁用 |
//! | AUTH-2003 | 用户不存在 |

use tx_error::CodeMsg;

/// 认证领域错误码
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("AUTH")]
pub enum AuthError {
    /// 用户名或密码错误（模糊提示，防枚举）
    #[err(2001, "用户名或密码错误")]
    InvalidCredentials,

    /// 用户已被禁用
    #[err(2002, "用户已被禁用")]
    UserDisabled,

    /// 用户不存在
    #[err(2003, "用户名或密码错误")]
    UserNotFound,
}
