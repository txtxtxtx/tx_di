use serde::Serialize;

/// 接口返回结构体
#[derive(Debug, Serialize)]
pub struct ApiR<T> {
    // 业务状态码，通常 200 或是 0 表示成功
    pub code: i32,
    // 给客户端的提示信息
    pub msg: String,
    // 返回的数据
    pub data: Option<T>,
}



impl<T> ApiR<T> {
    /// 成功响应（带数据）
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            data: Some(data),
            msg: "success".to_string(),
        }
    }

    /// 错误响应（带数据）
    pub fn error_with_data(code: i32, msg: String, data: T) -> Self {
        Self {
            code,
            data: Some(data),
            msg,
        }
    }
}

/// 接口返回结构体，无数据
pub type ApiRes = ApiR<()>;

impl ApiRes {
    /// 成功响应（无数据）
    pub fn ok() -> Self {
        Self {
            code: 200,
            data: None,
            msg: "success".to_string(),
        }
    }

    /// 错误响应
    pub fn error(code: i32, msg: String) -> Self {
        Self {
            code,
            data: None,
            msg,
        }
    }

    /// 失败响应
    pub fn fail(msg: String) -> Self {
        Self {
            code: -1,
            data: None,
            msg,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RCode {
    // 成功
    Success = 200,
    // 错误
    Error = -1,
    // 未找到
    NotFound = 404,
    // 未授权
    Unauthorized = 401,
    // 禁止
    Forbidden = 403,
    // 参数错误
    BadRequest = 400,
    // 服务器错误
    ServerError = 500,
    // 业务错误
    CustomError = 10000,
}
impl RCode {
    /// 获取错误信息
    pub fn msg(&self) -> &'static str {
        match self {
            RCode::Success => "成功",
            RCode::Error => "错误",
            RCode::NotFound => "未找到",
            RCode::Unauthorized => "未授权",
            RCode::Forbidden => "禁止",
            RCode::BadRequest => "参数错误",
            RCode::ServerError => "服务器错误",
            RCode::CustomError => "业务错误",
        }
    }
    /// 获取错误码
    pub fn code(&self) -> i32 {
        *self as i32
    }
    /// 转换为ApiR
    pub fn to_api_r<T>(&self, data: T) -> ApiR<T> {
        ApiR::error_with_data(self.code(),
                              self.msg().to_string(),
                              data)
    }
}
/// 实现从 ErrorCode 到 ApiR<()> 的转换
impl From<RCode> for ApiRes {
    fn from(error_code: RCode) -> Self {
        ApiRes::error(
            error_code.code(),
            error_code.msg().to_string()
        )
    }
}


// use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
// 将ApiR类型的响应转换为HTTP响应
//
// 该实现将序列化的ApiR对象转换为JSON格式的HTTP响应，
// 并设置状态码为200 OK
//
// # 类型参数
// * `T` - 实现Serialize trait的类型，用于序列化响应数据
//
// # 参数
// * `self` - ApiR<T>实例，包含要转换的响应数据
//
// # 返回值
// * `Response` - HTTP响应对象，包含JSON格式的响应体和200状态码
// impl <T: Serialize> IntoResponse for ApiR<T> {
//     fn into_response(self) -> Response {
//         // 序列化ApiR对象为JSON格式的响应体
//         (StatusCode::OK, Json(self)).into_response()
//     }
// }