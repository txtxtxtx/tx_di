//! anyhow::Error → AppError 转换

use crate::AppError;

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        // 尝试向下转型为 AppError
        if let Some(app_err) = err.downcast_ref::<AppError>() {
            return app_err.clone();
        }
        // 直接构造 ErrCode 变体
        AppError::WithContext {
            domain: "SYS",
            code: 90000,
            message: "Internal error",
            context: err.to_string(),
        }
    }
}
